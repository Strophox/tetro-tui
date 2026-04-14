#[allow(unused)]
mod dense_terminal_double_buffer;
#[allow(unused)]
mod dense_terminal_single_buffer;
#[allow(unused)]
mod sparse_terminal_double_buffer;

use crossterm::style::Color;
use falling_tetromino_engine::{Coordinate, GameEndCause, Phase, TileID};
use rand::RngExt;

use crate::{
    fmt_helpers::fmt_lineclear_name,
    tui_settings::{
        HardDropEffect, LineClearEffect, LineClearInlineEffect, LineClearParticleEffect,
        LockEffect, Palette, TileTexture,
    },
};

use super::*;

use dense_terminal_single_buffer::DenseTerminalSingleBuffer as StandardTerminalBuffer;
// use dense_terminal_double_buffer::DenseTerminalDoubleBuffer as StandardTerminalBuffer;

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

    fn write_char(&mut self, x: u16, y: u16, cell: TermCell);
    fn write_tile(&mut self, x: u16, y: u16, tile: TileTexture, fg: Color);
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
    lifetime: InGameTime,
    origin: (usize, usize),
    momentum: (f32, f32),
    acceleration: (f32, f32),
    tile_id: TileID,
}

#[derive(PartialEq, PartialOrd, Hash, Clone, Debug)]
pub struct LineClearEffectLine {
    creation_time: InGameTime,
    line_clear_duration: InGameTime,
    y: usize,
}

