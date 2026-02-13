use std::io::{self, Write};

use crate::application::{Application, Menu, MenuUpdate};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_title(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scoreboard,
            Menu::About,
            Menu::Quit,
        ];
        self.generic_menu("", selection)
    }
}
