use crossterm::{cursor, style, terminal, QueueableCommand};

use super::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, Default)]
pub struct TwoxelRenderer {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

impl Renderer for TwoxelRenderer {
    fn update_feed(
        &mut self,
        _feed: impl IntoIterator<Item = (Notification, InGameTime)>,
        _settings: &Settings,
    ) {
        // We do not use/display feedback_msg-related things for this renderer at this time.
    }

    fn reset_veffects_state(&mut self) {
        // We do not store any effects state associated with the game at this time.
    }

    fn reset_viewport_with_offset_and_area(&mut self, (x, y): (u16, u16), (w, h): (u16, u16)) {
        self.x = x;
        self.y = y;
        self.w = w;
        self.h = h;
    }

    fn render<T: Write>(
        &mut self,
        term: &mut T,
        game: &Game,
        _meta_data: &GameMetaData,
        settings: &Settings,
        _temp_data: &TemporaryAppData,
        _keybinds_legend: &KeybindsLegend,
        _replay_extra: Option<(InGameTime, f64)>,
    ) -> io::Result<()> {
        let mut board = game.state().board;

        if let Some(piece) = game.phase().piece() {
            for ((x, y), tile_id) in piece.tiles() {
                board[y as usize][x as usize] = Some(tile_id);
            }
        }

        let btxt_lines = [
            [18, 19],
            [16, 17],
            [14, 15],
            [12, 13],
            [10, 11],
            [8, 9],
            [6, 7],
            [4, 5],
            [2, 3],
            [0, 1],
        ]
        .iter()
        .map(|[i0, i1]| {
            let [l0, l1] = [board[*i0], board[*i1]];
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
                .iter()
                .map(|j0| {
                    let b0 = if l0[*j0].is_some() { 1 } else { 0 };
                    let b1 = if l1[*j0].is_some() { 2 } else { 0 };
                    settings.small_tet_style().parts[b0 + b1]
                })
                .collect::<String>()
        });

        term.queue(terminal::Clear(terminal::ClearType::All))?;

        let (w_render, h_render) = (1 + 10 + 1, 10);
        let (x_render, y_render) = (
            self.x + self.w.saturating_sub(w_render) / 2,
            self.y + self.h.saturating_sub(h_render) / 2,
        );

        for (dy, b_line) in btxt_lines.enumerate() {
            term.queue(cursor::MoveTo(
                x_render,
                y_render + u16::try_from(dy).unwrap(),
            ))?
            .queue(style::Print(format!("|{b_line}|")))?;
        }

        term.flush()?;

        Ok(())
    }
}
