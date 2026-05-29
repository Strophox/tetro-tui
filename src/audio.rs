use std::{
    collections::VecDeque,
    io::Write,
    process::Command,
    sync::mpsc,
    thread,
    time::Duration,
};

use crate::{
    core_game_engine::{Notification, NotificationFeed},
    settings::{AudioSettings, SfxPack, ThemeSong},
};

#[derive(Clone, Copy, Debug)]
pub enum SoundEffect {
    Keypress,
    PieceLock,
    LineClear { lines: u32 },
    GameOver,
}

#[derive(Clone, Copy, Debug)]
struct Note {
    frequency_hz: u16,
    duration_ms: u16,
    rest_ms: u16,
}

enum AudioCommand {
    PlaySfx(SoundEffect),
    Stop,
}

pub struct AudioController {
    settings: AudioSettings,
    sender: Option<mpsc::Sender<AudioCommand>>,
}

impl AudioController {
    pub fn new(settings: AudioSettings) -> Self {
        if !settings.enabled {
            return Self {
                settings,
                sender: None,
            };
        }

        let (sender, receiver) = mpsc::channel::<AudioCommand>();
        thread::spawn(move || audio_worker(receiver, settings));
        Self {
            settings,
            sender: Some(sender),
        }
    }

    pub fn play_keypress(&self) {
        if self.settings.enabled && self.settings.sfx_enabled && self.settings.keypress_sfx {
            self.send(AudioCommand::PlaySfx(SoundEffect::Keypress));
        }
    }

    pub fn play_from_notifications(&self, feed: &NotificationFeed) {
        if !(self.settings.enabled && self.settings.sfx_enabled) {
            return;
        }
        for (notification, _) in feed {
            match notification {
                Notification::PieceLocked { .. } if self.settings.piece_lock_sfx => {
                    self.send(AudioCommand::PlaySfx(SoundEffect::PieceLock));
                }
                Notification::Accolade { lineclears, .. } if self.settings.line_clear_sfx => {
                    self.send(AudioCommand::PlaySfx(SoundEffect::LineClear { lines: *lineclears }));
                }
                Notification::GameEnded { is_win: false, .. } if self.settings.game_over_sfx => {
                    self.send(AudioCommand::PlaySfx(SoundEffect::GameOver));
                }
                _ => {}
            }
        }
    }

    fn send(&self, command: AudioCommand) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(command);
        }
    }
}

impl Drop for AudioController {
    fn drop(&mut self) {
        self.send(AudioCommand::Stop);
    }
}

