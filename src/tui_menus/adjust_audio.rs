use std::io::{self, Write};

use crossterm::{
    QueueableCommand,
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{Color, PrintStyledContent, Stylize},
    terminal::{self, Clear, ClearType},
};

use crate::{
    Application,
    settings::{AudioBackend, SfxPack, audio_backend_is_available},
    tui_menus::{Menu, MenuUpdate, heading_line},
};

impl<W: Write> Application<W> {
    pub fn run_menu_adjust_audio(&mut self) -> io::Result<MenuUpdate> {
        let mut selected = 0usize;
        loop {
            if self.settings.tui_coloring().bg_tui == Color::Reset {
                self.term.queue(Clear(ClearType::All))?;
            } else {
                self.term.queue(MoveTo(0, 0))?.queue(PrintStyledContent({
                    let (w, h) = terminal::size()?;
                    " ".repeat(w as usize * h as usize)
                        .on(self.settings.tui_coloring().bg_tui)
                }))?;
            }
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();
            let y_selection = (Self::H_MAIN / 5).saturating_sub(1);
            self.term
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "= Audio Settings =")
                        .bold()
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", heading_line(&self.settings))
                        .with(self.settings.tui_coloring().fg_accent)
                        .on(self.settings.tui_coloring().bg_tui),
                ))?;

            let labels = [
                format!("Audio enabled = {}", on_off(self.settings.audio.enabled)),
                format!(
                    "Audio output = {}{}",
                    fmt_audio_backend(self.settings.audio.backend),
                    backend_status_suffix(self.settings.audio.backend)
                ),
                format!(
                    "Theme (BGM) enabled = {}",
                    on_off(self.settings.audio.theme_enabled)
                ),
                format!(
                    "Sound effects enabled = {}",
                    on_off(self.settings.audio.sfx_enabled)
                ),
                format!(
                    "Theme song = {}",
                    fmt_theme_song(self.settings.audio.theme_song)
                ),
                format!(
                    "Theme tempo = {}%",
                    self.settings.audio.theme_tempo_percent.clamp(20, 250)
                ),
                format!("SFX pack = {}", fmt_sfx_pack(self.settings.audio.sfx_pack)),
                format!(
                    "Keypress SFX = {}",
                    on_off(self.settings.audio.keypress_sfx)
                ),
                format!(
                    "Piece lock SFX = {}",
                    on_off(self.settings.audio.piece_lock_sfx)
                ),
                format!(
                    "Line clear SFX = {}",
                    on_off(self.settings.audio.line_clear_sfx)
                ),
                format!(
                    "Game over SFX = {}",
                    on_off(self.settings.audio.game_over_sfx)
                ),
            ];

            let selection_len = labels.len();
            for (i, label) in labels.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            if i == selected {
                                format!(
                                    "{} {label} {}",
                                    self.settings.tui_symbols().menu_pointers[0],
                                    self.settings.tui_symbols().menu_pointers[1]
                                )
                            } else {
                                label
                            }
                        )
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui),
                    ))?;
            }

            self.term.flush()?;

            match event::read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c' | 'C'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    ..
                }) => break Ok(MenuUpdate::Push(Menu::Quit)),

                Event::Key(KeyEvent {
                    code: KeyCode::Char('?'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    let client_menu_name = "Audio Settings menu";
                    let legend = vec![(
                        "Normal keybinds".to_owned(),
                        [
                            ("Escape Backspace q", "Exit menu"),
                            ("Delete d", "Reset audio settings"),
                            ("↓/↑ j/k", "Navigate down/up"),
                            ("←/→ h/l", "Adjust value"),
                            ("?", "Open Keybinds overview"),
                        ]
                        .into_iter()
                        .map(|(lhs, rhs)| (lhs.to_owned(), rhs.to_owned()))
                        .collect(),
                    )];

                    break Ok(MenuUpdate::Push(Menu::KeybindsOverview {
                        client_menu_name,
                        legend,
                    }));
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q' | 'Q') | KeyCode::Backspace,
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k' | 'K'),
                    kind: Press | Repeat,
                    ..
                }) => selected += selection_len - 1,

                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j' | 'J'),
                    kind: Press | Repeat,
                    ..
                }) => selected += 1,

                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d' | 'D'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    self.settings.audio = Default::default();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if !modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) => {
                    adjust_audio(&mut self.settings.audio, selected, true)
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if !modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) => {
                    adjust_audio(&mut self.settings.audio, selected, false)
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('l' | 'L'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.load_savefile_result = self.savefile_read();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('s' | 'S'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.store_savefile_result = self.savefile_write();
                }

                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }
}

fn on_off(flag: bool) -> &'static str {
    if flag { "On" } else { "Off" }
}

fn fmt_theme_song(_song: crate::settings::ThemeSong) -> &'static str {
    "Korobeiniki"
}

fn fmt_audio_backend(backend: AudioBackend) -> &'static str {
    match backend {
        AudioBackend::Auto => "Auto (beep -> sox)",
        AudioBackend::PcSpeakerBeep => "PC speaker (beep)",
        AudioBackend::SoundCardSox => "Sound card (sox)",
    }
}

fn fmt_sfx_pack(pack: SfxPack) -> &'static str {
    match pack {
        SfxPack::Classic => "Classic",
        SfxPack::Arcade => "Arcade",
    }
}

fn adjust_audio(settings: &mut crate::settings::AudioSettings, selected: usize, increase: bool) {
    match selected {
        0 => settings.enabled ^= true,
        1 => {
            settings.backend = match (settings.backend, increase) {
                (AudioBackend::Auto, true) | (AudioBackend::PcSpeakerBeep, false) => {
                    AudioBackend::PcSpeakerBeep
                }
                (AudioBackend::PcSpeakerBeep, true) | (AudioBackend::SoundCardSox, false) => {
                    AudioBackend::SoundCardSox
                }
                (AudioBackend::SoundCardSox, true) | (AudioBackend::Auto, false) => {
                    AudioBackend::Auto
                }
            };
        }
        2 => settings.theme_enabled ^= true,
        3 => settings.sfx_enabled ^= true,
        4 => {}
        5 => {
            let delta = if increase { 5 } else { -5 };
            settings.theme_tempo_percent =
                (i32::from(settings.theme_tempo_percent) + delta).clamp(20, 250) as u16;
        }
        6 => {
            settings.sfx_pack = match (settings.sfx_pack, increase) {
                (SfxPack::Classic, true) | (SfxPack::Arcade, false) => SfxPack::Arcade,
                (SfxPack::Arcade, true) | (SfxPack::Classic, false) => SfxPack::Classic,
            };
        }
        7 => settings.keypress_sfx ^= true,
        8 => settings.piece_lock_sfx ^= true,
        9 => settings.line_clear_sfx ^= true,
        10 => settings.game_over_sfx ^= true,
        _ => {}
    }
}

fn backend_status_suffix(backend: AudioBackend) -> &'static str {
    if audio_backend_is_available(backend) {
        ""
    } else {
        " [missing]"
    }
}
