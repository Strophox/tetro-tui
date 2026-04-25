#[allow(unused)]
mod dense_terminal_double_buffer;
#[allow(unused)]
mod dense_terminal_single_buffer;
#[allow(unused)]
mod sparse_terminal_double_buffer;

use std::{collections::VecDeque, time::Duration};

use crossterm::style::Color;
use falling_tetromino_engine::{
    Button, Coordinate, ExtDuration, GameEndCause, LOCK_OUT_HEIGHT, Orientation, Phase, Stat,
    Tetromino, TileID, WIDTH,
};
use rand::RngExt;

use crate::{
    fmt_helpers::{fmt_duration, fmt_hertz, fmt_lineclear_name},
    tui_settings::{
        HardDropEffect, LineClearEffect, LineClearInlineEffect, LineClearParticleEffect,
        LockEffect, Palette, TileTexture,
    },
};

use super::*;

// use dense_terminal_single_buffer::DenseTerminalSingleBuffer as StandardTerminalBuffer;
use dense_terminal_double_buffer::DenseTerminalDoubleBuffer as StandardTerminalBuffer;
// use sparse_terminal_double_buffer::SparseTerminalDoubleBuffer as StandardTerminalBuffer;

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
    // fn with_offset_and_area(offsets: (u16, u16), dimensions: (u16, u16)) -> Self;
    fn offset_and_area(&self) -> ((u16, u16), (u16, u16));
    fn reset_with_offset_and_area(&mut self, offsets: (u16, u16), dimensions: (u16, u16));

    fn write_char(&mut self, x: u16, y: u16, cell: TermCell);
    fn write_tile(&mut self, x: u16, y: u16, tile: TileTexture, fg: Color);
    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color);
    fn write_str_wrapping(&mut self, x: u16, y: u16, str: &str, fg: Color);
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
    line_clear_duration: InGameTime,
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
    line: [TileID; WIDTH],
}

