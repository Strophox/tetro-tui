use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
    QueueableCommand,
};

use crate::application::{Application, Menu, MenuUpdate};

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_title(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::ScoresAndReplays,
            Menu::About,
            Menu::Quit,
        ];
        let mut selected = 0usize;
        loop {
            let w_main: usize = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;

            let title = [
                "▄▄▄▄▄▄▄  ▄▄▄▄ ▄▄▄▄▄▄▄  ▄▄▄▄    ▄▄▄▄ ",
                "   ▄▀   █▄▄      ▄▀   ▄█▄▄▀  ▄█   ▄█",
                "  █▀   █▄▄▄▄▄▄  █▀   █▀  ▀█  ▀▄▄▄▄▀ ",
            ];
            let title_colors = [
                "1111555  1111 1111555  5666    1111 ",
                "   35   666      35   35526  33   33",
                "  33   6661111  33   33  22  311113 ",
            ];

            self.term.queue(Clear(ClearType::All))?;

            let dx_title = w_main.saturating_sub(36) / 2;

            for (dy, (bline, cline)) in title.iter().zip(title_colors).enumerate() {
                for (dx, (bchar, cchar)) in bline.chars().zip(cline.chars()).enumerate() {
                    self.term.queue(MoveTo(
                        x_main + u16::try_from(dx_title + dx).unwrap(),
                        y_main + y_selection + u16::try_from(dy).unwrap(),
                    ))?;

                    self.term.queue(PrintStyledContent(bchar.to_string().with(
                        if cchar == ' ' {
                            Color::Reset
                        } else {
                            *self
                                .settings
                                .palette()
                                .get(
                                    &falling_tetromino_engine::Tetromino::VARIANTS
                                        [cchar.to_string().parse::<usize>().unwrap()]
                                    .tiletypeid()
                                    .get(),
                                )
                                .unwrap_or(&Color::Reset)
                        },
                    )))?;
                }
            }

            let names = selection
                .iter()
                .map(|menu| menu.to_string())
                .collect::<Vec<_>>();
            let n_names = names.len();
            for (i, name) in names.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 5 + u16::try_from(i).unwrap(),
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
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 5 + u16::try_from(n_names).unwrap() + 2,
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        "(Controls: [←|↓|↑|→] [Esc|Enter|Del] / hjklqed)",
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
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    state: _,
                }) => break Ok(MenuUpdate::Push(Menu::Quit)),
                Event::Key(KeyEvent {
                    code:
                        KeyCode::Esc
                        | KeyCode::Char('q' | 'Q')
                        | KeyCode::Backspace
                        | KeyCode::Char('b' | 'B'),
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    selected = 4;
                }
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e' | 'E'),
                    kind: KeyEventKind::Press,
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
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) => {
                    if !selection.is_empty() {
                        selected += selection.len() - 1;
                    }
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j' | 'J'),
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
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
