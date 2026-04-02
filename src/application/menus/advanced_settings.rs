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
    application::{
        menus::{Menu, MenuUpdate},
        Application, SavefileGranularity,
    },
    fmt_helpers::FmtBool,
    game_renderers::TetroTUIRenderer,
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_advanced_settings(&mut self) -> io::Result<MenuUpdate> {
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;

            // Draw menu title.
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "§ Advanced Settings §").bold(),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw config selection.
            let labels = [
                format!(
                    "Save contents: {}",
                    match self.temp_data.save_on_exit {
                        SavefileGranularity::NoSavefile => "--Nothing",
                        SavefileGranularity::RememberSettings =>
                            "Only settings --No scores,replays",
                        SavefileGranularity::RememberSettingsScores =>
                            "Only settings,scores --No replays",
                        SavefileGranularity::RememberSettingsScoresReplays =>
                            "Everything (settings,scores,replays)",
                    }
                ),
                format!(
                    "Assume enhanced-key-events work = {} *",
                    self.temp_data.kitty_assumed.fmt_on_off()
                ),
                format!(
                    "Blindfold gameplay = {}",
                    self.temp_data.blindfold_enabled.fmt_on_off()
                ),
                format!(
                    "Renderertype = {} (applies on New Game)",
                    TetroTUIRenderer::with_number(self.temp_data.renderernumber).name()
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
                            format!("Trying to load savefile on start caused this error:")
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
                        self.temp_data.blindfold_enabled = false;
                    }
                    3 => {
                        self.temp_data.renderernumber = 0;
                    }
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
                }) => match selected {
                    0 => {
                        self.temp_data.save_on_exit = match self.temp_data.save_on_exit {
                            SavefileGranularity::NoSavefile => {
                                SavefileGranularity::RememberSettings
                            }
                            SavefileGranularity::RememberSettings => {
                                SavefileGranularity::RememberSettingsScores
                            }
                            SavefileGranularity::RememberSettingsScores => {
                                SavefileGranularity::RememberSettingsScoresReplays
                            }
                            SavefileGranularity::RememberSettingsScoresReplays => {
                                SavefileGranularity::NoSavefile
                            }
                        };
                    }
                    1 => {
                        self.temp_data.kitty_assumed ^= true;
                    }
                    2 => {
                        self.temp_data.blindfold_enabled ^= true;
                    }
                    3 => {
                        self.temp_data.renderernumber += 1;
                        self.temp_data.renderernumber %= TetroTUIRenderer::NUM_VARIANTS;
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
                                SavefileGranularity::RememberSettingsScoresReplays
                            }
                            SavefileGranularity::RememberSettingsScoresReplays => {
                                SavefileGranularity::RememberSettingsScores
                            }
                            SavefileGranularity::RememberSettingsScores => {
                                SavefileGranularity::RememberSettings
                            }
                            SavefileGranularity::RememberSettings => {
                                SavefileGranularity::NoSavefile
                            }
                        };
                    }
                    1 => {
                        self.temp_data.kitty_assumed ^= true;
                    }
                    2 => {
                        self.temp_data.blindfold_enabled ^= true;
                    }
                    3 => {
                        self.temp_data.renderernumber += TetroTUIRenderer::NUM_VARIANTS - 1;
                        self.temp_data.renderernumber %= TetroTUIRenderer::NUM_VARIANTS;
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
