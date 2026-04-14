#[allow(unused)]
mod dense_terminal_double_buffer;
#[allow(unused)]
mod sparse_terminal_double_buffer;

use crossterm::style::Color;
use falling_tetromino_engine::{Coordinate, TileID};

use crate::tui_settings::{
    HardDropEffect, LineClearInlineEffect, LineClearParticleEffect, LockEffect,
};

use super::*;

use dense_terminal_double_buffer::DenseTerminalDoubleBuffer as StandardTerminalBuffer;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct TermCell {
    ch: char,
    fg: Color,
}

impl TermCell {
    const EMPTY: TermCell = TermCell {
        ch: ' ',
        fg: Color::Reset,
    };
}

pub trait TerminalBuffer {
    fn with_offset_and_area(offsets: (u16, u16), dimensions: (u16, u16)) -> Self;
    fn offset_and_area(&self) -> ((u16, u16), (u16, u16));
    fn reset_with_offset_and_area(&mut self, offsets: (u16, u16), dimensions: (u16, u16));

    fn write(&mut self, x: u16, y: u16, cell: TermCell);
    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color);
    fn flush(&mut self, term: &mut impl Write) -> io::Result<()>;
}

#[derive(PartialEq, PartialOrd, Clone, Debug)]
pub struct HardDropEffectTile {
    creation_time: InGameTime,
    pos: Coordinate,
    normalized_height: f32,
    original_tile_id: TileID,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub struct LockEffectTile {
    creation_time: InGameTime,
    pos: Coordinate,
    original_tile_id: TileID,
}

#[derive(PartialEq, PartialOrd, Clone, Debug)]
pub struct LineClearEffectTile {
    creation_time: InGameTime,
    origin: (usize, usize),
    momentum: (f32, f32),
    acceleration: (f32, f32),
    tile_id: TileID,
}

#[derive(PartialEq, PartialOrd, Hash, Clone, Debug)]
pub struct LineClearEffectLine {
    creation_time: InGameTime,
    y: usize,
}

/* The renderer must take care of:
Textures:
- locked + air tiles (=board + grid)
   * possibly blindfolded
- active piece
- shadow piece
- spawn piece shadow
- next pieces (normal, small, mini)
- hold piece
- slashed/crossed piece (if game over, forfeit etc.)
Effects:
- hard drop effect
- lock effect
- line clear effect
General TUI:
- TUI frames (main, hold, next, heading lines)
- stats hud
   * time, lines, points, gravity; adaptive lock delay
   * replay speed, replay length
- keybinds
- buttons
- goal hud
- message feed
  * gather/generate msgs from Accolades, Custom, Debug, GameEnded */

#[derive(PartialEq, PartialOrd, Clone, Debug, Default)]
pub struct StandardBufferedRenderer {
    term_buf: StandardTerminalBuffer,
    message_render_buf: Vec<(InGameTime, String)>,
    hard_drop_effect_buf: Vec<(HardDropEffect, Vec<HardDropEffectTile>)>,
    lock_effect_buf: Vec<(LockEffect, Vec<LockEffectTile>)>,
    line_clear_inline_effect_buf: Vec<(LineClearInlineEffect, LineClearEffectLine)>,
    line_clear_particle_effect_buf: Vec<(LineClearParticleEffect, Vec<LineClearEffectTile>)>,
}

// TODO: implement.
impl Renderer for StandardBufferedRenderer {
    fn update_feed(
        &mut self,
        feed: impl IntoIterator<Item = (Notification, InGameTime)>,
        settings: &Settings,
    ) {
        for (notif, time) in feed {
            match notif {
                Notification::PieceLocked { piece } => {
                    let lock_effect = settings.lock_effect_picked();
                }

                Notification::LinesClearing {
                    y_coords,
                    line_clear_duration,
                } => {}

                Notification::HardDrop {
                    height_dropped,
                    dropped_piece,
                } => {}

                Notification::Accolade {
                    point_bonus,
                    lineclears,
                    combo,
                    is_spin,
                    is_perfect,
                    tetromino,
                } => {}

                Notification::GameEnded { is_win } => {}

                Notification::Debug(_) => {}

                Notification::Custom(_) => {}
            }
        }
    }

    fn reset_veffects_state(&mut self) {
        self.message_render_buf.clear();
        self.hard_drop_effect_buf.clear();
        self.lock_effect_buf.clear();
        self.line_clear_inline_effect_buf.clear();
        self.line_clear_particle_effect_buf.clear();
    }

    fn reset_viewport_with_offset_and_area(&mut self, offsets: (u16, u16), dimensions: (u16, u16)) {
        self.term_buf
            .reset_with_offset_and_area(offsets, dimensions);
    }

    fn render<T: Write>(
        &mut self,
        term: &mut T,
        game: &Game,
        meta_data: &GameMetaData,
        settings: &Settings,
        temp_data: &TemporaryAppData,
        keybinds_legend: &KeybindsLegend,
        replay_extra: Option<(InGameTime, f64)>,
    ) -> io::Result<()> {
        todo!()
    }
}
