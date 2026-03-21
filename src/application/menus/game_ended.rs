use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
    QueueableCommand,
};

use crate::{
    application::{
        menus::{Menu, MenuUpdate},
        Application, ScoresEntry,
    },
    fmt_helpers::{fmt_duration, fmt_hertz, fmt_tetromino_counts},
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_game_ended(
        &mut self,
        game_scoring: &ScoresEntry,
    ) -> io::Result<MenuUpdate> {
        let ScoresEntry {
            game_meta_data,
            end_cause,
            is_win,
            time_elapsed,
            lineclears,
            points_scored,
            pieces_locked,
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
            Menu::Quit,
        ];

        let color_tetromino_rainbow = "1643502"
            .chars()
            .map(|ch| {
                self.settings
                    .palette()
                    .get(
                        &falling_tetromino_engine::Tetromino::VARIANTS
                            [ch.to_string().parse::<usize>().unwrap()]
                        .tiletypeid()
                        .get(),
                    )
                    .unwrap_or(&Color::Reset)
            })
            .copied()
            .collect::<Vec<_>>();
        let mut timing_offset = 0usize;
        let mut coloring_width = 2;
        let animation_delay =
            std::time::Duration::from_secs_f64(1. / self.settings.graphics().game_fps);

        if *is_win
            && game_meta_data.title == "Marathon"
            && !self.settings.new_game.master_mode_unlocked
        {
            self.settings.new_game.master_mode_unlocked = true;
        } else if *is_win
            && game_meta_data.title == "Puzzle"
            && !self.settings.new_game.experimental_mode_unlocked
        {
            self.settings.new_game.experimental_mode_unlocked = true;
            // FIXME: Remove unused code or reinstate it: hacky 'notification' screen for unlocking.
            // let w_main = Self::W_MAIN.into();
            // let (x_main, y_main) = Self::fetch_main_xy();
            // let y_half = (Self::H_MAIN / 2).saturating_sub(1);
            // self.term
            //     .queue(Clear(ClearType::All))?
            //     .queue(MoveTo(x_main, y_main + y_half))?
            //     .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            // self.term
            //     .queue(MoveTo(x_main, y_main + y_half + 1))?
            //     .queue(Print(
            //         format!(
            //             "{:^w_main$}",
            //             "New experimental Mode unlocked."
            //         )
            //     ))?;
            // self.term
            //     .queue(MoveTo(x_main, y_main + y_half + 2))?
            //     .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            // self.term.flush()?;
            // // Wait.
            // event::read()?;
        }

        let mut selected = 0usize;
        let mut refresh_fully = true;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            if *is_win {
                let clear_type = if refresh_fully {
                    refresh_fully = false;
                    ClearType::All
                } else {
                    ClearType::CurrentLine
                };
                self.term
                    .queue(MoveTo(x_main, y_main + y_selection))?
                    .queue(Clear(clear_type))?;

                let line = format!(
                    "{:^w_main$}",
                    format!("++ Game Completed ({}) ++", game_meta_data.title)
                );
                for (x_offset, c) in line.chars().enumerate() {
                    let added_offsets = timing_offset + x_offset;
                    let mut rainbow_offset = added_offsets / coloring_width;
                    // Some horrible hacking to make it look smoother + dithered on higher framerates.
                    if self.settings.graphics().game_fps >= 42.0 {
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
                            color_tetromino_rainbow[rainbow_offset % color_tetromino_rainbow.len()],
                        )))?;
                }
            } else {
                self.term
                    .queue(Clear(ClearType::All))?
                    .queue(MoveTo(x_main, y_main + y_selection))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            format!("-- Game Over: {end_cause} ({}) --", game_meta_data.title)
                        )
                        .bold(),
                    ))?;
            }

            self.term
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            timing_offset = timing_offset.saturating_add(1);

            let mut stats = vec![
                format!("Time elapsed: {}", fmt_duration(*time_elapsed)),
                format!("Lines: {lineclears}"),
                format!("Score: {points_scored}"),
                format!("Pieces: {}", fmt_tetromino_counts(pieces_locked)),
                format!("Gravity: {}", fmt_hertz(fall_delay_reached.as_hertz())),
            ];

            if let Some(lock_delay_reached) = lock_delay_reached {
                stats.push(format!(
                    "Lock delay: {}ms",
                    lock_delay_reached.saturating_duration().as_millis()
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
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            let names = selection
                .iter()
                .map(|menu| menu.to_string())
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
                            format!(">> {name} <<")
                        } else {
                            name
                        }
                    )))?;
            }
            self.term.flush()?;

            if !event::poll(animation_delay)? {
                continue;
            }

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
                }) => {
                    if !selection.is_empty() {
                        let menu = selection.into_iter().nth(selected).unwrap();
                        break Ok(MenuUpdate::Push(menu));
                    }
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

                // Other event: don't care.
                _ => {}
            }
            selected = selected.rem_euclid(selection.len());
            refresh_fully = true;
        }
    }
}
