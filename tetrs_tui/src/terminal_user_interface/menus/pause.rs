use std::io::{self, Write};

use crate::terminal_user_interface::{Application, Menu, MenuUpdate};

impl<T: Write> Application<T> {
    pub(in crate::terminal_user_interface) fn menu_pause(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::About,
            Menu::Quit("quit from pause".to_owned()),
        ];
        self.generic_placeholder_menu("Game Paused", selection)
    }
}
