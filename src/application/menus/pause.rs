use std::io::{self, Write};

use crate::application::{Application, Menu, MenuUpdate};

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_pause(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::ScoresAndReplays {
                cursor_pos: 0,
                camera_pos: 0,
            },
            Menu::About,
            Menu::Quit,
        ];
        self.generic_menu("Game Paused", selection)
    }
}
