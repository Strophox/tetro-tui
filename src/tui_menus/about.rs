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
    tui_menus::{title_bar, Menu, MenuUpdate},
    Application,
};

const ABOUT_ME_TEXT: &str = r"Tetro TUI started as a passion project from someone
who loves programming, minimalistic games and ASCII art.

Out of curiosity I looked into the depths of this common type
of Tetromino game: Basic versions are simple to code up,
but it gets surprisingly nontrivial when it comes to
comprehensively supporting of modern/advanced features
and slews of QOL mechanics while dealing with terminal limitations!

I've given it my best effort to implement a most featureful
and customizable version that not only remains totally faithful
to the basic idea of the game, but also runs
and looks nice within the confines of a mere terminal - Enjoy!
☻ L. Werner";

impl<T: Write> Application<T> {
    pub fn run_menu_about(&mut self) -> io::Result<MenuUpdate> {
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();

            let lines = ABOUT_ME_TEXT.lines();

            self.term.queue(Clear(ClearType::All))?;

            let y_selection = Self::H_MAIN / 5;

            self.term
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        format!(
                            "~ About Tetro TUI v{} - https://github.com/Strophox/tetro-tui ~",
                            crate::VERSION
                        )
                    )
                    .bold(),
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