#[derive(PartialEq, PartialOrd, Clone, Debug, Default)]
pub struct StandardBufferedRenderer {
    // NOTE: Deriving default also means that this terminal buffers has offsets and dimensions 0.
    term_buf: StandardTerminalBuffer,
    text_message_buf: VecDeque<(InGameTime, String)>,
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
                            .map(|(y, line)| LineClearEffectLine {
                                creation_time: time,
                                line_clear_duration,
                                y,
                                line,
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
                                let xpos = 2.0 * (x as f32) / (WIDTH as f32) - 1.0;
                                let lcpe = line_clear_particle_effect;
                                let mmx = lcpe.momentum_base.0
                                    + lcpe.momentum_rand.0 * rand0
                                    + lcpe.momentum_xpos * xpos;
                                let mmy = lcpe.momentum_base.1 + lcpe.momentum_rand.1 * rand1;
                                line_clear_effect_particles.push(LineClearEffectTile {
                                    creation_time: time,
                                    line_clear_duration,
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
                    self.text_message_buf.push_front((time, tokens.join(" ")));
                }

                Notification::GameEnded { cause, is_win } => {
                    let game_end_msg = if is_win {
                        "Game Complete!".to_owned()
                    } else {
                        format!("{cause}...")
                    };

                    self.text_message_buf.push_front((time, game_end_msg));
                }

                Notification::Custom(custom_msg) => {
                    self.text_message_buf.push_front((time, custom_msg));
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
    // * Grid.
    // * Locked tiles.
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
        let (_offset, (w_viewport, h_viewport)) = self.term_buf.offset_and_area();

        /// Horizontal padding to the left of everything.
        const W_PAD_LEFT: u16 = 1;
        /// Total *additional* width of an active game HUD (on the left);
        /// In addition to the 'hold' widget which already protrudes to the left and reserves space available underneath it.
        const W_ADD_ACTIVE_HUD: u16 = 15;
        /// Total width of the 'hold' widget including frame.
        const W_HOLD: u16 = 7;
        /// Total width of the inside of the game field.
        /// 2x because of font width.
        const W_FIELD: u16 = 2 * (WIDTH as u16);
        /// Total width of the board including frame.
        const W_BOARD: u16 = 1 + W_FIELD + 1;
        /// Total width of the 'next' widget(s) including frame.
        const W_NEXT: u16 = 13;

        /// Vertical padding atop board.
        const H_PAD_TOP: u16 = 1;
        /// Total height of the inside of the game field.
        const H_FIELD: u16 = LOCK_OUT_HEIGHT as u16;
        /// Total height of the board including frame.
        const H_BOARD: u16 = 1 + H_FIELD + 1;
        /// Vertical padding below board.
        const H_PAD_BOT: u16 = 2;
        /// The effective number of units of the board that end up being (allowed to be) rendered on-screen.
        const RENDERED_FIELD_HEIGHT: usize = LOCK_OUT_HEIGHT + 1 + (H_PAD_TOP as usize);

        // NOTE: An alternative would be to e.g. always show hud in replay, and/or dynamically adjust to terminal.
        // let enough_space_for_hud = w_viewport >= W_PAD_LEFT + W_ADD_ACTIVE_HUD + W_HOLD + W_BOARD + W_NEXT;
        // let hud_active = enough_space_for_hud && (settings.graphics().show_main_hud || replay_extra.is_some());
        let hud_active = settings.graphics().show_main_hud;
        // Additional width of the hud actually required.
        let w_addhud = if hud_active { W_ADD_ACTIVE_HUD } else { 0 };

        // Free margin toward left of viewport.
        let w_float =
            w_viewport.saturating_sub(W_PAD_LEFT + w_addhud + W_HOLD + W_BOARD + W_NEXT) / 2;
        // Free margin toward top of viewport.
        let h_float = h_viewport.saturating_sub(H_PAD_TOP + H_BOARD + H_PAD_BOT) / 2;

        // -- 'General TUI' rendering --

        let tui_style = settings.tui_symbols();

        // RENDER: Stats HUD.

        if hud_active {
            // Frame glyph.
            let [c_m_tb] = tui_style.headingline;
            const W_TITLE_MARGIN: u16 = 2;
            let w_tmp_hudtl = w_float + W_PAD_LEFT; // (width temporary HUD-top-left)
            const H_TITLE_OFFSET: u16 = 3;
            let h_tmp_hudtl = h_float + H_PAD_TOP + H_TITLE_OFFSET;

            // Render game/mode title.
            #[rustfmt::skip] self.term_buf.write_char(w_tmp_hudtl, h_tmp_hudtl + 1, TermCell { ch: c_m_tb, fg: Color::Reset });
            #[rustfmt::skip] self.term_buf.write_char(w_tmp_hudtl + 1, h_tmp_hudtl + 1, TermCell { ch: c_m_tb, fg: Color::Reset });
            for (dx, opt_ch) in meta_data
                .title
                .chars()
                .map(Some)
                .chain([None, None])
                .take((w_addhud + W_HOLD).saturating_sub(W_TITLE_MARGIN) as usize)
                .enumerate()
            {
                if let Some(ch) = opt_ch {
                    #[rustfmt::skip] self.term_buf.write_char(w_tmp_hudtl + W_TITLE_MARGIN + (dx as u16), h_tmp_hudtl, TermCell { ch, fg: Color::Reset });
                }
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_hudtl + W_TITLE_MARGIN + (dx as u16), h_tmp_hudtl + 1, TermCell { ch: c_m_tb, fg: Color::Reset });
            }

            // Render stats.
            let mut stats: Vec<Option<(&str, String)>> = vec![];

            if meta_data.show_stats.contains(ShowStats::TIME) {
                stats.push(Some(("Time: ", fmt_duration(game.state().time))));
            }

            if meta_data.show_stats.contains(ShowStats::LINES) {
                stats.push(Some(("Lines: ", game.state().lineclears.to_string())));
            }

            if meta_data.show_stats.contains(ShowStats::POINTS) {
                stats.push(Some(("Points: ", game.state().points.to_string())));
            }

            if meta_data.show_stats.contains(ShowStats::PIECES) {
                stats.push(Some((
                    "Pieces: ",
                    game.state().pieces_locked.iter().sum::<u32>().to_string(),
                )));
            }

            if meta_data.show_stats.contains(ShowStats::GRAVITY) {
                stats.push(Some((
                    "Gravity: ",
                    fmt_hertz(game.state().fall_delay.as_hertz()),
                )));
            }

            if meta_data.show_stats.contains(ShowStats::LOCKDELAY) {
                stats.push(Some((
                    "Lock delay: ",
                    if let ExtDuration::Finite(lock_delay) = game.state().lock_delay {
                        format!("{}ms", lock_delay.as_millis())
                    } else {
                        "infty".to_owned()
                    },
                )));
            }

            // Show mod stats.
            for modifier in &game.modifiers {
                for stat in modifier.stats() {
                    stats.push(Some(("", stat.to_owned())));
                }
            }

            // Only show Replay stats if available.
            if let Some((replay_len, replay_speed)) = replay_extra {
                // Spacing.
                stats.push(None);

                stats.push(Some(("REPLAY ", fmt_duration(replay_len))));

                stats.push(Some(("", {
                    let (partial_glyphs, full_glyph) = &tui_style.progressbar;
                    let w_progressbar = (W_ADD_ACTIVE_HUD + W_HOLD).saturating_sub(3);
                    let progress = game.state().time.as_secs_f32() / replay_len.as_secs_f32();
                    let granularity = if partial_glyphs.is_empty() {
                        1
                    } else {
                        partial_glyphs.len()
                    };
                    let scaled =
                        (progress * (w_progressbar as f32) * (granularity as f32)).round() as usize;
                    let mut progress_bar = String::new();
                    progress_bar.push_str(&full_glyph.to_string().repeat(scaled / granularity));
                    if !scaled.is_multiple_of(granularity) {
                        progress_bar.push(partial_glyphs[scaled % granularity]);
                    }
                    progress_bar.push_str(
                        &" ".repeat(w_progressbar as usize - progress_bar.chars().count()),
                    );
                    progress_bar.push(']');
                    progress_bar
                })));

                stats.push(Some(("Replay speed: ", format!("{replay_speed:.02}x"))));
            }

            let h_stats = stats.len();

            for (dy, opt_stat) in stats.into_iter().enumerate() {
                if let Some((str_statname, str_statval)) = opt_stat {
                    #[rustfmt::skip] self.term_buf.write_str(w_tmp_hudtl + 1, h_tmp_hudtl + 2 + (dy as u16), str_statname, Color::Reset);
                    let w_statname = str_statname.len() as u16;
                    #[rustfmt::skip] self.term_buf.write_str(w_tmp_hudtl + 1 + w_statname, h_tmp_hudtl + 2 + (dy as u16), &str_statval, Color::Reset);
                }
            }

            // RENDER: Keybinds HUD.

            if settings.graphics().show_keybinds {
                // Frame glyph.
                let w_tmp_ktl = w_float + W_PAD_LEFT; // (width temporary keybinds-top-left)
                let h_tmp_ktl = (h_tmp_hudtl + 2 + (h_stats as u16) + 1)
                    .max(h_float + H_PAD_TOP + H_FIELD.saturating_sub(MAX_LEGEND_ENTRIES));

                #[rustfmt::skip] self.term_buf.write_char(w_tmp_ktl, h_tmp_ktl, TermCell { ch: c_m_tb, fg: Color::Reset });
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_ktl + 1, h_tmp_ktl, TermCell { ch: c_m_tb, fg: Color::Reset });
                #[rustfmt::skip] self.term_buf.write_str(w_tmp_ktl + 1 + 1, h_tmp_ktl, "basic keybinds", Color::Reset);
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_ktl + 1 + 1 + 14, h_tmp_ktl, TermCell { ch: c_m_tb, fg: Color::Reset });
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_ktl + 1 + 1 + 14 + 1, h_tmp_ktl, TermCell { ch: c_m_tb, fg: Color::Reset });

                const W_KEYBINDS: usize = (W_ADD_ACTIVE_HUD + W_HOLD).saturating_sub(1) as usize;
                // FIXME: Correct but kinda inefficient?
                let w_max_description = keybinds_legend
                    .iter()
                    .map(|s| s.1.chars().count())
                    .max()
                    .unwrap_or(0);
                let w_budget_icons = W_KEYBINDS - w_max_description - 1;
                let w_icons = keybinds_legend
                    .iter()
                    .map(|s| s.0.chars().count())
                    .max()
                    .unwrap_or(0)
                    .min(w_budget_icons);
                for (dy, (icons, description)) in keybinds_legend.iter().enumerate() {
                    let icons = icons.chars().take(w_icons).collect::<String>();
                    let str = format!("{icons: >w_icons$} {description}");
                    #[rustfmt::skip] self.term_buf.write_str(w_tmp_ktl + 1, h_tmp_ktl + (dy as u16) + 1, &str, Color::Reset);
                }
            }
        }

