use std::io::{self, Write};

use crate::{tui_menus::MenuUpdate, Application};

impl<T: Write> Application<T> {
    pub fn run_menu_about(&mut self) -> io::Result<MenuUpdate> {
        let head = format!(
            "~ About Tetro TUI v{} - https://github.com/Strophox/tetro-tui ~",
            crate::VERSION
        );
        let body = r"Tetro TUI started as a passion project from someone
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

        self.run_text_menu(&head, body, false, "About screen")
    }
}
