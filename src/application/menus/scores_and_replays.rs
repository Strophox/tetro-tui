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
use falling_tetromino_engine::Stat;

use crate::{
    application::{
        Application, CompressedInputHistory, GameRestorationData, Menu, MenuUpdate, ScoresEntry,
        ScoresSorting,
    },
    fmt_helpers::fmt_duration,
    game_renderers::TetroTUIRenderer,
};

impl<T: Write> Application<T> {
    #[allow(clippy::len_zero)]
    pub(in crate::application) fn run_menu_scores_and_replays(
        &mut self,
        cursor_pos: &mut usize,
        camera_pos: &mut usize,
    ) -> io::Result<MenuUpdate> {
        let mut re_sort_scoreboard = true;
        const CAMERA_SIZE: usize = 11;
        const CAMERA_MARGIN: usize = 2;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "* Scores and Replays *").bold(),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            let fmt_comparison_stat = |p: &ScoresEntry| match p.game_meta_data.comparison_stat.0 {
                Stat::TimeElapsed(_) => format!("time: {}", fmt_duration(p.time_elapsed)),
                Stat::PiecesLocked(_) => format!("pieces: {}", p.pieces_locked.iter().sum::<u32>()),
                Stat::LinesCleared(_) => format!("lines: {}", p.lineclears),
                Stat::PointsScored(_) => format!("score: {}", p.points_scored),
            };

            let sorting = self.scores_and_replays.sorting;
            let fmt_past_game = |(rank, (entry, opt_rep)): (
                usize,
                &(
                    ScoresEntry,
                    Option<GameRestorationData<CompressedInputHistory>>,
                ),
            )| {
                let lhs_annotation = match sorting {
                    ScoresSorting::Chronological => entry.game_meta_data.datetime.to_owned(),
                    ScoresSorting::Scoring => {
                        format!("{rank: >2}{}", if rank == 1 { '#' } else { '.' })
                    }
                };
                format!(
                    "{} {} | {}{}{}",
                    lhs_annotation,
                    entry.game_meta_data.title,
                    if entry.is_win { "" } else { "unf." },
                    fmt_comparison_stat(entry),
                    if opt_rep.is_some() { " | RP" } else { "" }
                )
            };

            if self.scores_and_replays.entries.is_empty() {
                self.term
                    .queue(MoveTo(x_main, y_main + y_selection + 4 + 3))?
                    .queue(PrintStyledContent(
                        format!("{:^w_main$}", "The scoreboard is empty.").italic(),
                    ))?
                    .queue(MoveTo(x_main, y_main + y_selection + 4 + 4))?
                    .queue(PrintStyledContent(
                        format!("{:^w_main$}", "If you finish a game it will show up here!")
                            .italic(),
                    ))?;
            } else if re_sort_scoreboard {
                re_sort_scoreboard = false;
                let mut h = std::hash::DefaultHasher::new();
                std::hash::Hash::hash(&self.scores_and_replays.entries[*cursor_pos], &mut h);
                let old_hash = std::hash::Hasher::finish(&h);

                match self.scores_and_replays.sorting {
                    ScoresSorting::Chronological => self.sort_past_games_chronologically(),
                    ScoresSorting::Scoring => self.sort_past_games_semantically(),
                };

                // let d_pos = cursor_pos.saturating_sub(*camera_pos);
                *cursor_pos = self
                    .scores_and_replays
                    .entries
                    .iter()
                    .enumerate()
                    .find_map(|(i, entry)| {
                        let mut h = std::hash::DefaultHasher::new();
                        std::hash::Hash::hash(entry, &mut h);
                        let new_hash = std::hash::Hasher::finish(&h);
                        old_hash.eq(&new_hash).then_some(i)
                    })
                    .unwrap_or(*cursor_pos);
                // *camera_pos = cursor_pos.saturating_sub(d_pos);
                *camera_pos = cursor_pos.saturating_sub(CAMERA_SIZE / 2).min(
                    self.scores_and_replays
                        .entries
                        .len()
                        .saturating_sub(CAMERA_SIZE),
                );
            }

            for (i, entry) in self
                .scores_and_replays
                .entries
                .iter()
                .scan((1, None), |(i, prev_title), e| {
                    if Some(&e.0.game_meta_data.title) != prev_title.as_ref() {
                        *prev_title = Some(e.0.game_meta_data.title.clone());
                        *i = 1;
                    } else {
                        *i += 1;
                    }
                    Some((*i, e))
                })
                .skip(*camera_pos)
                .take(CAMERA_SIZE)
                .map(fmt_past_game)
                .enumerate()
            {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(PrintStyledContent(if *cursor_pos == *camera_pos + i {
                        format!("{:<w_main$}", format!(">{}", entry)).bold()
                    } else {
                        format!("{:<w_main$}", format!(" {}", entry)).reset()
                    }))?;
            }