        // RENDER: Goal HUD.

        if let Some((end_condition_stat, _)) = game
            .config
            .game_limits
            .iter()
            .find(|(_stat, to_win)| *to_win)
        {
            // Produce value and text to render.
            let (str_statval, str_stattxt) = match end_condition_stat {
                Stat::TimeElapsed(t) => (
                    t.saturating_sub(game.state().time).as_secs().to_string(),
                    "seconds remain",
                ),
                Stat::PiecesLocked(p) => (
                    p.saturating_sub(game.state().pieces_locked.iter().sum::<u32>())
                        .to_string(),
                    "pieces remain",
                ),
                Stat::LinesCleared(l) => (
                    l.saturating_sub(game.state().lineclears).to_string(),
                    "lines remain",
                ),
                Stat::PointsScored(s) => (
                    s.saturating_sub(game.state().points).to_string(),
                    "points remain",
                ),
            };

            let w_tmp_gtl = w_float + W_PAD_LEFT + w_addhud + W_HOLD + W_BOARD + 2; // (width temporary goal-top-left)
            let h_tmp_tl = h_float + H_PAD_TOP + H_FIELD;
            #[rustfmt::skip] self.term_buf.write_str(w_tmp_gtl, h_tmp_tl, &str_statval, Color::Reset);
            let w_str_val = str_statval.len();
            #[rustfmt::skip] self.term_buf.write_str(w_tmp_gtl + 1 + (w_str_val as u16), h_tmp_tl, str_stattxt, Color::Reset);
        }

        // RENDER: Buttons HUD.

        // Draw button state also on replay.
        if settings.graphics().show_buttons || replay_extra.is_some() {
            let w_tmp_btntl = w_float + W_PAD_LEFT + w_addhud + W_HOLD + W_BOARD + 2; // (width temporary buttons-top-left)
            let h_tmp_btntl = h_float + H_PAD_TOP + H_FIELD + 1;

            let elements = [
                Ok('['),
                Err(Button::MoveLeft),
                Err(Button::DropSoft),
                Err(Button::MoveRight),
                // Ok(' '),
                Err(Button::RotateLeft),
                Err(Button::Rotate180),
                Err(Button::RotateRight),
                // Ok(' '),
                Err(Button::DropHard),
                // Ok(' '),
                Err(Button::HoldPiece),
                // Ok(' '),
                Err(Button::TeleLeft),
                Err(Button::TeleDown),
                Err(Button::TeleRight),
                Ok(']'),
            ];
            for (dx, elem) in elements.into_iter().enumerate() {
                let ch = elem.unwrap_or_else(|b| {
                    if game.state().active_buttons[b].is_some() {
                        settings.tui_symbols().buttons[b]
                    } else {
                        ' '
                    }
                });

                #[rustfmt::skip] self.term_buf.write_char(w_tmp_btntl + (dx as u16), h_tmp_btntl, TermCell { ch, fg: Color::Reset });
            }
        }

