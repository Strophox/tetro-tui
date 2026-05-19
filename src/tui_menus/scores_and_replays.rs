use std::io::{self, Write};

use crate::core_game_engine::Stat;
use crossterm::{
    QueueableCommand,
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{Print, PrintStyledContent, Stylize},
    terminal::{self},
};

use crate::{
    Application, ScoreSummary, ScoreboardSorting,
    fmt_helpers::fmt_duration,
    game_renderers::MiscGameRenderers,
    game_restoration::{EncodedInputHistory, GameRestorationData},
    tui_menus::{
        Menu, MenuUpdate, heading_line,
        replay_game::{REPLAY_ANCHOR_INTERVAL, calculate_game_and_replay_anchors},
    },
};

impl<W: Write> Application<W> {
    #[allow(clippy::len_zero)]
    pub fn run_menu_scores_and_replays(
        &mut self,
        cursor_pos: &mut usize,
        camera_pos: &mut usize,
    ) -> io::Result<MenuUpdate> {
        let mut re_sort_scoreboard = true;
        let mut view_replay_error = String::new();
        const CAMERA_SIZE: usize = 11;
        const CAMERA_MARGIN: usize = 2;
        loop {
            self.term.queue(MoveTo(0, 0))?.queue(PrintStyledContent({
                let (w, h) = terminal::size()?;
                " ".repeat((w * h) as usize)
                    .on(self.settings.tui_coloring().bg_tui)
            }))?;
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "* Scores and Replays *")
                        .bold()
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(
                    format!("{:^w_main$}", heading_line(&self.settings))
                        .with(self.settings.tui_coloring().fg_accent)
                        .on(self.settings.tui_coloring().bg_tui),
                ))?;

            let sorting = self.scores_and_replays.sorting;
            let fmt_stat = |p: &ScoreSummary| {
                let show_stat = match sorting {
                    ScoreboardSorting::Chronological | ScoreboardSorting::ModeDependent => {
                        p.game_meta_data.objective_sort_descending.0
                    }
                    ScoreboardSorting::GameStat(stat) => stat,
                };
                match show_stat {
                    Stat::TimeElapsed(_) => fmt_duration(p.time),
                    Stat::PiecesLocked(_) => {
                        format!("{} tetrominos", p.pieces.iter().sum::<u32>())
                    }
                    Stat::LinesCleared(_) => format!("{} lines", p.lineclears),
                    Stat::PointsScored(_) => format!("{} points", p.points),
                }
                // match show_stat {
                //     Stat::TimeElapsed(_) => format!("time: {}", fmt_duration(p.time_elapsed)),
                //     Stat::PiecesLocked(_) => format!("pieces: {}", p.pieces_locked.iter().sum::<u32>()),
                //     Stat::LinesCleared(_) => format!("lines: {}", p.lineclears),
                //     Stat::PointsScored(_) => format!("points: {}", p.points_scored),
                // }
            };
            let fmt_past_game = |(rank, (entry, opt_rep)): (
                usize,
                &(
                    ScoreSummary,
                    Option<GameRestorationData<EncodedInputHistory>>,
                ),
            )| {
                let lhs_annotation = match sorting {
                    ScoreboardSorting::Chronological => entry.game_meta_data.timestamp.to_owned(),
                    ScoreboardSorting::ModeDependent | ScoreboardSorting::GameStat(_) => {
                        format!("{rank: >2}{}", if rank == 1 { '#' } else { '.' })
                    }
                };
                format!(
                    "{} {}{} | {}{}",
                    lhs_annotation,
                    if entry.is_win { "" } else { "unf." },
                    entry.game_meta_data.title,
                    fmt_stat(entry),
                    if opt_rep.is_some() { "°" } else { "" }
                )
            };

