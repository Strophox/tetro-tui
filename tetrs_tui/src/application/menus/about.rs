use std::io::{self, Write};

use crate::application::{Application, MenuUpdate};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_about(&mut self) -> io::Result<MenuUpdate> {
        /* FIXME: About menu. */
        self.generic_placeholder_menu(
            "About tetrs - Visit https://github.com/Strophox/tetrs",
            vec![],
        )
    }
}
