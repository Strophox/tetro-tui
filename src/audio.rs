use std::{
    collections::VecDeque,
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crate::{
    core_game_engine::{Notification, NotificationFeed},
    settings::{AudioBackend, AudioSettings, SfxPack},
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

#[derive(Clone, Copy, Debug)]
struct PlaybackNote {
    frequency_hz: u16,
    duration_ms: u32,
    rest_ms: u32,
}

enum AudioCommand {
    PlaySfx(SoundEffect),
    Stop,
}

#[derive(Clone, Copy, Debug)]
enum AudioBackendState {
    Pending(AudioBackend),
    Active(AudioBackend),
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaybackKind {
    Theme,
    ThemeResume,
    Sfx,
}

struct ActiveNotePlayback {
    note: PlaybackNote,
    sound_deadline: Instant,
    deadline: Instant,
    kind: PlaybackKind,
    child: Option<Child>,
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
                    self.send(AudioCommand::PlaySfx(SoundEffect::LineClear {
                        lines: *lineclears,
                    }));
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
    let mut stop_requested = false;
    let mut backend_state = AudioBackendState::Pending(settings.backend);
    let mut active_note: Option<ActiveNotePlayback> = None;
    let mut theme_resume_note: Option<PlaybackNote> = None;
    let tempo_percent = settings.theme_tempo_percent.max(MIN_TEMPO_PERCENT);
    let theme = theme_notes(settings);

    loop {
        if stop_requested {
            stop_active_note(&mut active_note);
            break;
        }

        if let Some(playback) = active_note.as_mut()
            && Instant::now() >= playback.deadline
        {
            finish_active_note(&mut active_note);
        }

        if let Some(playback) = active_note.as_mut()
            && !queued_sfx.is_empty()
            && matches!(
                playback.kind,
                PlaybackKind::Theme | PlaybackKind::ThemeResume
            )
        {
            let now = Instant::now();
            let remaining_duration_ms = playback
                .sound_deadline
                .saturating_duration_since(now)
                .as_millis()
                .try_into()
                .unwrap_or(u32::MAX);
            let remaining_rest_ms = playback
                .deadline
                .saturating_duration_since(playback.sound_deadline.max(now))
                .as_millis()
                .try_into()
                .unwrap_or(u32::MAX);

            theme_resume_note =
                (remaining_duration_ms > 0 || remaining_rest_ms > 0).then_some(PlaybackNote {
                    frequency_hz: playback.note.frequency_hz,
                    duration_ms: remaining_duration_ms,
                    rest_ms: remaining_rest_ms,
                });
            stop_active_note(&mut active_note);
        }

        if active_note.is_none() {
            if let Some(notes) = queued_sfx.front_mut() {
                if let Some((note, rest)) = next_note_in_slice(notes) {
                    if rest.is_empty() {
                        queued_sfx.pop_front();
                    } else {
                        *notes = rest;
                    }
                    active_note = Some(play_note(
                        scale_note(note, tempo_percent),
                        PlaybackKind::Sfx,
                        &mut backend_state,
                    ));
                    continue;
                }
                queued_sfx.pop_front();
                continue;
            }

            if let Some(note) = theme_resume_note.take() {
                active_note = Some(play_note(
                    note,
                    PlaybackKind::ThemeResume,
                    &mut backend_state,
                ));
                continue;
            }

            if settings.theme_enabled {
                let note = theme[theme_index % theme.len()];
                theme_index = (theme_index + 1) % theme.len();
                active_note = Some(play_note(
                    scale_note(note, tempo_percent),
                    PlaybackKind::Theme,
                    &mut backend_state,
                ));
                continue;
            }
        }

        let timeout = active_note
            .as_ref()
            .map(|playback| playback.deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(POLL_INTERVAL)
            .min(POLL_INTERVAL);

        match receiver.recv_timeout(timeout) {
            Ok(AudioCommand::PlaySfx(effect)) => {
                queued_sfx.push_back(notes_for_sfx(effect, settings))
            }
            Ok(AudioCommand::Stop) => stop_requested = true,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        drain_commands(&receiver, &mut queued_sfx, settings, &mut stop_requested);
    }
}

fn drain_commands(
    receiver: &mpsc::Receiver<AudioCommand>,
    queued_sfx: &mut VecDeque<&'static [Note]>,
    settings: AudioSettings,
    stop_requested: &mut bool,
) {
    while let Ok(command) = receiver.try_recv() {
        match command {
            AudioCommand::PlaySfx(effect) => queued_sfx.push_back(notes_for_sfx(effect, settings)),
            AudioCommand::Stop => *stop_requested = true,
        }
    }
}

fn next_note_in_slice(notes: &'static [Note]) -> Option<(Note, &'static [Note])> {
    notes.split_first().map(|(first, rest)| (*first, rest))
}

fn finish_active_note(active_note: &mut Option<ActiveNotePlayback>) {
    if let Some(playback) = active_note.as_mut()
        && let Some(child) = playback.child.as_mut()
    {
        let _ = child.try_wait();
    }
    *active_note = None;
}

fn stop_active_note(active_note: &mut Option<ActiveNotePlayback>) {
    if let Some(playback) = active_note.as_mut()
        && let Some(child) = playback.child.as_mut()
    {
        let _ = child.kill();
        let _ = child.wait();
    }
    *active_note = None;
}

fn play_note(
    note: PlaybackNote,
    kind: PlaybackKind,
    backend_state: &mut AudioBackendState,
) -> ActiveNotePlayback {
    let start = Instant::now();
    let child = if note.frequency_hz == 0 || note.duration_ms == 0 {
        None
    } else {
        spawn_note(note, backend_state)
    };

    ActiveNotePlayback {
        note,
        sound_deadline: start + Duration::from_millis(u64::from(note.duration_ms)),
        deadline: start + Duration::from_millis(u64::from(note.duration_ms + note.rest_ms)),
        kind,
        child,
    }
}

fn spawn_note(note: PlaybackNote, backend_state: &mut AudioBackendState) -> Option<Child> {
    match *backend_state {
        AudioBackendState::Pending(AudioBackend::Auto) => {
            if let Some(child) = spawn_with_backend(AudioBackend::PcSpeakerBeep, note) {
                *backend_state = AudioBackendState::Active(AudioBackend::PcSpeakerBeep);
                Some(child)
            } else if let Some(child) = spawn_with_backend(AudioBackend::SoundCardSox, note) {
                *backend_state = AudioBackendState::Active(AudioBackend::SoundCardSox);
                Some(child)
            } else {
                *backend_state = AudioBackendState::Unavailable;
                None
            }
        }
        AudioBackendState::Pending(backend) | AudioBackendState::Active(backend) => {
            if let Some(child) = spawn_with_backend(backend, note) {
                *backend_state = AudioBackendState::Active(backend);
                Some(child)
            } else {
                *backend_state = AudioBackendState::Unavailable;
                None
            }
        }
        AudioBackendState::Unavailable => None,
    }
}

fn spawn_with_backend(backend: AudioBackend, note: PlaybackNote) -> Option<Child> {
    match backend {
        AudioBackend::Auto => None,
        AudioBackend::PcSpeakerBeep => Command::new("beep")
            .arg("-f")
            .arg(note.frequency_hz.to_string())
            .arg("-l")
            .arg(note.duration_ms.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok(),
        AudioBackend::SoundCardSox => Command::new("sox")
            .arg("-q")
            .arg("-n")
            .arg("-d")
            .arg("synth")
            .arg(format!("{:.3}", f64::from(note.duration_ms) / 1000.0))
            .arg("sine")
            .arg(note.frequency_hz.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok(),
    }
}

fn scale_note(note: Note, tempo_percent: u16) -> PlaybackNote {
    let duration_ms = (u32::from(note.duration_ms) * 100)
        .checked_div(u32::from(tempo_percent))
        .unwrap_or(u32::from(note.duration_ms))
        .clamp(1, 10_000);
    let rest_ms = (u32::from(note.rest_ms) * 100)
        .checked_div(u32::from(tempo_percent))
        .unwrap_or(u32::from(note.rest_ms))
        .clamp(0, 10_000);

    PlaybackNote {
        frequency_hz: note.frequency_hz,
        duration_ms,
        rest_ms,
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

fn theme_notes(_settings: AudioSettings) -> &'static [Note] {
    &THEME_KOROBEINIKI
}

const THEME_KOROBEINIKI: [Note; 38] = [
    n(659, 400, 12),
    n(494, 200, 12),
    n(523, 200, 12),
    n(587, 400, 12),
    n(523, 200, 12),
    n(494, 200, 12),
    n(440, 400, 12),
    n(440, 200, 12),
    n(523, 200, 12),
    n(659, 400, 12),
    n(587, 200, 12),
    n(523, 200, 12),
    n(494, 600, 20),
    n(523, 200, 12),
    n(587, 400, 12),
    n(659, 400, 12),
    n(523, 400, 12),
    n(440, 400, 12),
    n(440, 800, 20),
    n(0, 200, 12),
    n(587, 400, 12),
    n(698, 200, 12),
    n(880, 400, 12),
    n(784, 200, 12),
    n(698, 200, 12),
    n(659, 600, 12),
    n(523, 200, 12),
    n(659, 400, 12),
    n(587, 200, 12),
    n(523, 200, 12),
    n(494, 400, 12),
    n(494, 200, 12),
    n(523, 200, 12),
    n(587, 400, 12),
    n(659, 400, 12),
    n(523, 400, 12),
    n(440, 400, 12),
    n(440, 800, 20),
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
const SFX_LINE_CLEAR_ARCADE: [Note; 4] =
    [n(659, 35, 2), n(784, 35, 2), n(988, 35, 2), n(1175, 70, 8)];
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

const MIN_TEMPO_PERCENT: u16 = 20;
const POLL_INTERVAL: Duration = Duration::from_millis(5);