            if self.scores_and_replays.entries.is_empty() {
                self.term
                    .queue(MoveTo(x_main, y_main + y_selection + 4 + 3))?
                    .queue(PrintStyledContent(
                        format!("{:^w_main$}", "The scoreboard is empty.")
                            .italic()
                            .with(self.settings.tui_coloring().fg_tui)
                            .on(self.settings.tui_coloring().bg_tui),
                    ))?
                    .queue(MoveTo(x_main, y_main + y_selection + 4 + 4))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            "When you finish a game it will show up here!"
                        )
                        .italic()
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui),
                    ))?;
            } else if re_sort_scoreboard {
                re_sort_scoreboard = false;
                let mut h = std::hash::DefaultHasher::new();
                std::hash::Hash::hash(&self.scores_and_replays.entries[*cursor_pos], &mut h);
                let old_hash = std::hash::Hasher::finish(&h);

                self.scores_and_replays.sort();

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
                        format!(
                            "{:<w_main$}",
                            format!("{}{entry}", self.settings.tui_symbols().menu_pointers[0])
                        )
                        .bold()
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui)
                    } else {
                        format!(
                            "{:<w_main$}",
                            format!(
                                "{}{entry}",
                                " ".repeat(
                                    self.settings.tui_symbols().menu_pointers[0].chars().count()
                                )
                            )
                        )
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui)
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
                            format!("(... +{entries_left})")
                        } else {
                            "".to_owned()
                        }
                    )
                    .italic()
                    .with(self.settings.tui_coloring().fg_tui)
                    .on(self.settings.tui_coloring().bg_tui),
                ))?;
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(CAMERA_SIZE).unwrap() + 1,
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        format!("(Order = {} [←|→])", self.scores_and_replays.sorting)
                    )
                    .italic()
                    .with(self.settings.tui_coloring().fg_tui)
                    .on(self.settings.tui_coloring().bg_tui),
                ))?;
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(CAMERA_SIZE).unwrap() + 2,
                ))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "[↓/↑]=scroll [Del]=delete [Enter]=replay°")
                        .italic()
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui),
                ))?;
            if !view_replay_error.is_empty() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(CAMERA_SIZE).unwrap() + 3,
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            format!("Error loading replay: {view_replay_error}")
                        )
                        .italic()
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui),
                    ))?;
            }

            self.term.flush()?;

            // Wait for new input.
            match event::read()? {
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
                    let client_menu_name = "Scores and Replays menu";
                    let legend = vec![
                        (
                            "Normal keybinds".to_owned(),
                            [
                                ("Enter e", "View selected replay"),
                                ("Escape Backspace q", "Exit menu"),
                                ("Delete d", "Delete selected entry"),
                                ("↓/↑ j/k", "Navigate down/up"),
                                ("?", "Open Keybinds overview"),
                            ]
                            .into_iter()
                            .map(|(lhs, rhs)| (lhs.to_owned(), rhs.to_owned()))
                            .collect(),
                        ),
                        (
                            "Special keybinds".to_owned(),
                            [
                                ("Home/End", "Navigate to first/last"),
                                ("Alt+Delete Alt+d", "Delete replay of selected only"),
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

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k' | 'K'),
                    kind: kind @ (Press | Repeat),
                    ..
                }) if self.scores_and_replays.entries.len() > 0
                    // We allow wrapping cursor pos, but only on manual presses (if detectable).
                    && (0 < *cursor_pos || kind == Press) =>
                {
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

                // Move selector top.
                Event::Key(KeyEvent {
                    code: KeyCode::Home,
                    kind: Press | Repeat,
                    ..
                }) if self.scores_and_replays.entries.len() > 0 => {
                    *cursor_pos = 0;
                    *camera_pos = 0;
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j' | 'J'),
                    kind: kind @ (Press | Repeat),
                    ..
                }) if self.scores_and_replays.entries.len() > 0
                    // We allow wrapping cursor pos, but only on manual presses (if detectable).
                    && (*cursor_pos < self.scores_and_replays.entries.len() - 1 || kind == Press) =>
                {
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

                // Move selector bottom.
                Event::Key(KeyEvent {
                    code: KeyCode::End,
                    kind: Press | Repeat,
                    ..
                }) if self.scores_and_replays.entries.len() > 0 => {
                    *cursor_pos = self.scores_and_replays.entries.len() - 1;
                    *camera_pos = self
                        .scores_and_replays
                        .entries
                        .len()
                        .saturating_sub(CAMERA_SIZE);
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    self.scores_and_replays.sorting = match self.scores_and_replays.sorting {
                        ScoreboardSorting::Chronological => ScoreboardSorting::ModeDependent,
                        ScoreboardSorting::ModeDependent => {
                            ScoreboardSorting::GameStat(Stat::LinesCleared(0))
                        }
                        ScoreboardSorting::GameStat(Stat::LinesCleared(_)) => {
                            ScoreboardSorting::GameStat(Stat::PiecesLocked(0))
                        }
                        ScoreboardSorting::GameStat(Stat::PiecesLocked(_)) => {
                            ScoreboardSorting::GameStat(Stat::PointsScored(0))
                        }
                        ScoreboardSorting::GameStat(Stat::PointsScored(_)) => {
                            ScoreboardSorting::GameStat(Stat::TimeElapsed(Default::default()))
                        }
                        ScoreboardSorting::GameStat(Stat::TimeElapsed(_)) => {
                            ScoreboardSorting::Chronological
                        }
                    };
                    re_sort_scoreboard = true;
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
                }) => {
                    self.scores_and_replays.sorting = match self.scores_and_replays.sorting {
                        ScoreboardSorting::Chronological => {
                            ScoreboardSorting::GameStat(Stat::TimeElapsed(Default::default()))
                        }
                        ScoreboardSorting::ModeDependent => ScoreboardSorting::Chronological,
                        ScoreboardSorting::GameStat(Stat::LinesCleared(_)) => {
                            ScoreboardSorting::ModeDependent
                        }
                        ScoreboardSorting::GameStat(Stat::PiecesLocked(_)) => {
                            ScoreboardSorting::GameStat(Stat::LinesCleared(0))
                        }
                        ScoreboardSorting::GameStat(Stat::PointsScored(_)) => {
                            ScoreboardSorting::GameStat(Stat::PiecesLocked(0))
                        }
                        ScoreboardSorting::GameStat(Stat::TimeElapsed(_)) => {
                            ScoreboardSorting::GameStat(Stat::PointsScored(0))
                        }
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

                // Load slot as replay.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e' | 'E'),
                    kind: Press | Repeat,
                    ..
                }) if self.scores_and_replays.entries.len() > 0 => {
                    let (score_entry, opt_restoration_data) =
                        &self.scores_and_replays.entries[*cursor_pos];
                    if let Some(game_restoration_data) = opt_restoration_data {
                        match game_restoration_data.clone().try_decode() {
                            Ok(game_restoration_data) => {
                                let game_meta_data = score_entry.game_meta_data.clone();
                                let replay_length = score_entry.time;
                                let game_renderer =
                                    MiscGameRenderers::with_num(self.temp_data.renderer_used)
                                        .into();
                                let cached_game_and_replay_anchors =
                                    calculate_game_and_replay_anchors(
                                        &mut self.term,
                                        &game_restoration_data,
                                        REPLAY_ANCHOR_INTERVAL,
                                        replay_length,
                                    )?;
                                break Ok(MenuUpdate::Push(Menu::ReplayGame {
                                    game_restoration_data: Box::new(game_restoration_data),
                                    game_meta_data,
                                    replay_length,
                                    game_renderer,
                                    cached_game_and_replay_anchors: Box::new(
                                        cached_game_and_replay_anchors,
                                    ),
                                }));
                            }
                            Err(e) => view_replay_error = e,
                        }
                    } else {
                        view_replay_error =
                            format!("no data for {}", score_entry.game_meta_data.title)
                    }
                }

                // Other event: don't care.
                _ => {}
            };
        }
    }
}
