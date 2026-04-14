#[allow(unused)]
mod dense_terminal_double_buffer;
#[allow(unused)]
mod sparse_terminal_double_buffer;

use crossterm::style::Color;
use falling_tetromino_engine::{Coordinate, Phase, TileID};

use crate::{
    fmt_helpers::fmt_lineclear_name,
    tui_settings::{HardDropEffect, LineClearInlineEffect, LineClearParticleEffect, LockEffect},
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
    text_message_buf: Vec<(InGameTime, String)>,
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
        // Convert all the incoming notification into renderer details (effects, user text messages on screen etc.)
        for (notif, time) in feed {
            match notif {
                Notification::HardDrop {
                    height_dropped,
                    dropped_piece,
                } => {
                    // Don't even start generating a hard drop effect in degenerate cases.
                    if height_dropped == 0
                        || settings.hard_drop_effect().animation.is_empty()
                        || settings.hard_drop_effect().duration.is_zero()
                    {
                        continue;
                    }
                    let mut hard_drop_effect_tiles = Vec::new();

                    // Iterate through tiles starting top right, downwards then leftwards.
                    let mut current_x = None;
                    for ((x, y), tile_id) in dropped_piece.tiles().into_iter().rev() {
                        if Some(x) == current_x {
                            // Skip duplicate tile in same column.
                            continue;
                        }
                        // (Topmost) tile from new column.
                        current_x = Some(x);
                        for dy in 1..=height_dropped {
                            hard_drop_effect_tiles.push(HardDropEffectTile {
                                creation_time: time,
                                pos: (x, y + (dy as isize)),
                                normalized_height: (dy as f32) / (height_dropped as f32),
                                original_tile_id: tile_id,
                            });
                        }
                    }

                    self.hard_drop_effect_buf
                        .push((settings.hard_drop_effect().clone(), hard_drop_effect_tiles));
                }

                Notification::PieceLocked { piece } => {
                    // Don't even start generating a lock effect in degenerate cases.
                    if settings.lock_effect().animation.is_empty()
                        || settings.lock_effect().duration.is_zero()
                    {
                        continue;
                    }
                    let mut lock_effect_tiles = Vec::new();

                    for (pos, tile_id) in piece.tiles() {
                        lock_effect_tiles.push(LockEffectTile {
                            creation_time: time,
                            pos,
                            original_tile_id: tile_id,
                        });
                    }

                    self.lock_effect_buf
                        .push((settings.lock_effect().clone(), lock_effect_tiles));
                }

                Notification::LinesClearing {
                    y_coords,
                    line_clear_duration,
                } => {}

                Notification::Accolade {
                    point_bonus,
                    lineclears,
                    combo,
                    is_spin,
                    is_perfect,
                    tetromino,
                } => {
                    let mut tokens = Vec::new();
                    tokens.push(format!("+{point_bonus},"));
                    if is_perfect {
                        tokens.push("Perfect".to_owned());
                    }
                    tokens.push(fmt_lineclear_name(lineclears).to_string());
                    if is_spin {
                        tokens.push(format!("{tetromino:?}-spin"));
                    }
                    if combo > 1 {
                        tokens.push(format!("x{combo}"));
                    }
                    self.text_message_buf.push((time, tokens.join(" ")));
                }

                Notification::GameEnded { cause, is_win } => {
                    let game_end_msg = if is_win {
                        "Game Complete!".to_owned()
                    } else {
                        format!("{cause}...")
                    };

                    self.text_message_buf.push((time, game_end_msg));
                }

                Notification::Debug(debug_msg) => {
                    self.text_message_buf.push((time, debug_msg));
                }

                Notification::Custom(custom_msg) => {
                    self.text_message_buf.push((time, custom_msg));
                }
            }
        }
    }

    fn reset_veffects_state(&mut self) {
        self.text_message_buf.clear();
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