#[derive(PartialEq, PartialOrd, Clone, Debug, Default)]
pub struct StandardBufferedRenderer {
    term_buf: StandardTerminalBuffer,
    text_message_buf: Vec<(InGameTime, String)>,
    hard_drop_effect_buf: Vec<(HardDropEffect, Vec<HardDropEffectTile>)>,
    lock_effect_buf: Vec<(LockEffect, Vec<LockEffectTile>)>,
    line_clear_inline_effect_buf: Vec<(LineClearInlineEffect, Vec<LineClearEffectLine>)>,
    line_clear_particle_effect_buf: Vec<(LineClearParticleEffect, Vec<LineClearEffectTile>)>,
}

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
                    lines,
                    line_clear_duration,
                } => match settings.line_clear_effect() {
                    LineClearEffect::Inline(line_clear_inline_effect) => {
                        let line_clear_effect_lines = lines
                            .into_iter()
                            .map(|(y, _line)| LineClearEffectLine {
                                creation_time: time,
                                line_clear_duration,
                                y,
                            })
                            .collect();

                        self.line_clear_inline_effect_buf
                            .push((line_clear_inline_effect.clone(), line_clear_effect_lines));
                    }

                    LineClearEffect::Particle(line_clear_particle_effect) => {
                        let mut line_clear_effect_particles = Vec::new();
                        for (y, line) in lines {
                            for (x, tile_id) in line.into_iter().enumerate() {
                                // Some random values inside [-1, 1].
                                let (rand0, rand1) = (
                                    rand::rng().random_range(-1.0..1.0),
                                    rand::rng().random_range(-1.0..1.0),
                                );
                                // `xpos` as a value inside [-1, 1] representing its horizontal position within the line.
                                let xpos = 2.0 * (x as f32) / (Game::WIDTH as f32) - 1.0;
                                let lcpe = line_clear_particle_effect;
                                let mmx = lcpe.momentum_base.0
                                    + lcpe.momentum_rand.0 * rand0
                                    + lcpe.momentum_xpos * xpos;
                                let mmy = lcpe.momentum_base.1 + lcpe.momentum_rand.1 * rand1;
                                let lifetime = line_clear_particle_effect
                                    .duration_override
                                    .unwrap_or(line_clear_duration);
                                line_clear_effect_particles.push(LineClearEffectTile {
                                    creation_time: time,
                                    lifetime,
                                    origin: (x, y),
                                    momentum: (mmx, mmy),
                                    acceleration: line_clear_particle_effect.acceleration,
                                    tile_id,
                                });
                            }
                        }

                        self.line_clear_particle_effect_buf.push((
                            line_clear_particle_effect.clone(),
                            line_clear_effect_particles,
                        ));
                    }
                },

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

    fn reset_viewport_state_with_offset_and_area(
        &mut self,
        offsets: (u16, u16),
        dimensions: (u16, u16),
    ) {
        self.term_buf
            .reset_with_offset_and_area(offsets, dimensions);
    }

    // The renderer must take care of:
    // 'General TUI':
    // * 'Board' frame.
    // * 'Hold' widget.
    // * 'Next' widgets.
    // * Stats HUD.
    // * Keybinds HUD.
    // * Goal HUD.
    // * Buttons HUD.
    // * Text message feed.
    // 'Board tiles':
    // * Locked + air tiles (board including grid).
    // * Shadow piece.
    // * Spawn (shadow) piece.
    // * Active piece (possibly slashed/crossed).
    // 'Game effects':
    // * Hard drop effect.
    // * Lock effect.
    // * Line clear effect.
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
        // Horizontal padding to the left of everything.
        const W_PAD_LEFT: u16 = 1;
        // Total *additional* width of an active game HUD (on the left);
        // In addition to the 'hold' widget which already protrudes to the left and reserves space available underneath it.
        const W_ACTIVE_HUD: u16 = 17;
        // Total width of the 'hold' widget including frame.
        const W_HOLD: u16 = 7;
        // Total width of the inside of the game field.
        // 2x because of font width.
        const W_FIELD: u16 = 2 * (Game::WIDTH as u16);
        // Total width of the board including frame.
        const W_BOARD: u16 = 1 + W_FIELD + 1;
        // Total width of the 'next' widget(s) including frame.
        const W_NEXT: u16 = 13;

        // Vertical padding atop board.
        const H_PAD_TOP: u16 = 1;
        // Total height of the inside of the game field.
        const H_FIELD: u16 = Game::LOCK_OUT_HEIGHT as u16;
        // Total height of the board including frame.
        const H_BOARD: u16 = 1 + H_FIELD + 1;
        // Vertical padding below board.
        const H_PAD_BOT: u16 = 2;

        let hud_active = settings.graphics().show_main_hud || replay_extra.is_some();
        // Additional width of the hud actually required.
        let w_addhud = if hud_active { W_ACTIVE_HUD } else { 0 };

        let (_offset, (w_viewport, h_viewport)) = self.term_buf.offset_and_area();
        // Free margin toward left of viewport.
        let w_float =
            w_viewport.saturating_sub(W_PAD_LEFT + w_addhud + W_HOLD + W_BOARD + W_NEXT) / 2;
        // Free margin toward top of viewport.
        let h_float = h_viewport.saturating_sub(H_PAD_TOP + H_BOARD + H_PAD_BOT) / 2;

        // -- 'General TUI' rendering --

        let tui_style = settings.tui_style();

        // RENDER: 'Board' frame.

        // Board frame glyphs.
        let [c_fr_tl, c_fr_t, c_fr_tr, c_fr_r, c_fr_br, c_fr_b, c_fr_bl, c_fr_l] =
            tui_style.frameglyphs;
        let w_tmp1 = w_float + W_PAD_LEFT + w_addhud + W_HOLD;
        let h_tmp1 = h_float + H_PAD_TOP;

        // Complete top edge.
        // 2x's because of font width.
        #[rustfmt::skip] self.term_buf.write_char(w_tmp1, h_tmp1, TermCell { ch: c_fr_tl, fg: Color::Reset });
        for dx in 0..W_FIELD as u16 {
            #[rustfmt::skip] self.term_buf.write_char(w_tmp1 + 1 + dx, h_tmp1, TermCell { ch: c_fr_t, fg: Color::Reset });
        }
        #[rustfmt::skip] self.term_buf.write_char(w_tmp1 + 1 + W_FIELD, h_tmp1, TermCell { ch: c_fr_tr, fg: Color::Reset });

        // Complete bottom edge.
        // 2x's because of font width.
        #[rustfmt::skip] self.term_buf.write_char(w_tmp1, h_tmp1 + 1 + H_FIELD, TermCell { ch: c_fr_bl, fg: Color::Reset });
        for dx in 0..W_FIELD {
            #[rustfmt::skip] self.term_buf.write_char(w_tmp1 + 1 + dx, h_tmp1 + 1 + H_FIELD, TermCell { ch: c_fr_b, fg: Color::Reset });
        }
        #[rustfmt::skip] self.term_buf.write_char(w_tmp1 + 1 + W_FIELD, h_tmp1 + 1 + H_FIELD, TermCell { ch: c_fr_br, fg: Color::Reset });

        // Left edge.
        for dy in 0..H_FIELD {
            #[rustfmt::skip] self.term_buf.write_char(w_tmp1, h_tmp1 + 1 + dy, TermCell { ch: c_fr_l, fg: Color::Reset });
        }

        // Right edge.
        for dy in 0..H_FIELD {
            #[rustfmt::skip] self.term_buf.write_char(w_tmp1 + 1 + 2 * Game::WIDTH as u16, h_tmp1 + 1 + dy, TermCell { ch: c_fr_r, fg: Color::Reset });
        }

        // RENDER: 'Hold' widget.

        if let Some((tet, is_swappable)) = game.state().piece_held {
            // 'Hold' frame glyphs.
            let [c_h_tb, c_h_tl, c_h_l, c_h_bl] = tui_style.holdglyphs;
            let w_tmp2 = w_float + W_PAD_LEFT + w_addhud;
            let h_tmp2 = h_float + H_PAD_TOP;

            // Complete top edge.
            #[rustfmt::skip] self.term_buf.write_char(w_tmp2, h_tmp2, TermCell { ch: c_h_tl, fg: Color::Reset });
            #[rustfmt::skip] self.term_buf.write_char(w_tmp2 + 1, h_tmp2, TermCell { ch: c_h_tb, fg: Color::Reset });
            #[rustfmt::skip] self.term_buf.write_str(w_tmp2 + 1 + 1, h_tmp2, "hold",Color::Reset);
            #[rustfmt::skip] self.term_buf.write_char(w_tmp2 + 1 + 1 + 4, h_tmp2, TermCell { ch: c_h_tb, fg: Color::Reset });
            // Left edge
            #[rustfmt::skip] self.term_buf.write_char(w_tmp2, h_tmp2 + 1, TermCell { ch: c_h_l, fg: Color::Reset });
            // Complete bottom edge.
            #[rustfmt::skip] self.term_buf.write_char(w_tmp2, h_tmp2 + 2, TermCell { ch: c_h_bl, fg: Color::Reset });
            for dx in 0..6 {
                #[rustfmt::skip] self.term_buf.write_char(w_tmp2 + 1 + dx, h_tmp2 + 2, TermCell { ch: c_h_tb, fg: Color::Reset });
            }

            // Render 'hold' piece.
            let small_tet = &settings.small_tet_style().tets[tet as usize];
            let tile_id = if is_swappable {
                tet.tile_id()
            } else {
                Palette::GRAY
            };
            let color = settings
                .palette()
                .get(&tile_id)
                .copied()
                .unwrap_or(Color::Reset);
            #[rustfmt::skip] self.term_buf.write_str(w_tmp2 + 2, h_tmp2 + 1, small_tet, color);

            // Go the extra mile to render the character 'x' if we can't hold.
            if !is_swappable {
                #[rustfmt::skip] self.term_buf.write_char(w_tmp2 + 1, h_tmp2 + 1, TermCell { ch: 'x', fg: color });
            }
        }

        // RENDER: 'Next' widgets.

        // TODO

        // RENDER: Stats HUD.

        // TODO

        // RENDER: Keybinds HUD.

        // TODO

        // RENDER: Goal HUD.

        // TODO

        // RENDER: Buttons HUD.

        // TODO

        // RENDER: Text message feed.

        // TODO

        // -- 'Board tiles' rendering --

        let mino_textures = settings.mino_textures();
        let w_tmp3 = w_float + W_PAD_LEFT + w_addhud + W_HOLD + 1;
        let h_tmp3 = h_float + H_PAD_TOP + H_FIELD;
        let ftch_col_or_rset = |tile_id: &TileID| {
            settings
                .palette()
                .get(tile_id)
                .copied()
                .unwrap_or(Color::Reset)
        };

        // RENDER: Locked + air tiles (board including grid).

        let mut y_highest_tile: isize = -1;
        for (dy, line) in game
            .state()
            .board
            .iter()
            .take(Game::LOCK_OUT_HEIGHT + 1 + (H_PAD_TOP as usize))
            .enumerate()
        {
            for (dx, tile) in line.iter().enumerate() {
                let (tile_texture, color) = if let Some(tile_id) = tile {
                    y_highest_tile = dy as isize;
                    (mino_textures.locked, ftch_col_or_rset(tile_id))
                } else {
                    // Hacky but: Do *not* draw air/grid over top board frame or above.
                    if dy >= Game::LOCK_OUT_HEIGHT {
                        continue;
                    }
                    (mino_textures.air, Color::Reset)
                };

                #[rustfmt::skip] self.term_buf.write_tile(w_tmp3 + 2 * dx as u16, h_tmp3.saturating_sub(dy as u16), tile_texture, color);
            }
        }

        // RENDER: Spawn (shadow) piece.

        if settings.graphics().show_spawn {
            // Get upcoming piece if possible.
            if let Some(next_tetromino) = game.state().piece_preview.front() {
                let spawn_piece = next_tetromino.spawn_piece();
                // Only show it if the highest tile is 4 units below us or less.
                if spawn_piece.position.1 <= y_highest_tile + 4 {
                    for ((dx, dy), tile_id) in spawn_piece.tiles() {
                        let tile_texture = mino_textures.shadow;
                        let color = ftch_col_or_rset(&tile_id);
                        #[rustfmt::skip] self.term_buf.write_tile(w_tmp3 + 2 * dx as u16, h_tmp3.saturating_sub(dy as u16), tile_texture, color);
                    }
                }
            }
        }

        match game.phase() {
            // We currently do not have any visual indicator to pass this phase.
            Phase::Spawning { spawn_time: _ } => {}

            Phase::PieceInPlay {
                piece,
                autoshift_scheduled: _,
                fall_or_lock_time: _,
                lock_cap_time: _,
                lowest_y: _,
            } => {
                // RENDER: Shadow piece.

                if settings.graphics().show_shadow {
                    let shadow_piece = piece.teleported(&game.state().board, (0, -1));
                    for ((dx, dy), tile_id) in shadow_piece.tiles() {
                        let tile_texture = mino_textures.shadow;
                        let color = ftch_col_or_rset(&tile_id);
                        #[rustfmt::skip] self.term_buf.write_tile(w_tmp3 + 2 * dx as u16, h_tmp3.saturating_sub(dy as u16), tile_texture, color);
                    }
                }

                // RENDER: Active piece.

                for ((dx, dy), tile_id) in piece.tiles() {
                    let tile_texture = mino_textures.play;
                    let color = ftch_col_or_rset(&tile_id);
                    #[rustfmt::skip] self.term_buf.write_tile(w_tmp3 + 2 * dx as u16, h_tmp3.saturating_sub(dy as u16), tile_texture, color);
                }
            }

            // We currently do not have any visual indicator to pass this phase.
            Phase::LinesClearing {
                clear_finish_time: _,
                point_bonus: _,
            } => {}

            Phase::GameEnd { cause, is_win: _ } => {
                match cause {
                    GameEndCause::LockOut { locking_piece } => {
                        // RENDER: Active piece when locked out.

                        for ((dx, dy), tile_id) in locking_piece.tiles() {
                            let tile_texture = mino_textures.crossed;
                            let color = ftch_col_or_rset(&tile_id);
                            #[rustfmt::skip] self.term_buf.write_tile(w_tmp3 + 2 * dx as u16, h_tmp3.saturating_sub(dy as u16), tile_texture, color);
                        }
                    }

                    GameEndCause::BlockOut { blocked_piece } => {
                        // RENDER: Active piece when blocked out.

                        for ((dx, dy), tile_id) in blocked_piece.tiles() {
                            let (tile_texture, color) = if let Some(blocking_tile_id) =
                                game.state().board[dy as usize][dx as usize]
                            {
                                (mino_textures.crossed, ftch_col_or_rset(&blocking_tile_id))
                            } else {
                                (mino_textures.slashed, ftch_col_or_rset(&tile_id))
                            };
                            #[rustfmt::skip] self.term_buf.write_tile(w_tmp3 + 2 * dx as u16, h_tmp3.saturating_sub(dy as u16), tile_texture, color);
                        }
                    }

                    // FIXME: No visual indicator for Top out implemented! Though currently game does not even emit this.
                    GameEndCause::TopOut { top_lines: _ } => {}

                    // We currently do not have any visual indicator to display game-end by limit hit.
                    GameEndCause::Limit(_stat) => {}

                    GameEndCause::Forfeit { piece_in_play } => {
                        if let Some(forfeit_piece) = piece_in_play {
                            // RENDER: Active piece when forfeited.

                            for ((dx, dy), tile_id) in forfeit_piece.tiles() {
                                let tile_texture = mino_textures.slashed;
                                let color = ftch_col_or_rset(&tile_id);
                                #[rustfmt::skip] self.term_buf.write_tile(w_tmp3 + 2 * dx as u16, h_tmp3.saturating_sub(dy as u16), tile_texture, color);
                            }
                        }
                    }

                    // We currently do not have any visual indicator to display game-end by custom end-cause.
                    GameEndCause::Custom(_) => todo!(),
                }
            }
        }

        // -- 'Game effects' rendering --

        // RENDER: Hard drop effect.

        // TODO

        // RENDER: Lock effect.

        // TODO

        // RENDER: Line clear effect.

        // TODO

        self.term_buf.flush(term)
    }
}
