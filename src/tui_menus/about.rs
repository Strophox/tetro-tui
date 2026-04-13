use std::io::{self, Write};

use crate::{tui_menus::MenuUpdate, Application};

impl<T: Write> Application<T> {
    pub fn run_menu_about(&mut self) -> io::Result<MenuUpdate> {
        /* FIXME: Implement About section. */
        self.generic_menu(
            &format!(
                "About Tetro TUI {} - https://github.com/Strophox/tetro-tui",
                crate::VERSION
            ),
            vec![],
        )
    }
}
