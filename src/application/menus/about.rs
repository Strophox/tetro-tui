use std::io::{self, Write};

use crate::application::{menus::MenuUpdate, Application};

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_about(&mut self) -> io::Result<MenuUpdate> {
        /* FIXME: Implement About section. */
        self.generic_menu(
            concat!(
                "About Tetro TUI ",
                clap::crate_version!(),
                " - https://github.com/Strophox/tetro-tui"
            ),
            vec![],
        )
    }
}
