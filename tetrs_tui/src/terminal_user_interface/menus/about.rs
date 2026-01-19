use std::io::{self, Write};

use crate::terminal_user_interface::{Application, MenuUpdate};

impl<T: Write> Application<T> {
    pub(in crate::terminal_user_interface) fn menu_about(&mut self) -> io::Result<MenuUpdate> {
        /* FIXME: About menu. */
        self.generic_placeholder_menu(
            "About tetrs - Visit https://github.com/Strophox/tetrs",
            vec![],
        )
    }
}
