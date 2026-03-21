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
        Application, Statistics,
    },
    fmt_helpers::fmt_duration,
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_statistics(&mut self) -> io::Result<MenuUpdate> {
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();

            let Statistics {
                total_new_games_started: _,
                total_games_ended,
                total_play_time,
                total_pieces_locked,
                total_points_scored: _,
                total_lines_cleared,
                total_singles,
                total_doubles,
                total_triples,
                total_quads,
                total_spins,
                total_perfect_clears,
                total_combos: _,
            } = &self.statistics;

            let lines = [
                // format!("New Games started: {total_new_games_started}"),
                format!("Games finished: {total_games_ended}"),
                format!("Total play time: {}", fmt_duration(*total_play_time)),
                format!("Total pieces locked: {total_pieces_locked}"),
                // format!("Total points scored: {total_points_scored}"),
                format!("Total lines cleared: {total_lines_cleared}"),
                format!("Total Singles: {total_singles}"),
                format!("Total Doubles: {total_doubles}"),
                format!("Total Triples: {total_triples}"),
                format!("Total Quads: {total_quads}"),
                format!("Total Spins: {total_spins}"),
                format!("Total Perfect Clears: {total_perfect_clears}"),
            ]
            .into_iter();

            self.term.queue(Clear(ClearType::All))?;

            let y_selection = Self::H_MAIN / 5;

            self.term
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "¦ Statistics ¦").bold(),
                ))?;

            self.term
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            for (dy, line) in lines.enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(dy).unwrap(),
                    ))?
                    .queue(Print(format!("{line:^w_main$}")))?;
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

                // Other event: don't care.
                _ => {}
            }
        }
    }
}
