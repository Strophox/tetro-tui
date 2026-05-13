use crate::tetromino_engine::Board;
use crossterm::{QueueableCommand, cursor, style, terminal};

use super::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Default)]
pub struct TwoxelRenderer {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    cached_board: Board,
}

impl GameRenderer for TwoxelRenderer {
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

    fn reset_viewport_state_with_offset_and_area(
        &mut self,
        (x, y): (u16, u16),
        (w, h): (u16, u16),
    ) {
        self.x = x;
        self.y = y;
        self.w = w;
        self.h = h;
        self.cached_board = Board::default();
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
        let mut board = game.state().board.clone();

        if let Some(piece) = game.phase().piece() {
            for (x, y) in piece.coords() {
                board[y as usize].0[x as usize] = Some(piece.tetromino.into());
            }
        }

        // Simple optimization: Do not do anything if board (=our entire view state) has not changed!
        if board == self.cached_board {
            return Ok(());
        }
        self.cached_board = board;

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
            let [l0, l1] = [self.cached_board[*i0], self.cached_board[*i1]];
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
                .iter()
                .map(|j0| {
                    let b0 = if l0.0[*j0].is_some() { 1 } else { 0 };
                    let b1 = if l1.0[*j0].is_some() { 2 } else { 0 };
                    settings.small_tetromino_symbols().parts[b0 + b1]
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
