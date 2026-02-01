use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::Print,
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use tetrs_engine::Stat;

use crate::{
    application::{
        Application, GameRestorationData, Menu, MenuUpdate, ScoreboardEntry, ScoreboardSorting,
    },
    fmt_utils::fmt_duration,
};

impl<T: Write> Application<T> {
    #[allow(clippy::len_zero)]
    pub(in crate::application) fn menu_scoreboard(&mut self) -> io::Result<MenuUpdate> {
        const CAMERA_SIZE: usize = 14;
        const CAMERA_MARGIN: usize = 3;
        let mut cursor_pos = 0usize;
        let mut camera_pos = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "* Scoreboard *")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            let fmt_comparison_stat = |p: &ScoreboardEntry| match p.game_meta_data.comparison_stat.0
            {
                Stat::TimeElapsed(_) => format!("time: {}", fmt_duration(&p.time_elapsed)),
                Stat::PiecesLocked(_) => format!("pieces: {}", p.pieces_locked.iter().sum::<u32>()),
                Stat::LinesCleared(_) => format!("lines: {}", p.lines_cleared),
                Stat::GravityReached(_) => format!("gravity: {}", p.gravity_reached),
                Stat::PointsScored(_) => format!("score: {}", p.points_scored),
            };

            let fmt_past_game = |(e, _): &(ScoreboardEntry, Option<GameRestorationData>)| {
                format!(
                    "{} {} | {}{}",
                    e.game_meta_data.datetime,
                    e.game_meta_data.title,
                    fmt_comparison_stat(e),
                    if e.result.is_ok() { "" } else { " (unf.)" }
                )
            };

            match self.scoreboard.sorting {
                ScoreboardSorting::Chronological => self.sort_past_games_chronologically(),
                ScoreboardSorting::Semantic => self.sort_past_games_semantically(),
            };

            for (i, entry) in self
                .scoreboard
                .entries
                .iter()
                .skip(camera_pos)
                .take(CAMERA_SIZE)
                .map(fmt_past_game)
                .enumerate()
            {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:<w_main$}",
                        if cursor_pos == camera_pos + i {
                            format!(">{}", entry)
                        } else {
                            entry
                        }
                    )))?;
            }
            let entries_left = self
                .scoreboard
                .entries
                .len()
                .saturating_sub(camera_pos + CAMERA_SIZE);
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(CAMERA_SIZE).unwrap(),
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!(
                        "{}{}",
                        if entries_left > 0 {
                            format!("... +{entries_left} more ")
                        } else {
                            "".to_owned()
                        },
                        format!("({:?} order [←|→])", self.scoreboard.sorting)
                    )
                )))?;
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

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: kind @ (Press | Repeat),
                    ..
                }) if self.scoreboard.entries.len() > 0 => {
                    // We allow wrapping cursor pos, but only on manual presses (if detectable).
                    if 0 < cursor_pos || kind == Press {
                        // Cursor pos possibly wraps back down.
                        cursor_pos += self.scoreboard.entries.len() - 1;
                        cursor_pos %= self.scoreboard.entries.len();
                        // If it does, then manually reset camera to bottom of scoreboard.
                        if cursor_pos == self.scoreboard.entries.len() - 1 {
                            camera_pos = self.scoreboard.entries.len().saturating_sub(CAMERA_SIZE);
                        // Otherwise cursor just moved normally, and we may have to adapt camera (unless it hit scoreboard end).
                        } else if 0 < camera_pos && cursor_pos < camera_pos + CAMERA_MARGIN {
                            camera_pos -= 1;
                        }
                    }
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: kind @ (Press | Repeat),
                    ..
                }) if self.scoreboard.entries.len() > 0 => {
                    // We allow wrapping cursor pos, but only on manual presses (if detectable).
                    if cursor_pos < self.scoreboard.entries.len() - 1 || kind == Press {
                        // Cursor pos possibly wraps back up.
                        cursor_pos += 1;
                        cursor_pos %= self.scoreboard.entries.len();
                        // If it does, then manually reset camera to bottom of scoreboard.
                        if cursor_pos == 0 {
                            camera_pos = 0;
                        // Otherwise cursor just moved normally, and we may have to adapt camera (unless it hit scoreboard end).
                        } else if camera_pos + CAMERA_SIZE - CAMERA_MARGIN <= cursor_pos
                            && camera_pos
                                < self.scoreboard.entries.len().saturating_sub(CAMERA_SIZE)
                        {
                            camera_pos += 1;
                        }
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    self.scoreboard.sorting = match self.scoreboard.sorting {
                        ScoreboardSorting::Chronological => ScoreboardSorting::Semantic,
                        ScoreboardSorting::Semantic => ScoreboardSorting::Chronological,
                    };
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    self.scoreboard.sorting = match self.scoreboard.sorting {
                        ScoreboardSorting::Chronological => ScoreboardSorting::Semantic,
                        ScoreboardSorting::Semantic => ScoreboardSorting::Chronological,
                    };
                }

                // Delete entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press | Repeat,
                    ..
                }) if self.scoreboard.entries.len() > 0 => {
                    self.scoreboard.entries.remove(cursor_pos);
                    if 0 < cursor_pos && cursor_pos == self.scoreboard.entries.len() {
                        cursor_pos -= 1;
                        camera_pos = camera_pos.saturating_sub(1);
                    }
                }

                // Load slot as savepoint.
                // TODO: make this visible to frontend user.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e'),
                    kind: Press | Repeat,
                    ..
                }) if self.scoreboard.entries.len() > 0 => {
                    if let (ScoreboardEntry { game_meta_data, .. }, Some(game_restoration_data)) =
                        &self.scoreboard.entries[cursor_pos]
                    {
                        let _ = self.game_savepoint.insert((
                            game_meta_data.clone(),
                            game_restoration_data.clone(),
                            game_restoration_data.button_inputs.0.len(),
                        ));
                    }
                }

                // Other event: don't care.
                _ => {}
            };
        }
    }
}