        // RENDER: Text message feed.

        const MESSAGE_EXPIRATION_TIME: Duration = Duration::from_secs(4);
        {
            let w_aesthetic_pad = if h_viewport > h_float + H_PAD_TOP + H_BOARD + 1 {
                1
            } else {
                0
            };
            let mut dy = 0;
            self.text_message_buf.retain(|(creation_time, message)| {
                let is_unexpired = game.state().time.saturating_sub(*creation_time) < MESSAGE_EXPIRATION_TIME;
                if is_unexpired {
                    let w_msg = message.chars().count() as u16;
                    // The message should be rendered centered around board middle.
                    let x_msg = (w_float + w_addhud + W_HOLD + (W_BOARD / 2)).saturating_sub(w_msg / 2);
                    #[rustfmt::skip] self.term_buf.write_str_wrapping(x_msg, h_float + H_PAD_TOP + H_BOARD + w_aesthetic_pad + dy, message, Color::Reset);

                    dy += 1;
                }

                is_unexpired
            });
        }
        let mino_textures = settings.mino_symbols();
        let ftch_col_or_rset = |tile_id: &TileID| {
            settings
                .palette()
                .get(tile_id)
                .copied()
                .unwrap_or(Color::Reset)
        };

        // RENDER: 'Board' frame.

        // Board frame glyphs.
        let [
            c_fr_tl,
            c_fr_t,
            c_fr_tr,
            c_fr_r,
            c_fr_br,
            c_fr_b,
            c_fr_bl,
            c_fr_l,
        ] = tui_style.boardframe;
        let w_tmp_btl = w_float + W_PAD_LEFT + w_addhud + W_HOLD; // (width temporary board-top-left)
        let h_tmp_btl = h_float + H_PAD_TOP;

        // Complete top edge.
        // 2x's because of font width.
        #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl, h_tmp_btl, TermCell { ch: c_fr_tl, fg: Color::Reset });
        #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl, h_tmp_btl + 1 + H_FIELD, TermCell { ch: c_fr_bl, fg: Color::Reset });
        for dx in 0..W_FIELD {
            #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl + 1 + dx, h_tmp_btl, TermCell { ch: c_fr_t, fg: Color::Reset });
            #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl + 1 + dx, h_tmp_btl + 1 + H_FIELD, TermCell { ch: c_fr_b, fg: Color::Reset });
        }
        #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl + 1 + W_FIELD, h_tmp_btl, TermCell { ch: c_fr_tr, fg: Color::Reset });
        #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl + 1 + W_FIELD, h_tmp_btl + 1 + H_FIELD, TermCell { ch: c_fr_br, fg: Color::Reset });

        // Left and right edges.
        for dy in 0..H_FIELD {
            #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl, h_tmp_btl + 1 + dy, TermCell { ch: c_fr_l, fg: Color::Reset });
            #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl + 1 + 2 * WIDTH as u16, h_tmp_btl + 1 + dy, TermCell { ch: c_fr_r, fg: Color::Reset });
        }

        // RENDER: 'Hold' widget.

