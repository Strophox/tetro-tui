use std::io::{self, Write};

use crossterm::{
    ExecutableCommand, QueueableCommand,
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{self, Press, Repeat},
        KeyModifiers,
    },
    style::{Print, PrintStyledContent, Stylize},
    terminal::{self, Clear, ClearType},
};

use crate::{
    Application, SavefileGranularity,
    fmt_helpers::BoolAsOnOff,
    game_renderers::MiscGameRenderers,
    tui_menus::{Menu, MenuUpdate, heading_line},
};

impl<W: Write> Application<W> {
    pub fn run_menu_advanced_settings(&mut self) -> io::Result<MenuUpdate> {
        if self.temp_data.kitty_assumed {
            let f = Self::GAME_KEYBOARD_ENHANCEMENT_FLAGS;
            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
            let _r = self.term.execute(event::PushKeyboardEnhancementFlags(f));
        }

        let mut selected = 0usize;
        let mut latest_input_info: Option<(KeyCode, KeyModifiers, KeyEventKind)> = None;
        let menu_update = loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();
            let y_selection = Self::H_MAIN / 5;

            // Draw menu title.
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "§ Advanced Settings §").bold(),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", heading_line(&self.settings))))?;

            // Draw config selection.
            let warning_star = if self.temp_data.kitty_detected {
                ""
            } else {
                " *"
            };
            let labels = [
                format!(
                    "Save: {}",
                    match self.temp_data.save_on_exit {
                        SavefileGranularity::NoSavefile => "--Nothing",
                        SavefileGranularity::StoreSettings =>
                            "Only settings,stats --No scoreboard,replays",
                        SavefileGranularity::StoreSettingsScores =>
                            "Only settings,stats,scoreboard --No replays",
                        SavefileGranularity::StoreSettingsScoresReplays =>
                            "Everything (settings,stats,scoreboard,replays)",
                    }
                ),
                format!(
                    "Renderer used = {} (applies on New Game)",
                    MiscGameRenderers::name_from_num(self.temp_data.renderer_used)
                ),
                format!(
                    "Pause on focus lost = {} (experimental)",
                    self.temp_data.pause_on_focus_lost.on_off()
                ),
                format!(
                    "'Blindfold' game = {}",
                    self.temp_data.blindfold_game.on_off()
                ),
                format!(
                    "Assume enhanced-key-events available = {}{warning_star}",
                    self.temp_data.kitty_assumed.on_off()
                ),
            ];

            let selection_len = labels.len();

