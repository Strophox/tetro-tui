use std::io::{self, Write};

use crate::core_game_engine::ExtDuration;
use crossterm::{
    QueueableCommand,
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
};

use crate::{
    Application, ScoreSummary,
    fmt_helpers::{fmt_duration, fmt_hertz, fmt_tetromino_counts},
    game_mode_blueprints::GameModeBlueprint,
    game_renderers::ShowStatsHud,
    tui_menus::{Menu, MenuUpdate, heading_line},
};

impl<W: Write> Application<W> {
    pub fn run_menu_game_ended(&mut self, game_scoring: &ScoreSummary) -> io::Result<MenuUpdate> {
        let ScoreSummary {
            game_meta_data,
            end_cause,
            is_win,
            time: time_elapsed,
            lineclears,
            points: points_scored,
            pieces: pieces_locked,
            fall_delay_reached,
            lock_delay_reached,
        } = game_scoring;
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::ScoresAndReplays {
                cursor_pos: 0,
                camera_pos: 0,
            },
            Menu::Statistics,
            Menu::Quit,
        ];

        let mut timing_offset = 0usize;
        let mut coloring_width = 2;
        let animation_delay =
            std::time::Duration::from_secs_f64(self.settings.graphics().fps.get().recip());

        // Unlock modes if specific modes beaten.
        if *is_win
            && game_meta_data.title == GameModeBlueprint::TITLE_REGULAR
            && (!self.settings.game_mode_preferences.unlock_master_mode
                || !self.settings.game_mode_preferences.unlock_classic_mode)
        {
            self.settings.game_mode_preferences.unlock_master_mode = true;
            self.settings.game_mode_preferences.unlock_classic_mode = true;
        } /*FIXME: Unused code: else if *is_win
        && game_meta_data.title == GameModePreset::TITLE_PUZZLE
        && !self.settings.game_mode_preferences.unlock_experimental_mode
        {
        self.settings.game_mode_preferences.unlock_experimental_mode = true;
        }*/

        let mut selected = 0usize;
        let mut refresh_fully = true;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();
            let y_selection = Self::H_MAIN / 5;
            let clear_type = if refresh_fully {
                refresh_fully = false;
                ClearType::All
            } else {
                ClearType::CurrentLine
            };
            self.term
                .queue(MoveTo(x_main, y_main + y_selection))?
                // TODO: fix thsi printing...
                .queue(Clear(clear_type))?;

            if *is_win {
                let line = format!(
                    "{:^w_main$}",
                    format!("++ Game Completed ({}) ++", game_meta_data.title)
                );
                for (x_offset, c) in line.chars().enumerate() {
                    let added_offsets = timing_offset + x_offset;
                    let mut rainbow_offset = added_offsets / coloring_width;
                    // Some horrible hacking to make it look smoother + dithered on higher framerates.
                    if self.settings.graphics().fps.get() >= 42.0 {
                        coloring_width = 9;
                        rainbow_offset += 1;
                        let modulod_offsets = added_offsets % coloring_width;
                        if modulod_offsets == 0 {
                            rainbow_offset -= 1;
                        } else if modulod_offsets == coloring_width - 1 {
                            rainbow_offset += 1;
                        }
                    }
                    self.term
                        .queue(MoveTo(
                            x_main + u16::try_from(x_offset).unwrap(),
                            y_main + y_selection,
                        ))?
                        .queue(PrintStyledContent(c.bold().with(
                            self.settings.tile_coloring().tetromino_rainbow()[rainbow_offset % 7],
                        )))?;
                }
            } else {
                self.term.queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        format!("-- Game Over: {end_cause} ({}) --", game_meta_data.title)
                    )
                    .bold(),
                ))?;
            }

            self.term
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", heading_line(&self.settings))))?;

            timing_offset = timing_offset.saturating_add(1);

            let mut stats = vec![];
            if game_meta_data.show_stats.contains(ShowStatsHud::TIME) {
                stats.push(format!("Time elapsed: {}", fmt_duration(*time_elapsed)));
            }
            if game_meta_data.show_stats.contains(ShowStatsHud::LINES) {
                stats.push(format!("Lines cleared: {lineclears}"));
            }
            if game_meta_data.show_stats.contains(ShowStatsHud::POINTS) {
                stats.push(format!("Points scored: {points_scored}"));
            }
            if game_meta_data.show_stats.contains(ShowStatsHud::GRAVITY) {
                stats.push(format!(
                    "Gravity reached: {}",
                    fmt_hertz(fall_delay_reached.as_hertz())
                ));
            }
            if game_meta_data.show_stats.contains(ShowStatsHud::LOCKDELAY) {
                stats.push(format!(
                    "Lock delay: {}",
                    if let ExtDuration::Finite(lock_delay) = lock_delay_reached {
                        format!("{}ms", lock_delay.as_millis())
                    } else {
                        "inf".to_owned()
                    }
                ));
            }
            if game_meta_data.show_stats.contains(ShowStatsHud::PIECES) {
                stats.push(format!(
                    "Pieces locked: {}",
                    pieces_locked.iter().sum::<u32>()
                ));
                stats.push(fmt_tetromino_counts(
                    pieces_locked,
                    self.settings.mini_tetromino_symbols(),
                ));
            }

            for (i, s) in stats.iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 3 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!("{s:^w_main$}")))?;
            }

            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 3 + u16::try_from(stats.len()).unwrap(),
                ))?
                .queue(Print(format!("{:^w_main$}", heading_line(&self.settings))))?;

            let names = selection
                .iter()
                .map(|menu| menu.str_for_list_menu_selection())
                .collect::<Vec<_>>();

            for (i, name) in names.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 3 + u16::try_from(stats.len() + 2 + i).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if i == selected {
                            format!(
                                "{} {name} {}",
                                self.settings.tui_symbols().menu_pointers[0],
                                self.settings.tui_symbols().menu_pointers[1]
                            )
                        } else {
                            name.to_owned()
                        }
                    )))?;
            }
            self.term.flush()?;

            if !event::poll(animation_delay)? {
                continue;
            }

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
                    let client_menu_name = "Game Ended menu";
                    let legend = vec![
                        (
                            "Normal keybinds".to_owned(),
                            [
                                ("Enter e", "Select"),
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
                }) if !selection.is_empty() => {
                    let menu = selection.into_iter().nth(selected).unwrap();
                    break Ok(MenuUpdate::Push(menu));
                }

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k' | 'K'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += selection.len() - 1;
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

                Event::Resize(..) => {}

                // Other event: don't care.
                _ => {}
            }
            selected = selected.rem_euclid(selection.len());
            refresh_fully = true;
        }
    }
}
