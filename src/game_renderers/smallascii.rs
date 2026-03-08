use crossterm::{cursor, style, terminal, QueueableCommand};

use super::*;

const SMALL_ASCII: &str = " .°:";

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct SmallAsciiRenderer {
    x_draw: usize,
    y_draw: usize,
}

impl Renderer for SmallAsciiRenderer {
    fn push_game_feedback_msgs(
        &mut self,
        _feedback_msgs: impl IntoIterator<Item = (InGameTime, Feedback)>,
    ) {
        // We do not use/display feedback_msg-related things for this renderer at this time.
    }

    fn reset_game_associated_state(&mut self) {
        // We do not store any state associated with the game at this time.
    }

    fn reset_view_diff_state(&mut self) {
        // We do not implement diff'ing for this renderer at this time.
    }

    fn set_render_offset(&mut self, x: usize, y: usize) {
        self.x_draw = x;
        self.y_draw = y;
    }

    fn render<T: Write>(
        &mut self,
        game: &Game,
        _meta_data: &GameMetaData,
        _settings: &Settings,
        _keybinds_legend: &KeybindsLegend,
        _replay_extra: Option<(InGameTime, f64)>,
        term: &mut T,
    ) -> io::Result<()> {
        let falling_tetromino_engine::State { board, .. } = game.state();

        let mut board = *board;
        if let Some(piece) = game.phase().piece() {
            for ((x, y), tile_type_id) in piece.tiles() {
                board[y][x] = Some(tile_type_id);
            }
        }

        let small_ascii = SMALL_ASCII.chars().collect::<Vec<char>>();

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
                    small_ascii[b0 + b1]
                })
                .collect::<String>()
        });

        term.queue(terminal::Clear(terminal::ClearType::All))?;
        term.queue(cursor::MoveTo(
            u16::try_from(self.x_draw).unwrap(),
            u16::try_from(self.y_draw).unwrap(),
        ))?;

        for (dy, b_line) in btxt_lines.enumerate() {
            term.queue(cursor::MoveTo(
                u16::try_from(self.x_draw).unwrap(),
                u16::try_from(self.y_draw + dy).unwrap(),
            ))?
            .queue(style::Print(format!("|{b_line}|")))?;
        }

        term.flush()?;

        Ok(())
    }
}