            for (i, label) in labels.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!(
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
                    )))?;
            }

            let mut temp_offset = 0;

            if !self.temp_data.kitty_detected {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(selection_len).unwrap()
                            + 1
                            + temp_offset,
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            "(*Unlikely to work; Enhanced-key-events seem unsupported by terminal)"
                        )
                        .italic(),
                    ))?;
                temp_offset += 1;
            }

            if self.temp_data.save_on_exit != SavefileGranularity::NoSavefile {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(selection_len).unwrap()
                            + 1
                            + temp_offset,
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            format!("Savefile path: {}", self.temp_data.savefile_path.display())
                        )
                        .italic(),
                    ))?;
                temp_offset += 1;
            }

            self.term
                .queue(MoveTo(
                    x_main,
                    y_main
                        + y_selection
                        + 4
                        + u16::try_from(selection_len).unwrap()
                        + 1
                        + temp_offset,
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        format!(
                            "Current terminal size (>=58x24 recommended): {:?}",
                            terminal::size()?
                        )
                    )
                    .italic(),
                ))?;
            temp_offset += 1;

            if let Err(e) = &self.temp_data.load_savefile_result {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(selection_len).unwrap()
                            + 1
                            + temp_offset,
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            format!("Latest error from trying to load savefile:")
                        )
                        .italic(),
                    ))?
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(selection_len).unwrap()
                            + 1
                            + temp_offset
                            + 1,
                    ))?
                    .queue(PrintStyledContent(
                        format!("{:^w_main$}", format!("'{e}'")).italic(),
                    ))?;
                temp_offset += 2;
            }

            if let Err(e) = &self.temp_data.store_savefile_result {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(selection_len).unwrap()
                            + 1
                            + temp_offset,
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            format!("Latest error from trying to store savefile:")
                        )
                        .italic(),
                    ))?
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(selection_len).unwrap()
                            + 1
                            + temp_offset
                            + 1,
                    ))?
                    .queue(PrintStyledContent(
                        format!("{:^w_main$}", format!("'{e}'")).italic(),
                    ))?;
                temp_offset += 2;
            }

            if let Some((code, modifiers, kind)) = latest_input_info {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(selection_len).unwrap()
                            + 1
                            + temp_offset,
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            format!("Latest input: {kind:?} {modifiers:?} {code:?}")
                        )
                        .italic(),
                    ))?;
            }

            self.term.flush()?;
            // Wait for new input.
            let evt = event::read()?;
            if let Event::Key(KeyEvent {
                code,
                modifiers,
                kind,
                state: _,
            }) = evt
            {
                latest_input_info = Some((code, modifiers, kind));
            }
            match evt {
                // Abort program.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c' | 'C'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => break Ok(MenuUpdate::Push(Menu::Quit)),

                // Keybinds help menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('?'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    let client_menu_name = "Advanced Settings menu";
                    let legend = vec![
                        (
                            "Normal keybinds".to_owned(),
                            [
                                ("Escape q Backspace", "Exit menu"),
                                ("Delete d", "Reset value to default"),
                                ("↓/↑ j/k", "Navigate down/up"),
                                ("←/→ h/l", "Adjust value"),
                                ("?", "Open Keybinds overview"),
                            ]
                            .into_iter()
                            .map(|(lhs, rhs)| (lhs.to_owned(), rhs.to_owned()))
                            .collect(),
                        ),
                        (
                            "Special keybinds".to_owned(),
                            [
                                ("Ctrl+C", "Quit program (respects save preferences)"),
                                (
                                    "Ctrl+Alt+S",
                                    "Perform savefile store (respects save preferences)",
                                ),
                                (
                                    "Ctrl+Alt+L",
                                    "Reload app from savefile (overwrites current data!)",
                                ),
                            ]
                            .into_iter()
                            .map(|(lhs, rhs)| (lhs.to_owned(), rhs.to_owned()))
                            .collect(),
                        ),
                    ];

                    break Ok(MenuUpdate::Push(Menu::KeybindsOverview {
                        client_menu_name,
                        legend,
                    }));
                }

                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q' | 'Q') | KeyCode::Backspace,
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d' | 'D'),
                    kind: Press,
                    ..
                }) => match selected {
                    0 => {
                        self.temp_data.save_on_exit = SavefileGranularity::NoSavefile;
                    }
                    1 => {
                        self.temp_data.renderer_used = 0;
                    }
                    2 => {
                        self.temp_data.pause_on_focus_lost = false;
                    }
                    3 => {
                        self.temp_data.blindfold_game = false;
                    }
                    4 => {
                        if !self.temp_data.kitty_assumed && self.temp_data.kitty_detected {
                            let f = Self::GAME_KEYBOARD_ENHANCEMENT_FLAGS;
                            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
                            let _r: Result<&mut W, io::Error> =
                                self.term.execute(event::PushKeyboardEnhancementFlags(f));
                        } else if self.temp_data.kitty_assumed && !self.temp_data.kitty_detected {
                            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
                            let _r = self.term.execute(event::PopKeyboardEnhancementFlags);
                        }
                        self.temp_data.kitty_assumed = self.temp_data.kitty_detected;
                    }
                    _ => {}
                },

                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e' | 'E'),
                    kind: Press,
                    ..
                }) if selected == 0 => {
                    self.temp_data.save_on_exit = SavefileGranularity::StoreSettingsScoresReplays;
                }

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k' | 'K'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += selection_len - 1;
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j' | 'J'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += 1;
                }

                // Reload from savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('l' | 'L'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.load_savefile_result = self.savefile_load();
                }

                // Store to savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('s' | 'S'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.store_savefile_result = self.savefile_store();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.temp_data.save_on_exit = match self.temp_data.save_on_exit {
                            SavefileGranularity::NoSavefile => SavefileGranularity::StoreSettings,
                            SavefileGranularity::StoreSettings => {
                                SavefileGranularity::StoreSettingsScores
                            }
                            SavefileGranularity::StoreSettingsScores => {
                                SavefileGranularity::StoreSettingsScoresReplays
                            }
                            SavefileGranularity::StoreSettingsScoresReplays => {
                                SavefileGranularity::NoSavefile
                            }
                        };
                    }
                    1 => {
                        self.temp_data.renderer_used += 1;
                        self.temp_data.renderer_used %= MiscGameRenderers::NUM_VARIANTS;
                    }
                    2 => {
                        self.temp_data.pause_on_focus_lost ^= true;
                    }
                    3 => {
                        self.temp_data.blindfold_game ^= true;
                    }
                    4 => {
                        self.temp_data.kitty_assumed ^= true;
                        if self.temp_data.kitty_assumed {
                            let f = Self::GAME_KEYBOARD_ENHANCEMENT_FLAGS;
                            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
                            let _r: Result<&mut W, io::Error> =
                                self.term.execute(event::PushKeyboardEnhancementFlags(f));
                        } else {
                            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
                            let _r = self.term.execute(event::PopKeyboardEnhancementFlags);
                        }
                    }
                    _ => {}
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.temp_data.save_on_exit = match self.temp_data.save_on_exit {
                            SavefileGranularity::NoSavefile => {
                                SavefileGranularity::StoreSettingsScoresReplays
                            }
                            SavefileGranularity::StoreSettingsScoresReplays => {
                                SavefileGranularity::StoreSettingsScores
                            }
                            SavefileGranularity::StoreSettingsScores => {
                                SavefileGranularity::StoreSettings
                            }
                            SavefileGranularity::StoreSettings => SavefileGranularity::NoSavefile,
                        };
                    }
                    1 => {
                        self.temp_data.renderer_used += MiscGameRenderers::NUM_VARIANTS - 1;
                        self.temp_data.renderer_used %= MiscGameRenderers::NUM_VARIANTS;
                    }
                    2 => {
                        self.temp_data.pause_on_focus_lost ^= true;
                    }
                    3 => {
                        self.temp_data.blindfold_game ^= true;
                    }
                    4 => {
                        self.temp_data.kitty_assumed ^= true;
                        if self.temp_data.kitty_assumed {
                            let f = Self::GAME_KEYBOARD_ENHANCEMENT_FLAGS;
                            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
                            let _r: Result<&mut W, io::Error> =
                                self.term.execute(event::PushKeyboardEnhancementFlags(f));
                        } else {
                            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
                            let _r = self.term.execute(event::PopKeyboardEnhancementFlags);
                        }
                    }
                    _ => {}
                },
                // Other event: don't care.
                _ => {}
            }
            selected %= selection_len;
        };

        if self.temp_data.kitty_assumed {
            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
            let _r = self.term.execute(event::PopKeyboardEnhancementFlags);
        }

        menu_update
    }
}
