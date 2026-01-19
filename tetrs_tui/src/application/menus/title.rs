use std::io::{self, Write};

use crate::application::{Application, Menu, MenuUpdate};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_title(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::About,
            Menu::Quit("quit from title menu".to_owned()),
        ];
        self.generic_placeholder_menu("", selection)
    }
}
