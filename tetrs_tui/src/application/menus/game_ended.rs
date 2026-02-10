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

use crate::{
    application::{Application, Menu, MenuUpdate, ScoreboardEntry},
    fmt_helpers::{fmt_duration, fmt_hertz, fmt_tetromino_counts},
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_game_ended(
        &mut self,
        past_game: &ScoreboardEntry,
    ) -> io::Result<MenuUpdate> {
        let ScoreboardEntry {
            game_meta_data,
            result,
            time_elapsed,
            lineclears,
            points_scored,
            pieces_locked,
            final_fall_delay,
            final_lock_delay,
        } = past_game;
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::Quit("quit after game ended".to_owned()),
        ];
        // if gamemode.name.as_ref().map(String::as_str) == Some("Puzzle")
        if result.is_ok() && game_meta_data.title == "Puzzle" {
            self.settings.new_game.experimental_mode_unlocked = true;
        }
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    match result {
                        Ok(_stat) => format!("++ Game Completed ({}) ++", game_meta_data.title),
                        Err(cause) =>
                            format!("-- Game Over ({}) by: {cause:?} --", game_meta_data.title),
                    }
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            let stats = [
                format!("Time elapsed: {}", fmt_duration(*time_elapsed)),
                format!("Lines: {}", lineclears),
                format!("Score: {points_scored}"),
                format!("Pieces: {}", fmt_tetromino_counts(pieces_locked)),
                format!("Gravity: {}", fmt_hertz(final_fall_delay.as_hertz())),
                format!(
                    "Lock delay: {}ms",
                    final_lock_delay.saturating_duration().as_millis()
                ),
            ];

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
                    code:
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace | KeyCode::Char('b'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e'),
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
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !selection.is_empty() {
                        selected += selection.len() - 1;
                    }
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !selection.is_empty() {
                        selected += 1;
                    }
                }
                // Other event: don't care.
                _ => {}
            }
            if !selection.is_empty() {
                selected = selected.rem_euclid(selection.len());
            }
        }
    }
}
