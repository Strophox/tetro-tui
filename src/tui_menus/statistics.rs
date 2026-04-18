use std::io::{self, Write};

use crate::{fmt_helpers::fmt_duration, tui_menus::MenuUpdate, Application, Statistics};

impl<T: Write> Application<T> {
    pub fn run_menu_statistics(&mut self) -> io::Result<MenuUpdate> {
        let head = "¦ All-Time Game Statistics ¦";

        let Statistics {
            new_games_started,
            games_ended,
            play_time,
            pieces_locked,
            points_scored: _,
            lines_cleared,
            monos,
            duos,
            tris,
            tetras,
            spins,
            perfect_clears,
            combo_clears: _,
        } = &self.statistics;

        let body = [
            format!("New Games started: {new_games_started}"),
            format!("Games finished: {games_ended}"),
            format!("Cumulative play time: {}", fmt_duration(*play_time)),
            format!("Tetrominos locked: {pieces_locked}"),
            format!("Total lines cleared: {lines_cleared}"),
            format!("'Mono's: {monos}"),
            format!("'Duo's: {duos}"),
            format!("'Tri's: {tris}"),
            format!("'Tetra's: {tetras}"),
            format!("Spins: {spins}"),
            format!("Perfect clears: {perfect_clears}"),
        ]
        .join("\n");

        self.run_text_menu(head, &body, true, "Game Statistics menu")
    }
}
