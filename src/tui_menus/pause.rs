use std::io::{self, Write};

use crate::{
    Application,
    tui_menus::{Menu, MenuUpdate},
};

impl<W: Write> Application<W> {
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
