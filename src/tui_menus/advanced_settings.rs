use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
    QueueableCommand,
};

use crate::{
    fmt_helpers::BoolAsOnOff,
    game_renderers::TetroTUIRenderer,
    tui_menus::{title_bar, Menu, MenuUpdate},
    Application, SavefileGranularity,
};

impl<T: Write> Application<T> {
    pub fn run_menu_advanced_settings(&mut self) -> io::Result<MenuUpdate> {
        let mut selected = 0usize;
        loop {
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
                .queue(Print(format!("{:^w_main$}", title_bar(&self.settings))))?;

            // Draw config selection.
            let labels = [
                format!(
                    "Save contents: {}",
                    match self.temp_data.save_on_exit {
                        SavefileGranularity::NoSavefile => "--Nothing",
                        SavefileGranularity::StoreSettings => "Only settings --No scores,replays",
                        SavefileGranularity::StoreSettingsScores =>
                            "Only settings,scores --No replays",
                        SavefileGranularity::StoreSettingsScoresReplays =>
                            "Everything (settings,scores,replays)",
                    }
                ),
                format!(
                    "Assume enhanced-key-events available = {} *",
                    self.temp_data.kitty_assumed.on_off()
                ),
                format!(
                    "Pause on focus lost = {} (may not work on some terminals)",
                    self.temp_data.pause_on_focus_lost.on_off()
                ),
                format!(
                    "'Blindfold' game = {}",
                    self.temp_data.blindfold_game.on_off()
                ),
                format!(
                    "Renderer used = {} (applies on New Game)",
                    TetroTUIRenderer::with_number(self.temp_data.renderer_used).name()
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
                            format!(">> {label} <<")
                        } else {
                            label
                        }
                    )))?;
            }

            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(selection_len).unwrap() + 2,
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        if self.temp_data.kitty_detected {
                            "(*Should apply, since terminal seems to support enhanced-key-events)"
                        } else {
                            "(*Unlikely to apply, enhanced-key-events seem unsupported by terminal)"
                        },
                    )
                    .italic(),
                ))?;

            let mut temp_offset = 0;
            if self.temp_data.save_on_exit != SavefileGranularity::NoSavefile {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(selection_len).unwrap() + 3,
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            format!("Save location: {}", self.temp_data.savefile_path.display())
                        )
                        .italic(),
                    ))?;
                temp_offset += 1;
            }

            if let Err(e) = &self.temp_data.loadfile_result {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(selection_len).unwrap()
                            + 3
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
                            + 3
                            + temp_offset
                            + 1,
                    ))?
                    .queue(PrintStyledContent(
                        format!("{:^w_main$}", format!("{e}")).italic(),
                    ))?;
                temp_offset += 1;
            }

            if let Err(e) = &self.temp_data.storefile_result {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(selection_len).unwrap()
                            + 3
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
                            + 3
                            + temp_offset
                            + 1,
                    ))?
                    .queue(PrintStyledContent(
                        format!("{:^w_main$}", format!("{e}")).italic(),
                    ))?;
            }

            self.term.flush()?;
            // Wait for new input.
            match event::read()? {
                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c' | 'C'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => break Ok(MenuUpdate::Push(Menu::Quit)),
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
                        self.temp_data.kitty_assumed = self.temp_data.kitty_detected;
                    }
                    2 => {
                        self.temp_data.pause_on_focus_lost = false;
                    }
                    3 => {
                        self.temp_data.blindfold_game = false;
                    }
                    4 => {
                        self.temp_data.renderer_used = 0;
                    }
                    _ => {}
                },

                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e' | 'E'),
                    kind: Press,
                    ..
                }) => {
                    if selected == 0 {
                        self.temp_data.save_on_exit =
                            SavefileGranularity::StoreSettingsScoresReplays;
                    }
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
                    self.temp_data.loadfile_result = self.savefile_load();
                }

                // Store to savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('s' | 'S'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.storefile_result = self.savefile_store();
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
                        self.temp_data.kitty_assumed ^= true;
                    }
                    2 => {
                        self.temp_data.pause_on_focus_lost ^= true;
                    }
                    3 => {
                        self.temp_data.blindfold_game ^= true;
                    }
                    4 => {
                        self.temp_data.renderer_used += 1;
                        self.temp_data.renderer_used %= TetroTUIRenderer::NUM_VARIANTS;
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
                        self.temp_data.kitty_assumed ^= true;
                    }
                    2 => {
                        self.temp_data.pause_on_focus_lost ^= true;
                    }
                    3 => {
                        self.temp_data.blindfold_game ^= true;
                    }
                    4 => {
                        self.temp_data.renderer_used += TetroTUIRenderer::NUM_VARIANTS - 1;
                        self.temp_data.renderer_used %= TetroTUIRenderer::NUM_VARIANTS;
                    }
                    _ => {}
                },
                // Other event: don't care.
                _ => {}
            }
            selected %= selection_len;
        }
    }
}