fn audio_worker(receiver: mpsc::Receiver<AudioCommand>, settings: AudioSettings) {
    let mut queued_sfx: VecDeque<&'static [Note]> = VecDeque::new();
    let mut theme_index = 0usize;
    let mut stop = false;

    while !stop {
        match receiver.recv_timeout(Duration::from_millis(5)) {
            Ok(AudioCommand::PlaySfx(effect)) => queued_sfx.push_back(notes_for_sfx(effect, settings)),
            Ok(AudioCommand::Stop) => stop = true,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        while let Ok(command) = receiver.try_recv() {
            match command {
                AudioCommand::PlaySfx(effect) => queued_sfx.push_back(notes_for_sfx(effect, settings)),
                AudioCommand::Stop => {
                    stop = true;
                    break;
                }
            }
        }

        if stop {
            break;
        }

        if let Some(notes) = queued_sfx.pop_front() {
            play_notes(notes, settings.theme_tempo_percent.max(20));
            continue;
        }

        if settings.theme_enabled {
            let theme = theme_notes(settings.theme_song);
            let note = theme[theme_index % theme.len()];
            theme_index += 1;
            play_note(note, settings.theme_tempo_percent.max(20));
        } else {
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn play_notes(notes: &[Note], tempo_percent: u16) {
    for note in notes {
        play_note(*note, tempo_percent);
    }
}

fn play_note(note: Note, tempo_percent: u16) {
    let scaled_duration_ms = (u32::from(note.duration_ms) * 100)
        .checked_div(u32::from(tempo_percent))
        .unwrap_or(u32::from(note.duration_ms))
        .clamp(1, 10_000);
    let scaled_rest_ms = (u32::from(note.rest_ms) * 100)
        .checked_div(u32::from(tempo_percent))
        .unwrap_or(u32::from(note.rest_ms))
        .clamp(0, 10_000);

    if note.frequency_hz == 0 {
        thread::sleep(Duration::from_millis(u64::from(scaled_duration_ms)));
    } else {
        let status = Command::new("beep")
            .arg("-f")
            .arg(note.frequency_hz.to_string())
            .arg("-l")
            .arg(scaled_duration_ms.to_string())
            .status();

        if status.is_err() {
            let _ = std::io::stdout().write_all(b"\x07");
            let _ = std::io::stdout().flush();
            thread::sleep(Duration::from_millis(u64::from(scaled_duration_ms)));
        }
    }

    if scaled_rest_ms > 0 {
        thread::sleep(Duration::from_millis(u64::from(scaled_rest_ms)));
    }
}

fn notes_for_sfx(effect: SoundEffect, settings: AudioSettings) -> &'static [Note] {
    match (settings.sfx_pack, effect) {
        (SfxPack::Classic, SoundEffect::Keypress) => &SFX_KEYPRESS_CLASSIC,
        (SfxPack::Classic, SoundEffect::PieceLock) => &SFX_PIECE_LOCK_CLASSIC,
        (SfxPack::Classic, SoundEffect::LineClear { lines }) if lines >= 4 => {
            &SFX_LINE_CLEAR_TETRIS_CLASSIC
        }
        (SfxPack::Classic, SoundEffect::LineClear { .. }) => &SFX_LINE_CLEAR_CLASSIC,
        (SfxPack::Classic, SoundEffect::GameOver) => &SFX_GAME_OVER_CLASSIC,
        (SfxPack::Arcade, SoundEffect::Keypress) => &SFX_KEYPRESS_ARCADE,
        (SfxPack::Arcade, SoundEffect::PieceLock) => &SFX_PIECE_LOCK_ARCADE,
        (SfxPack::Arcade, SoundEffect::LineClear { lines }) if lines >= 4 => {
            &SFX_LINE_CLEAR_TETRIS_ARCADE
        }
        (SfxPack::Arcade, SoundEffect::LineClear { .. }) => &SFX_LINE_CLEAR_ARCADE,
        (SfxPack::Arcade, SoundEffect::GameOver) => &SFX_GAME_OVER_ARCADE,
    }
}

fn theme_notes(song: ThemeSong) -> &'static [Note] {
    match song {
        ThemeSong::KorobeinikiA => &THEME_KOROBEINIKI_A,
        ThemeSong::KorobeinikiB => &THEME_KOROBEINIKI_B,
    }
}

const THEME_KOROBEINIKI_A: [Note; 32] = [
    n(659, 125, 10),
    n(494, 63, 10),
    n(523, 63, 10),
    n(587, 125, 10),
    n(523, 63, 10),
    n(494, 63, 10),
    n(440, 125, 10),
    n(440, 63, 10),
    n(523, 63, 10),
    n(659, 125, 10),
    n(587, 63, 10),
    n(523, 63, 10),
    n(494, 188, 20),
    n(523, 63, 10),
    n(587, 125, 10),
    n(659, 125, 10),
    n(523, 125, 10),
    n(440, 125, 10),
    n(440, 125, 10),
    n(0, 63, 15),
    n(587, 125, 10),
    n(698, 63, 10),
    n(880, 125, 10),
    n(784, 63, 10),
    n(698, 63, 10),
    n(659, 188, 15),
    n(523, 63, 10),
    n(659, 125, 10),
    n(587, 63, 10),
    n(523, 63, 10),
    n(494, 188, 20),
    n(0, 63, 15),
];

const THEME_KOROBEINIKI_B: [Note; 24] = [
    n(659, 150, 8),
    n(523, 75, 8),
    n(587, 75, 8),
    n(659, 150, 8),
    n(587, 75, 8),
    n(523, 75, 8),
    n(494, 150, 8),
    n(494, 75, 8),
    n(587, 75, 8),
    n(698, 150, 8),
    n(659, 75, 8),
    n(587, 75, 8),
    n(523, 150, 8),
    n(523, 75, 8),
    n(587, 75, 8),
    n(659, 150, 8),
    n(698, 75, 8),
    n(784, 75, 8),
    n(880, 225, 18),
    n(784, 75, 8),
    n(698, 75, 8),
    n(659, 150, 8),
    n(587, 150, 8),
    n(0, 75, 20),
];

const SFX_KEYPRESS_CLASSIC: [Note; 1] = [n(880, 22, 3)];
const SFX_PIECE_LOCK_CLASSIC: [Note; 2] = [n(392, 30, 2), n(330, 38, 4)];
const SFX_LINE_CLEAR_CLASSIC: [Note; 3] = [n(523, 45, 2), n(659, 45, 2), n(784, 65, 8)];
const SFX_LINE_CLEAR_TETRIS_CLASSIC: [Note; 5] = [
    n(523, 40, 2),
    n(659, 40, 2),
    n(784, 40, 2),
    n(988, 55, 2),
    n(1319, 95, 8),
];
const SFX_GAME_OVER_CLASSIC: [Note; 6] = [
    n(392, 110, 8),
    n(370, 110, 8),
    n(349, 110, 8),
    n(330, 150, 8),
    n(262, 220, 12),
    n(196, 280, 12),
];

const SFX_KEYPRESS_ARCADE: [Note; 2] = [n(988, 18, 2), n(1175, 20, 3)];
const SFX_PIECE_LOCK_ARCADE: [Note; 3] = [n(440, 22, 2), n(349, 24, 2), n(262, 45, 4)];
const SFX_LINE_CLEAR_ARCADE: [Note; 4] = [
    n(659, 35, 2),
    n(784, 35, 2),
    n(988, 35, 2),
    n(1175, 70, 8),
];
const SFX_LINE_CLEAR_TETRIS_ARCADE: [Note; 6] = [
    n(523, 32, 2),
    n(659, 32, 2),
    n(784, 32, 2),
    n(1047, 32, 2),
    n(1319, 70, 2),
    n(1568, 95, 8),
];
const SFX_GAME_OVER_ARCADE: [Note; 6] = [
    n(523, 90, 6),
    n(494, 90, 6),
    n(466, 90, 6),
    n(440, 120, 6),
    n(392, 170, 6),
    n(330, 220, 10),
];

const fn n(frequency_hz: u16, duration_ms: u16, rest_ms: u16) -> Note {
    Note {
        frequency_hz,
        duration_ms,
        rest_ms,
    }
}