        if let Some((tet, is_swappable)) = game.state().piece_held {
            // 'Hold' frame glyphs.
            let [c_h_tb, c_h_tl, c_h_l, c_h_bl] = tui_style.holdframe;
            let w_tmp_htl = w_float + W_PAD_LEFT + w_addhud; // (width temporary hold-top-left)
            let h_tmp_htl = h_float + H_PAD_TOP;

            // Complete top and bottom edges.
            #[rustfmt::skip] self.term_buf.write_char(w_tmp_htl, h_tmp_htl, TermCell { ch: c_h_tl, fg: Color::Reset });
            #[rustfmt::skip] self.term_buf.write_char(w_tmp_htl, h_tmp_htl + 2, TermCell { ch: c_h_bl, fg: Color::Reset });
            for dx in 0..6 {
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_htl + 1 + dx, h_tmp_htl, TermCell { ch: c_h_tb, fg: Color::Reset });
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_htl + 1 + dx, h_tmp_htl + 2, TermCell { ch: c_h_tb, fg: Color::Reset });
            }
            #[rustfmt::skip] self.term_buf.write_str(w_tmp_htl + 2, h_tmp_htl, "hold",Color::Reset);
            // Left edge
            #[rustfmt::skip] self.term_buf.write_char(w_tmp_htl, h_tmp_htl + 1, TermCell { ch: c_h_l, fg: Color::Reset });

            // Render 'hold' piece.
            let small_tet = &settings.small_tetromino_symbols().tets[tet as usize];
            let w_extra_for_o = if tet == Tetromino::O { 1 } else { 0 };

            let tile_id = if is_swappable {
                tet.tile_id()
            } else {
                Palette::GRAY
            };
            let color = ftch_col_or_rset(&tile_id);
            #[rustfmt::skip] self.term_buf.write_str(w_tmp_htl + 2 + w_extra_for_o, h_tmp_htl + 1, small_tet, color);

            // Go the extra mile to render the character 'x' if we can't hold.
            if !is_swappable {
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_htl + 1, h_tmp_htl + 1, TermCell { ch: 'x', fg: color });
            }
        }

        // RENDER: Preview widgets.

        let [c_n_tb, c_n_tr, c_n_r, c_n_jl, c_n_br, c_n_jd, c_n_ltb] = tui_style.nextframe;
        let w_tmp_ntl = w_float + W_PAD_LEFT + w_addhud + W_HOLD + W_BOARD; // (width temporary next-top-left)
        let h_tmp_ntl = h_float + H_PAD_TOP;

        let mut next_tetrominos = game.state().piece_preview.iter().copied();
        'render_preview: {
            // To begin, render normalsize previews.
            let draw_appended_normalsize_prev =
                |term_buf: &mut StandardTerminalBuffer, y_offset: u16, next_tet: Tetromino| {
                    // Top and bottom edge of first prev.
                    for dx in 0..12 {
                        #[rustfmt::skip] term_buf.write_char(w_tmp_ntl + dx, h_tmp_ntl + y_offset, TermCell { ch: c_n_ltb, fg: Color::Reset });
                        #[rustfmt::skip] term_buf.write_char(w_tmp_ntl + dx, h_tmp_ntl + y_offset + 3, TermCell { ch: c_n_tb, fg: Color::Reset });
                    }
                    // Complete right edge.
                    #[rustfmt::skip] term_buf.write_char(w_tmp_ntl + 12, h_tmp_ntl + y_offset, TermCell { ch: c_n_jl, fg: Color::Reset });
                    #[rustfmt::skip] term_buf.write_char(w_tmp_ntl + 12, h_tmp_ntl + y_offset + 1, TermCell { ch: c_n_r, fg: Color::Reset });
                    #[rustfmt::skip] term_buf.write_char(w_tmp_ntl + 12, h_tmp_ntl + y_offset + 2, TermCell { ch: c_n_r, fg: Color::Reset });
                    #[rustfmt::skip] term_buf.write_char(w_tmp_ntl + 12, h_tmp_ntl + y_offset + 3, TermCell { ch: c_n_br, fg: Color::Reset });

                    // Render preview piece.
                    let tile_texture = mino_textures.locked;
                    let color = ftch_col_or_rset(&next_tet.tile_id());
                    let w_extra_for_o = if next_tet == Tetromino::O { 2 } else { 0 };
                    for (dx, dy) in next_tet.minos(Orientation::N) {
                        #[rustfmt::skip] term_buf.write_tile(w_tmp_ntl + 2 + w_extra_for_o + 2 * (dx as u16), (h_tmp_ntl + y_offset + 2).saturating_sub(dy as u16), tile_texture, color);
                    }
                };

            let Some(first_next_tet) = next_tetrominos.next() else {
                break 'render_preview;
            };
            draw_appended_normalsize_prev(&mut self.term_buf, 0, first_next_tet);
            // Override top edge of first prev.
            for dx in 0..12 {
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_ntl + dx, h_tmp_ntl, TermCell { ch: c_n_tb, fg: Color::Reset });
            }
            #[rustfmt::skip] self.term_buf.write_char(w_tmp_ntl + 12, h_tmp_ntl, TermCell { ch: c_n_tr, fg: Color::Reset });
            #[rustfmt::skip] self.term_buf.write_str(w_tmp_ntl + 4, h_tmp_ntl, "next",Color::Reset);

            let mut idx = 1;
            let mut y_offset = 3;

            // Render remaining normalsize previews.
            while y_offset + 3 < 20
                && settings
                    .graphics()
                    .normalsize_preview_limit
                    .is_none_or(|limit| idx < limit.get())
            {
                let Some(next_tet) = next_tetrominos.next() else {
                    break 'render_preview;
                };
                draw_appended_normalsize_prev(&mut self.term_buf, y_offset, next_tet);
                idx += 1;
                y_offset += 3;
            }

            let draw_appended_small_prev =
                |term_buf: &mut StandardTerminalBuffer, y_offset: u16, next_tet: Tetromino| {
                    // Top and bottom edge of first prev.
                    for dx in 0..8 {
                        #[rustfmt::skip] term_buf.write_char(w_tmp_ntl + dx, h_tmp_ntl + y_offset, TermCell { ch: c_n_ltb, fg: Color::Reset });
                        #[rustfmt::skip] term_buf.write_char(w_tmp_ntl + dx, h_tmp_ntl + y_offset + 2, TermCell { ch: c_n_tb, fg: Color::Reset });
                    }
                    // Complete right edge.
                    #[rustfmt::skip] term_buf.write_char(w_tmp_ntl + 8, h_tmp_ntl + y_offset, TermCell { ch: c_n_jl, fg: Color::Reset });
                    #[rustfmt::skip] term_buf.write_char(w_tmp_ntl + 8, h_tmp_ntl + y_offset + 1, TermCell { ch: c_n_r, fg: Color::Reset });
                    #[rustfmt::skip] term_buf.write_char(w_tmp_ntl + 8, h_tmp_ntl + y_offset + 2, TermCell { ch: c_n_br, fg: Color::Reset });

                    // Render preview piece.
                    let small_tet = &settings.small_tetromino_symbols().tets[next_tet as usize];
                    let color = ftch_col_or_rset(&next_tet.tile_id());
                    let w_extra_for_o = if next_tet == Tetromino::O { 1 } else { 0 };
                    #[rustfmt::skip] term_buf.write_str(w_tmp_ntl + 2 + w_extra_for_o, h_tmp_ntl + y_offset + 1, small_tet, color);
                };

            // To continue, render small previews (if there's space)
            if y_offset + 2 < 20 {
                let Some(next_tet) = next_tetrominos.next() else {
                    break 'render_preview;
                };
                draw_appended_small_prev(&mut self.term_buf, y_offset, next_tet);
                // Override top right corner of first small prev.
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_ntl + 8, h_tmp_ntl + y_offset, TermCell { ch: c_n_jd, fg: Color::Reset });
                y_offset += 2;

                // Render remaining small previews.
                while y_offset + 2 < 20 {
                    let Some(next_tet) = next_tetrominos.next() else {
                        break 'render_preview;
                    };
                    draw_appended_small_prev(&mut self.term_buf, y_offset, next_tet);
                    y_offset += 2;
                }
            }

            for (x_offset, next_tet) in next_tetrominos.enumerate() {
                let mini_tet = settings.mini_tetromino_symbols().tets[next_tet as usize];
                let color = ftch_col_or_rset(&next_tet.tile_id());
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_ntl + 10 + 2 * (x_offset as u16), h_tmp_ntl + y_offset.saturating_sub(1), TermCell { ch: mini_tet, fg: color });
            }
        }

        // RENDER: Elekronika frame.
        // - This needs to happen after next/hold widgets because clean look of this 2nd frame drawn over colliding widgets takes priority.

        // Special 2nd frame rendering. Mostly relevant for Elektronika 60 style.
        if let Some([c_f2_l, c_f2_b0, c_f2_b1, c_f2_r]) = tui_style.boardframe2 {
            // Complete left edge (2).
            for dy in 0..H_FIELD + 1 {
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl.saturating_sub(1), h_tmp_btl + 1 + dy, TermCell { ch: c_f2_l, fg: Color::Reset });
            }
            // Complete right edge (2).
            for dy in 0..H_FIELD + 1 {
                #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl + W_BOARD, h_tmp_btl + 1 + dy, TermCell { ch: c_f2_r, fg: Color::Reset });
            }

            // Complete bottom edge.
            for dx in 0..W_FIELD {
                if dx.is_multiple_of(2) {
                    #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl + 1 + dx, h_tmp_btl + 1 + H_FIELD + 1, TermCell { ch: c_f2_b0, fg: Color::Reset });
                } else {
                    #[rustfmt::skip] self.term_buf.write_char(w_tmp_btl + 1 + dx, h_tmp_btl + 1 + H_FIELD + 1, TermCell { ch: c_f2_b1, fg: Color::Reset });
                }
            }
        }

        // -- 'In-Field part of Board tiles' rendering --

        let w_tmp_ftl = w_float + W_PAD_LEFT + w_addhud + W_HOLD + 1; // (width temporary field-top-left)
        let h_tmp_ftl = h_float + H_PAD_TOP + H_FIELD;

        if settings.graphics().show_grid {
            // RENDER: Grid.

            for dy in 0..LOCK_OUT_HEIGHT {
                for dx in 0..WIDTH {
                    let tile_texture = mino_textures.grid;
                    let color = Color::Reset;

                    #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * dx as u16, h_tmp_ftl.saturating_sub(dy as u16), tile_texture, color);
                }
            }
        } else {
            // RENDER: Air.
            // - This is to avoid anything that could accidentally overwrite things, e.g. mods or unexpectedly wide stats.

            for dy in 0..LOCK_OUT_HEIGHT {
                for dx in 0..WIDTH {
                    #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * dx as u16, h_tmp_ftl.saturating_sub(dy as u16), TileTexture::EMPTY, Color::Reset);
                }
            }
        }

        // RENDER: Hard drop effect.

        self.hard_drop_effect_buf.retain_mut(|(hard_drop_effect, hard_drop_effect_tiles)| {
            let HardDropEffect { duration, animation, y_decay } = hard_drop_effect;
            // Empty effect landed here somehow.
            if duration.is_zero() || animation.is_empty() {
                return false;
            }

            hard_drop_effect_tiles.retain(|hard_drop_effect_tile| {
                let HardDropEffectTile { creation_time, pos: (dx, dy), normalized_height, original_tile_id } = *hard_drop_effect_tile;

                // How much time has elapsed since creation.
                let elapsed = game.state().time.saturating_sub(creation_time);
                // How far along the effect we are shifting.
                let timeshift = elapsed.as_secs_f32() / duration.as_secs_f32();

                let factor = normalized_height * *y_decay + timeshift;

                if factor >= 1.0 {
                    return false
                }

                // render the tile
                let (tile_texture, recolor) = animation[(factor * (animation.len() - 1) as f32).round() as usize];
                let tile_id = recolor.unwrap_or(original_tile_id);
                let color = ftch_col_or_rset(&tile_id);
                #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), tile_texture, color);

                true
            });

            // Retain hard drop effect if it still has active tiles.
            !hard_drop_effect_tiles.is_empty()
        });

        if !temp_data.blindfold_game {
            // RENDER: Locked tiles.

            let mut y_highest_tile: isize = -1;
            for (dy, line) in game
                .state()
                .board
                .iter()
                .take(RENDERED_FIELD_HEIGHT)
                .enumerate()
            {
                for (dx, tile) in line.iter().enumerate() {
                    if let Some(tile_id) = tile {
                        let tile_texture = mino_textures.locked;
                        let color = settings
                            .lockedminopalette()
                            .get(tile_id)
                            .copied()
                            .unwrap_or(Color::Reset);
                        #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), tile_texture, color);

                        y_highest_tile = dy as isize;
                    }
                }
            }

            // RENDER: Spawn (shadow) piece.

            if settings.graphics().show_spawn && !game.has_ended() {
                // Get upcoming piece if possible.
                if let Some(next_tetromino) = game.state().piece_preview.front() {
                    let spawn_piece = next_tetromino.spawn_piece();
                    // Only show it if the highest tile is 4 units below us or less.
                    if spawn_piece.position.1 <= y_highest_tile + 4 {
                        for ((dx, dy), tile_id) in spawn_piece.tiles() {
                            if game.state().board[dy as usize][dx as usize].is_some() {
                                continue;
                            }
                            let tile_texture = mino_textures.shadow;
                            let color = ftch_col_or_rset(&tile_id);
                            #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), tile_texture, color);
                        }
                    }
                }
            }
        }

        match game.phase() {
            // We currently do not have any visual indicator to pass this phase.
            Phase::Spawning { spawn_time: _ } => {}

            Phase::PieceInPlay {
                piece: player_piece,
                autoshift_scheduled: _,
                fall_or_lock_time,
                lock_cap_time: _,
                lowest_y: _,
            } => {
                // RENDER: Shadow piece.

                if settings.graphics().show_shadow {
                    let shadow_piece = player_piece.teleported(&game.state().board, (0, -1));
                    for ((dx, dy), tile_id) in shadow_piece.tiles() {
                        let tile_texture = mino_textures.shadow;
                        let color = ftch_col_or_rset(&tile_id);
                        #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), tile_texture, color);
                    }
                }

                // RENDER: Active piece.

                for ((dx, dy), tile_id) in player_piece.tiles() {
                    let tile_texture = mino_textures.play;
                    let color = ftch_col_or_rset(&tile_id);
                    #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), tile_texture, color);
                }

                // RENDER: Lock delay countdown visual indicator.

                if settings.graphics().show_lockdelay {
                    // Only if piece is locking.
                    if !player_piece.is_airborne(&game.state().board) {
                        let elapsed = fall_or_lock_time
                            .saturating_sub(game.state().time)
                            .as_secs_f64();
                        let given = game.state().lock_delay.as_secs_ennf64();
                        // Only render if lock delay is nonzero
                        if !given.is_zero() && !given.is_infinite() && elapsed < given.get() {
                            let str =
                                &tui_style.timer[((tui_style.timer.len() as f64 - 1.0) * elapsed
                                    / given.get())
                                .floor() as usize];
                            let color = ftch_col_or_rset(&Palette::WHITE);
                            #[rustfmt::skip] self.term_buf.write_str((w_tmp_ftl + 2 * (player_piece.position.0 as u16)).saturating_sub(1), h_tmp_ftl.saturating_sub(player_piece.position.1 as u16).saturating_add(1), str, color);
                        }
                    }
                }
            }

            // We currently do not have any visual indicator to pass this phase.
            Phase::ClearingLines {
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
                            #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), tile_texture, color);
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
                                (mino_textures.hatched, ftch_col_or_rset(&tile_id))
                            };
                            #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), tile_texture, color);
                        }
                    }

                    // Visual indicator for Top out.
                    GameEndCause::BufferOut { overflowing_lines } => {
                        for dy in (h_tmp_ftl.saturating_sub(RENDERED_FIELD_HEIGHT as u16 - 1)..)
                            .take(overflowing_lines.len())
                        {
                            for dx in 0..WIDTH {
                                // FIXME: Remove this FIX-ME as soon as this is tested / sure it works correctly.
                                let tile_texture = mino_textures.hatched;
                                let color = Color::Reset;
                                #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy), tile_texture, color);
                            }
                        }
                    }

                    // We currently do not have any visual indicator to display game-end by limit hit.
                    GameEndCause::Limit(_stat) => {}

                    GameEndCause::Forfeit { piece_in_play } => {
                        if let Some(forfeit_piece) = piece_in_play {
                            // RENDER: Active piece when forfeited.

                            for ((dx, dy), tile_id) in forfeit_piece.tiles() {
                                let tile_texture = mino_textures.hatched;
                                let color = ftch_col_or_rset(&tile_id);
                                #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), tile_texture, color);
                            }
                        }
                    }

                    // We currently do not have any visual indicator to display game-end by custom end-cause.
                    GameEndCause::Custom(_) => {}
                }
            }
        }

        // -- 'Game effects' rendering --

        // RENDER: Lock effect.

        if !game.has_ended() {
            self.lock_effect_buf.retain_mut(|(lock_effect, lock_effect_tiles)| {
                let LockEffect { duration, animation } = lock_effect;
                // Empty effect landed here somehow.
                if duration.is_zero() || animation.is_empty() {
                    return false;
                }

                lock_effect_tiles.retain(|lock_effect_tile| {
                    let LockEffectTile { creation_time, pos: (dx, dy), original_tile_id } = *lock_effect_tile;

                    // How much time has elapsed since creation.
                    let elapsed = game.state().time.saturating_sub(creation_time);
                    // How far along the effect we are shifting.
                    let timeshift = elapsed.as_secs_f32() / duration.as_secs_f32();

                    if timeshift >= 1.0 {
                        return false
                    }

                    // render the tile
                    let (retexture, recolor) = animation[(timeshift * (animation.len() - 1) as f32).round() as usize];
                    let tile_texture = retexture.unwrap_or(mino_textures.locked);
                    let tile_id = recolor.unwrap_or(original_tile_id);
                    let color = settings
                        .lockedminopalette()
                        .get(&tile_id)
                        .copied()
                        .unwrap_or(Color::Reset);
                    #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), tile_texture, color);

                    true
                });

                // Retain hard drop effect if it still has active tiles.
                !lock_effect_tiles.is_empty()
            });
        }

        // RENDER: Inline Line clear effect.

        self.line_clear_inline_effect_buf.retain_mut(|(line_clear_inline_effect, line_clear_effect_lines)| {
            let LineClearInlineEffect { anim_indices, anim_lastidx, color_animation } = line_clear_inline_effect;
            line_clear_effect_lines.retain(|line_clear_effect_line| {
                let LineClearEffectLine { creation_time, line_clear_duration, y: dy , line } = *line_clear_effect_line;

                // How much time has elapsed since creation.
                let elapsed = game.state().time.saturating_sub(creation_time);
                // How far along the effect we are shifting.
                let timeshift = elapsed.as_secs_f32() / line_clear_duration.as_secs_f32();

                if timeshift >= 1.0 {
                    return false
                }

                // rerender the line
                for (dx, original_tile_id) in line.iter().enumerate() {
                    let tile_texture = mino_textures.locked;
                    let tile_id = if !color_animation.is_empty() {
                        color_animation[(timeshift * (color_animation.len() - 1) as f32).round() as usize].unwrap_or(*original_tile_id)
                    } else {
                        *original_tile_id
                    };
                    let color = settings
                        .lockedminopalette()
                        .get(&tile_id)
                        .copied()
                        .unwrap_or(Color::Reset);
                    #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), tile_texture, color);
                }

                // render carving progress
                let threshold = timeshift * (*anim_lastidx as f32);
                for (dx, anim_idx) in anim_indices.iter().enumerate() {
                    if (*anim_idx as f32) < threshold {
                        #[rustfmt::skip] self.term_buf.write_char(w_tmp_ftl + (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), TermCell { ch: ' ', fg: Color::Reset });
                    }
                }

                true
            });

            // Retain hard drop effect if it still has active tiles.
            !line_clear_effect_lines.is_empty()
        });

        // RENDER: Particle Line clear effect.
        // - Since it's the last, it's allowed to draw over everything, even UI at this time.

        self.line_clear_particle_effect_buf.retain_mut(|(line_clear_particle_effect, line_clear_effect_tiles)| {
            let LineClearParticleEffect { duration_override, animation, acceleration: _, momentum_base: _, momentum_rand: _, momentum_xpos: _  } = line_clear_particle_effect;

            line_clear_effect_tiles.retain(|line_clear_effect_tile| {
                let LineClearEffectTile { creation_time, line_clear_duration, origin: (dx, dy), momentum: (m_x, m_y), acceleration: (a_x, a_y), tile_id: original_tile_id  } = *line_clear_effect_tile;
                let lifetime = duration_override.unwrap_or(line_clear_duration);
                // Empty effect.
                if lifetime.is_zero() || animation.is_empty() {
                    return false;
                }

                // How much time has elapsed since creation.
                let elapsed = game.state().time.saturating_sub(creation_time);
                // How far along the effect we are shifting.
                let timeshift = elapsed.as_secs_f32() / lifetime.as_secs_f32();

                if timeshift >= 1.0 {
                    return false
                }

                // Render manually cleared out tiles at original position if we still have to.
                if elapsed < line_clear_duration {
                    // empty the tile at original position
                    #[rustfmt::skip] self.term_buf.write_tile(w_tmp_ftl + 2 * (dx as u16), h_tmp_ftl.saturating_sub(dy as u16), TileTexture::EMPTY, Color::Reset);
                }

                // render the tile
                let (retexture, recolor) = animation[(timeshift * (animation.len() - 1) as f32).round() as usize];
                let tile_texture = retexture.unwrap_or(mino_textures.locked);
                let tile_id = recolor.unwrap_or(original_tile_id);
                let color = settings
                    .lockedminopalette()
                    .get(&tile_id)
                    .copied()
                    .unwrap_or(Color::Reset);

                let t = elapsed.as_secs_f32();
                let x = (w_tmp_ftl + 2 * (dx as u16)) as f32 + m_x * t + a_x * t.powi(2) / 2.0;
                let y = (h_tmp_ftl.saturating_sub(dy as u16)) as f32 - m_y * t - a_y * t.powi(2) / 2.0;
                #[rustfmt::skip] self.term_buf.write_tile(x.round() as u16, y.round() as u16, tile_texture, color);

                true
            });

            // Retain hard drop effect if it still has active tiles.
            !line_clear_effect_tiles.is_empty()
        });

        self.term_buf.flush(term)
    }
}
