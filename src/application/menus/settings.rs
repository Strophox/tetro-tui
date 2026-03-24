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

use crate::application::{
    menus::{Menu, MenuUpdate},
    Application, SavefileGranularity,
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_settings(&mut self) -> io::Result<MenuUpdate> {
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "% Settings %").bold(),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let labels = [
                format!(
                    "Adjust graphics ({:?}) ...",
                    self.settings.graphics_slots[self.settings.graphics_slot_active].0
                ),
                format!(
                    "Adjust keybinds ({:?}) ...",
                    self.settings.keybinds_slots[self.settings.keybinds_slot_active].0
                ),
                format!(
                    "Adjust gameplay ({:?}) ...",
                    self.settings.gameplay_slots[self.settings.gameplay_slot_active].0
                ),
                format!(
                    "Keep save file: {}",
                    match self.temp_data.save_on_exit {
                        SavefileGranularity::NoSavefile => "No *",
                        SavefileGranularity::RememberSettings
                        | SavefileGranularity::RememberSettingsScores
                        | SavefileGranularity::RememberSettingsScoresReplays => "Yes",
                    }
                ),
                "Advanced settings...".to_owned(),
            ];

            let selection_len = labels.len();

            for (i, label) in labels.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(i).unwrap()
                            + if 2 < i { 1 } else { 0 },
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
                        if self.temp_data.save_on_exit == SavefileGranularity::NoSavefile {
                            "(*Caution: data will be wiped on exit)".to_owned()
                        } else {
                            "".to_owned()
                        },
                    )
                    .italic()
                    .with(
                        if self.temp_data.save_on_exit == SavefileGranularity::NoSavefile {
                            crossterm::style::Color::Yellow
                        } else {
                            crossterm::style::Color::Reset
                        },
                    ),
                ))?;
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
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e' | 'E'),
                    kind: Press,
                    ..
                }) => match selected {
                    0 => break Ok(MenuUpdate::Push(Menu::AdjustGraphics)),
                    1 => break Ok(MenuUpdate::Push(Menu::AdjustKeybinds)),
                    2 => break Ok(MenuUpdate::Push(Menu::AdjustGameplay)),
                    3 => {
                        self.temp_data.save_on_exit =
                            SavefileGranularity::RememberSettingsScoresReplays;
                    }
                    4 => break Ok(MenuUpdate::Push(Menu::AdvancedSettings)),
                    _ => {}
                },
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
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    match selected {
                        0 => {
                            self.settings.graphics_slot_active +=
                                self.settings.graphics_slots.len() + 1;
                            self.settings.graphics_slot_active %=
                                self.settings.graphics_slots.len();
                        }
                        1 => {
                            self.settings.keybinds_slot_active +=
                                self.settings.keybinds_slots.len() + 1;
                            self.settings.keybinds_slot_active %=
                                self.settings.keybinds_slots.len();
                        }
                        2 => {
                            self.settings.gameplay_slot_active +=
                                self.settings.gameplay_slots.len() + 1;
                            self.settings.gameplay_slot_active %=
                                self.settings.gameplay_slots.len();
                        }
                        3 => {
                            self.temp_data.save_on_exit = match self.temp_data.save_on_exit {
                                SavefileGranularity::NoSavefile
                                | SavefileGranularity::RememberSettingsScores
                                | SavefileGranularity::RememberSettings => {
                                    SavefileGranularity::RememberSettingsScoresReplays
                                }
                                SavefileGranularity::RememberSettingsScoresReplays => {
                                    SavefileGranularity::NoSavefile
                                }
                            };
                        }
                        4 => {}
                        // No accessible options beyond.
                        _ => {}
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    match selected {
                        0 => {
                            self.settings.graphics_slot_active +=
                                self.settings.graphics_slots.len() - 1;
                            self.settings.graphics_slot_active %=
                                self.settings.graphics_slots.len();
                        }
                        1 => {
                            self.settings.keybinds_slot_active +=
                                self.settings.keybinds_slots.len() - 1;
                            self.settings.keybinds_slot_active %=
                                self.settings.keybinds_slots.len();
                        }
                        2 => {
                            self.settings.gameplay_slot_active +=
                                self.settings.gameplay_slots.len() - 1;
                            self.settings.gameplay_slot_active %=
                                self.settings.gameplay_slots.len();
                        }
                        3 => {
                            self.temp_data.save_on_exit = match self.temp_data.save_on_exit {
                                SavefileGranularity::NoSavefile => {
                                    SavefileGranularity::RememberSettingsScoresReplays
                                }
                                SavefileGranularity::RememberSettingsScoresReplays
                                | SavefileGranularity::RememberSettingsScores
                                | SavefileGranularity::RememberSettings => {
                                    SavefileGranularity::NoSavefile
                                }
                            };
                        }
                        4 => {}
                        // No accessible options beyond.
                        _ => {}
                    }
                }

                // Set save_on_exit to false.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d' | 'D'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    match selected {
                        0 => {
                            self.settings.graphics_slot_active = 0;
                        }
                        1 => {
                            self.settings.keybinds_slot_active = 0;
                        }
                        2 => {
                            self.settings.gameplay_slot_active = 0;
                        }
                        3 => {
                            self.temp_data.save_on_exit = SavefileGranularity::NoSavefile;
                        }
                        4 => {}
                        // No accessible options beyond.
                        _ => {}
                    }
                }

                // Other event: Just ignore.
                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }
}
