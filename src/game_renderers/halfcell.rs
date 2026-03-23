use crossterm::{cursor, style, terminal, QueueableCommand};

use crate::graphics_settings::Glyphset;

use super::*;

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
pub struct HalfCellRenderer {
    x_draw: usize,
    y_draw: usize,
}

impl Renderer for HalfCellRenderer {
    fn push_game_notification_feed(
        &mut self,
        _feed: impl IntoIterator<Item = (Notification, InGameTime)>,
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
            for ((x, y), tile_type_id) in piece.tiles() {
                board[y as usize][x as usize] = Some(tile_type_id);
            }
        }

        // let small_ascii = " .°:".chars().collect::<Vec<char>>();
        let (halfcell, delim_l, delim_r) = match settings.graphics().glyphset {
            Glyphset::Elektronika_60 | Glyphset::ASCII => ([' ', '.', '°', ':'], '#', '#'),
            Glyphset::Unicode => ([' ', '▄', '▀', '█'], '░', '░'),
        };

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
                    halfcell[b0 + b1]
                })
                .collect::<String>()
        });

        term.queue(terminal::Clear(terminal::ClearType::All))?;

        let (w_term, h_term) = terminal::size()?;
        let (w_render, h_render) = (1 + 10 + 1, 10);

        let (x_render, y_render) = (
            w_term.saturating_sub(w_render) / 2,
            h_term.saturating_sub(h_render) / 2,
        );

        for (dy, b_line) in btxt_lines.enumerate() {
            term.queue(cursor::MoveTo(
                x_render,
                y_render + u16::try_from(dy).unwrap(),
            ))?
            .queue(style::Print(format!("{delim_l}{b_line}{delim_r}")))?;
        }

        term.flush()?;

        Ok(())
    }
}
