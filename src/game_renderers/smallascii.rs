use crossterm::{cursor, style, terminal, QueueableCommand};

use super::*;

const SMALL_ASCII: &str = "⠀.':";

#[allow(dead_code)]
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
pub struct SmallAsciiRenderer;

impl Renderer for SmallAsciiRenderer {
    fn push_game_feedback_msgs(
        &mut self,
        _feedback_msgs: impl IntoIterator<Item = (InGameTime, Feedback)>,
    ) {
        // We do not use/display feedback_msg-related things for this renderer at this time
    }

    fn render<T: Write>(
        &mut self,
        game: &Game,
        _meta_data: &GameMetaData,
        _settings: &Settings,
        _keybinds_legend: &KeybindsLegend,
        _replay_extra: Option<(InGameTime, f64)>,
        term: &mut T,
        _rerender_entire_view: bool,
    ) -> io::Result<()> {
        let falling_tetromino_engine::State { board, .. } = game.state();

        let mut board = *board;
        if let Some(piece) = game.phase().piece() {
            for ((x, y), tile_type_id) in piece.tiles() {
                board[y][x] = Some(tile_type_id);
            }
        }

        let small_ascii = SMALL_ASCII.chars().collect::<Vec<char>>();

        term.queue(cursor::MoveTo(0, 0))?
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;

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

        for b_line in btxt_lines {
            term.queue(style::Print(format!("|{b_line}|")))?
                .queue(cursor::MoveToNextLine(1))?;
        }

        term.flush()?;

        Ok(())
    }
}
