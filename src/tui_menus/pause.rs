use std::io::{self, Write};

use crate::{
    tui_menus::{Menu, MenuUpdate},
    Application,
};

impl<T: Write> Application<T> {
    pub fn run_menu_pause(&mut self) -> io::Result<MenuUpdate> {
        let head = "Game Paused";
        let body = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::ScoresAndReplays {
                cursor_pos: 0,
                camera_pos: 0,
            },
            Menu::Statistics,
            Menu::About,
            Menu::Quit,
        ];

        self.run_liminal_menu("Pause menu", head, body)
    }
}
