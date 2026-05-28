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
    Application, SavefileGranularity,
    core_game_engine::Tetromino,
    tui_menus::{Menu, MenuUpdate, heading_line},
};

impl<W: Write> Application<W> {
    pub fn run_menu_settings(&mut self) -> io::Result<MenuUpdate> {
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
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "% Settings %")
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
                format!(
                    "Adjust graphics ({}) ...",
                    self.settings
                        .graphics_slotmachine
                        .grab(self.settings.graphics_selected)
                        .0
                ),
                format!(
                    "Adjust keybinds ({}) ...",
                    self.settings
                        .keybinds_slotmachine
                        .grab(self.settings.keybinds_selected)
                        .0
                ),
                format!(
                    "Adjust gameplay ({}) ...",
                    self.settings
                        .gameplay_slotmachine
                        .grab(self.settings.gameplay_selected)
                        .0
                ),
                format!(
                    "Keep save file: {}",
                    match self.temp_data.save_on_exit {
                        SavefileGranularity::NoSavefile => "No *",
                        SavefileGranularity::StoreSettings
                        | SavefileGranularity::StoreSettingsScores
                        | SavefileGranularity::StoreSettingsScoresReplays => "Yes",
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
                            self.settings
                                .tile_coloring()
                                .simplified_tile_col(Tetromino::O.into(), 0)
                            // self.settings.tui_coloring().fg_accent
                        } else {
                            self.settings.tui_coloring().fg_tui
                        },
                    )
                    .on(self.settings.tui_coloring().bg_tui),
                ))?;
            self.term.flush()?;
            // Wait for new input.
            match event::read()? {
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
                    let client_menu_name = "Settings menu";
                    let legend = vec![
                        (
                            "Normal keybinds".to_owned(),
                            [
                                ("Enter e", "Select"),
                                ("Escape Backspace q", "Exit menu"),
                                (
                                    "Delete d",
                                    "Reset slot to default, reset Keep save file setting",
                                ),
                                ("↓/↑ j/k", "Navigate down/up"),
                                ("←/→ h/l", "Change slot, adjust value"),
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
                            SavefileGranularity::StoreSettingsScoresReplays;
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

                // Reload from savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('l' | 'L'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.load_savefile_result = self.savefile_read();
                }

                // Store to savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('s' | 'S'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.store_savefile_result = self.savefile_write();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    match selected {
                        0 => {
                            self.settings.graphics_selected +=
                                self.settings.graphics_slotmachine.slots.len() + 1;
                            self.settings.graphics_selected %=
                                self.settings.graphics_slotmachine.slots.len();
                        }
                        1 => {
                            self.settings.keybinds_selected +=
                                self.settings.keybinds_slotmachine.slots.len() + 1;
                            self.settings.keybinds_selected %=
                                self.settings.keybinds_slotmachine.slots.len();
                        }
                        2 => {
                            self.settings.gameplay_selected +=
                                self.settings.gameplay_slotmachine.slots.len() + 1;
                            self.settings.gameplay_selected %=
                                self.settings.gameplay_slotmachine.slots.len();
                        }
                        3 => {
                            self.temp_data.save_on_exit = match self.temp_data.save_on_exit {
                                SavefileGranularity::NoSavefile
                                | SavefileGranularity::StoreSettingsScores
                                | SavefileGranularity::StoreSettings => {
                                    SavefileGranularity::StoreSettingsScoresReplays
                                }
                                SavefileGranularity::StoreSettingsScoresReplays => {
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
                            self.settings.graphics_selected +=
                                self.settings.graphics_slotmachine.slots.len() - 1;
                            self.settings.graphics_selected %=
                                self.settings.graphics_slotmachine.slots.len();
                        }
                        1 => {
                            self.settings.keybinds_selected +=
                                self.settings.keybinds_slotmachine.slots.len() - 1;
                            self.settings.keybinds_selected %=
                                self.settings.keybinds_slotmachine.slots.len();
                        }
                        2 => {
                            self.settings.gameplay_selected +=
                                self.settings.gameplay_slotmachine.slots.len() - 1;
                            self.settings.gameplay_selected %=
                                self.settings.gameplay_slotmachine.slots.len();
                        }
                        3 => {
                            self.temp_data.save_on_exit = match self.temp_data.save_on_exit {
                                SavefileGranularity::NoSavefile => {
                                    SavefileGranularity::StoreSettingsScoresReplays
                                }
                                SavefileGranularity::StoreSettingsScoresReplays
                                | SavefileGranularity::StoreSettingsScores
                                | SavefileGranularity::StoreSettings => {
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
                            self.settings.graphics_selected = 0;
                        }
                        1 => {
                            self.settings.keybinds_selected = 0;
                        }
                        2 => {
                            self.settings.gameplay_selected = 0;
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
