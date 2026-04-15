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
    fmt_helpers::fmt_duration,
    tui_menus::{title_bar, Menu, MenuUpdate},
    Application, Statistics,
};

impl<T: Write> Application<T> {
    pub fn run_menu_statistics(&mut self) -> io::Result<MenuUpdate> {
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();

            let Statistics {
                new_games_started,
                games_ended,
                play_time,
                pieces_locked,
                points_scored: _,
                lines_cleared,
                monos,
                duos,
                tris,
                tetras,
                spins,
                perfect_clears,
                combo_clears: _,
            } = &self.statistics;

            let lines = [
                format!("New Games started: {new_games_started}"),
                format!("Games finished: {games_ended}"),
                format!("Cumulative play time: {}", fmt_duration(*play_time)),
                format!("Tetrominos locked: {pieces_locked}"),
                format!("Total lines cleared: {lines_cleared}"),
                format!("'Mono's: {monos}"),
                format!("'Duo's: {duos}"),
                format!("'Tri's: {tris}"),
                format!("'Tetra's: {tetras}"),
                format!("Spins: {spins}"),
                format!("Perfect clears: {perfect_clears}"),
            ]
            .into_iter();

            self.term.queue(Clear(ClearType::All))?;

            let y_selection = Self::H_MAIN / 5;

            self.term
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "¦ All-Time Player Statistics ¦").bold(),
                ))?;

            self.term
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", title_bar(&self.settings))))?;

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
