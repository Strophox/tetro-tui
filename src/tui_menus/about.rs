use std::io::{self, Write};

use crate::{Application, tui_menus::MenuUpdate};

impl<W: Write> Application<W> {
    pub fn run_menu_about(&mut self) -> io::Result<MenuUpdate> {
        let title = format!(
            "~ About Tetro TUI v{} - https://github.com/Strophox/tetro-tui ~",
            crate::VERSION
        );
        let body = r#"
Tetro TUI started as a passion project from someone who
loves programming, minimalistic games and ASCII art;

Out of curiosity I snuck a peek to see how deep the
mechanics of such a universal game can go:
Basic versions are simple to code up, but it gets surprisingly
complex when it comes to supporting all the modern/advanced features
(especially while dealing with terminal limitations)!

To the best of my abilities I have implemented a most featureful
& customizable version that still remains faithful to the essential
idea and also looks/runs nicely within a 'mere' terminal - Enjoy!
☺ L.C.Werner"#;

        self.run_text_menu(&title, body, false, "About screen")
    }
}
