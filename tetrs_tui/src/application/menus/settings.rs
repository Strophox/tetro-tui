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

use crate::application::{Application, Menu, MenuUpdate, SavefileGranularity};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_settings(&mut self) -> io::Result<MenuUpdate> {
        let selection_len = 4;
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "% Settings %")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let labels = [
                "Adjust Graphics...".to_owned(),
                "Adjust Keybinds...".to_owned(),
                "Adjust Gameplay...".to_owned(),
                format!(
                    "Keep save file: {}",
                    match self.save_on_exit {
                        SavefileGranularity::NoSavefile => "OFF*",
                        SavefileGranularity::RememberSettings => "ON (save settings)",
                        SavefileGranularity::RememberSettingsScoreboard =>
                            "ON (save settings, scores)",
                        SavefileGranularity::RememberSettingsScoreboardGamerecords =>
                            "ON (save settings, scores, game replays)",
                    }
                ),
            ];
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
                    y_main + y_selection + 4 + u16::try_from(selection_len).unwrap() + 1,
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        if self.save_on_exit == SavefileGranularity::NoSavefile {
                            "(*Caution: no data will not be stored on exit)".to_owned()
                        } else {
                            format!("(Save file - {:?})", Self::savefile_path())
                        },
                    )
                    .italic()
                    .with(
                        if self.save_on_exit == SavefileGranularity::NoSavefile {
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
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => {
                    break Ok(MenuUpdate::Push(Menu::Quit(
                        "exited with ctrl-c".to_owned(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e'),
                    kind: Press,
                    ..
                }) => match selected {
                    0 => break Ok(MenuUpdate::Push(Menu::AdjustGraphics)),
                    1 => break Ok(MenuUpdate::Push(Menu::AdjustKeybinds)),
                    2 => break Ok(MenuUpdate::Push(Menu::AdjustGameplay)),
                    3 => {
                        self.save_on_exit =
                            SavefileGranularity::RememberSettingsScoreboardGamerecords;
                    }
                    _ => {}
                },
                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += selection_len - 1;
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += 1;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 3 {
                        self.save_on_exit = match self.save_on_exit {
                            SavefileGranularity::NoSavefile => {
                                SavefileGranularity::RememberSettingsScoreboardGamerecords
                            }
                            SavefileGranularity::RememberSettingsScoreboardGamerecords => {
                                SavefileGranularity::RememberSettingsScoreboard
                            }
                            SavefileGranularity::RememberSettingsScoreboard => {
                                SavefileGranularity::RememberSettings
                            }
                            SavefileGranularity::RememberSettings => {
                                SavefileGranularity::NoSavefile
                            }
                        };
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 3 {
                        self.save_on_exit = match self.save_on_exit {
                            SavefileGranularity::NoSavefile => {
                                SavefileGranularity::RememberSettings
                            }
                            SavefileGranularity::RememberSettings => {
                                SavefileGranularity::RememberSettingsScoreboard
                            }
                            SavefileGranularity::RememberSettingsScoreboard => {
                                SavefileGranularity::RememberSettingsScoreboardGamerecords
                            }
                            SavefileGranularity::RememberSettingsScoreboardGamerecords => {
                                SavefileGranularity::NoSavefile
                            }
                        };
                    }
                }

                // Set save_on_exit to false.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 3 {
                        self.save_on_exit = SavefileGranularity::NoSavefile;
                    }
                }

                // Other event: Just ignore.
                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }
}