            let entries_left = self
                .scores_and_replays
                .entries
                .len()
                .saturating_sub(*camera_pos + CAMERA_SIZE);
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(CAMERA_SIZE).unwrap(),
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        if entries_left > 0 {
                            format!("... +{entries_left} more")
                        } else {
                            "".to_owned()
                        }
                    )
                    .italic(),
                ))?;
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(CAMERA_SIZE).unwrap() + 1,
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        format!("(Order = {:?} [←|→])", self.scores_and_replays.sorting)
                    )
                    .italic(),
                ))?;
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(CAMERA_SIZE).unwrap() + 2,
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        "(Controls: [↓|↑]=scroll [Del]=delete [Enter]=replay)"
                    )
                    .italic(),
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

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k' | 'K'),
                    kind: kind @ (Press | Repeat),
                    ..
                }) if self.scores_and_replays.entries.len() > 0 => {
                    // We allow wrapping cursor pos, but only on manual presses (if detectable).
                    if 0 < *cursor_pos || kind == Press {
                        // Cursor pos possibly wraps back down.
                        *cursor_pos += self.scores_and_replays.entries.len() - 1;
                        *cursor_pos %= self.scores_and_replays.entries.len();
                        // If it does, then manually reset camera to bottom of scoreboard.
                        if *cursor_pos == self.scores_and_replays.entries.len() - 1 {
                            *camera_pos = self
                                .scores_and_replays
                                .entries
                                .len()
                                .saturating_sub(CAMERA_SIZE);
                        // Otherwise cursor just moved normally, and we may have to adapt camera (unless it hit scoreboard end).
                        } else if 0 < *camera_pos && *cursor_pos < *camera_pos + CAMERA_MARGIN {
                            *camera_pos -= 1;
                        }
                    }
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j' | 'J'),
                    kind: kind @ (Press | Repeat),
                    ..
                }) if self.scores_and_replays.entries.len() > 0 => {
                    // We allow wrapping cursor pos, but only on manual presses (if detectable).
                    if *cursor_pos < self.scores_and_replays.entries.len() - 1 || kind == Press {
                        // Cursor pos possibly wraps back up.
                        *cursor_pos += 1;
                        *cursor_pos %= self.scores_and_replays.entries.len();
                        // If it does, then manually reset camera to bottom of scoreboard.
                        if *cursor_pos == 0 {
                            *camera_pos = 0;
                        // Otherwise cursor just moved normally, and we may have to adapt camera (unless it hit scoreboard end).
                        } else if *camera_pos + CAMERA_SIZE - CAMERA_MARGIN <= *cursor_pos
                            && *camera_pos
                                < self
                                    .scores_and_replays
                                    .entries
                                    .len()
                                    .saturating_sub(CAMERA_SIZE)
                        {
                            *camera_pos += 1;
                        }
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    self.scores_and_replays.sorting = match self.scores_and_replays.sorting {
                        ScoresSorting::Chronological => ScoresSorting::Scoring,
                        ScoresSorting::Scoring => ScoresSorting::Chronological,
                    };
                    re_sort_scoreboard = true;
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    self.scores_and_replays.sorting = match self.scores_and_replays.sorting {
                        ScoresSorting::Chronological => ScoresSorting::Scoring,
                        ScoresSorting::Scoring => ScoresSorting::Chronological,
                    };
                    re_sort_scoreboard = true;
                }

                // Delete entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d' | 'D'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) if self.scores_and_replays.entries.len() > 0 => {
                    if modifiers.contains(KeyModifiers::ALT) {
                        self.scores_and_replays.entries[*cursor_pos].1.take();
                    } else {
                        self.scores_and_replays.entries.remove(*cursor_pos);
                        if 0 < *cursor_pos && *cursor_pos == self.scores_and_replays.entries.len() {
                            *cursor_pos -= 1;
                            *camera_pos = camera_pos.saturating_sub(1);
                        }
                    }
                }

                // Load slot as savepoint.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e' | 'E'),
                    kind: Press | Repeat,
                    ..
                }) if self.scores_and_replays.entries.len() > 0 => {
                    if let (
                        ScoresEntry {
                            game_meta_data,
                            time_elapsed,
                            ..
                        },
                        Some(game_restoration_data),
                    ) = &self.scores_and_replays.entries[*cursor_pos]
                    {
                        let game_meta_data = game_meta_data.clone();

                        let game_restoration_data = game_restoration_data
                            .clone()
                            .map(|input_history| input_history.decompress());

                        break Ok(MenuUpdate::Push(Menu::ReplayGame {
                            game_restoration_data: Box::new(game_restoration_data),
                            game_meta_data,
                            replay_length: *time_elapsed,
                            game_renderer: TetroTUIRenderer::with_number(
                                self.temp_data.renderernumber,
                            )
                            .into(),
                        }));
                    } else {
                        // FIXME: Handle game-replay-unavailable?
                    }
                }

                // Other event: don't care.
                _ => {}
            };
        }
    }
}
