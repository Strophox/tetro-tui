use std::{
    cmp::Ordering,
    fmt::{Debug, Display},
    num::NonZeroU8,
    time::Duration,
};

use crossterm::{
    cursor,
    style::{self, Color, Print, PrintStyledContent, Stylize},
    terminal, QueueableCommand,
};

use falling_tetromino_engine::{
    Button, Coord, GameEndCause, InGameTime, Orientation, Phase, Stat, Tetromino, TileID,
};
use rand::RngExt;

use super::*;

use crate::{
    application::TemporaryAppData,
    fmt_helpers::{fmt_button, fmt_button_ascii, fmt_duration, fmt_hertz, FmtTetromino},
    graphics_settings::Glyphset,
};

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
struct TerminalScreenBuffer {
    prev: Vec<Vec<(char, Option<Color>)>>,
    next: Vec<Vec<(char, Option<Color>)>>,
    x_draw: usize,
    y_draw: usize,
}

impl TerminalScreenBuffer {
    fn set_render_offset(&mut self, x: usize, y: usize) {
        (self.x_draw, self.y_draw) = (x, y);
    }

    fn buffer_reset(&mut self) {
        self.prev.clear();
    }

    fn buffer_from<'a>(&mut self, base_screen: impl IntoIterator<Item = &'a str>) {
        self.next = base_screen
            .into_iter()
            .map(|str: &str| str.chars().zip(std::iter::repeat(None)).collect())
            .collect();
    }

    fn buffer_str(&mut self, str: &str, fg_color: Option<Color>, (x, y): (usize, usize)) {
        for (x_c, c) in str.chars().enumerate() {
            // Lazy: just fill up until desired starting row and column exist.
            while y >= self.next.len() {
                self.next.push(Vec::new());
            }
            let row = &mut self.next[y];
            while x + x_c >= row.len() {
                row.push((' ', None));
            }
            row[x + x_c] = (c, fg_color);
        }
    }

    fn put(&self, term: &mut impl Write, c: char, x: usize, y: usize) -> io::Result<()> {
        term.queue(cursor::MoveTo(
            u16::try_from(self.x_draw + x).unwrap(),
            u16::try_from(self.y_draw + y).unwrap(),
        ))?
        .queue(Print(c))?;
        Ok(())
    }

    fn put_styled<D: Display>(
        &self,
        term: &mut impl Write,
        content: style::StyledContent<D>,
        x: usize,
        y: usize,
    ) -> io::Result<()> {
        term.queue(cursor::MoveTo(
            u16::try_from(self.x_draw + x).unwrap(),
            u16::try_from(self.y_draw + y).unwrap(),
        ))?
        .queue(PrintStyledContent(content))?;
        Ok(())
    }

    fn flush(&mut self, term: &mut impl Write) -> io::Result<()> {
        // Begin frame update.
        term.queue(terminal::BeginSynchronizedUpdate)?;
        if self.prev.is_empty() {
            // Redraw entire screen.
            term.queue(terminal::Clear(terminal::ClearType::All))?;
            for (y, line) in self.next.iter().enumerate() {
                for (x, (c, col)) in line.iter().enumerate() {
                    if let Some(col) = col {
                        self.put_styled(term, c.with(*col), x, y)?;
                    } else {
                        self.put(term, *c, x, y)?;
                    }
                }
            }
        } else {
            // Compare next to previous frames and only write differences.
            for (y, (line_prev, line_next)) in self.prev.iter().zip(self.next.iter()).enumerate() {
                // Overwrite common line characters.
                for (x, (cell_prev @ (_c_prev, col_prev), cell_next @ (c_next, col_next))) in
                    line_prev.iter().zip(line_next.iter()).enumerate()
                {
                    // Relevant change occurred.
                    if cell_prev != cell_next {
                        // New color.
                        if let Some(col) = col_next {
                            self.put_styled(term, c_next.with(*col), x, y)?;
                        // Previously colored but not anymore, explicit reset.
                        } else if col_prev.is_some() && col_next.is_none() {
                            self.put_styled(term, c_next.reset(), x, y)?;
                        // Uncolored before and after, simple reprint.
                        } else {
                            self.put(term, *c_next, x, y)?;
                        }
                    }
                }
                // Handle differences in line length.
                match line_prev.len().cmp(&line_next.len()) {
                    // Previously shorter, just write out new characters now.
                    Ordering::Less => {
                        for (x, (c_next, col_next)) in
                            line_next.iter().enumerate().skip(line_prev.len())
                        {
                            // Write new colored char.
                            if let Some(col) = col_next {
                                self.put_styled(term, c_next.with(*col), x, y)?;
                            // Write new uncolored char.
                            } else {
                                self.put(term, *c_next, x, y)?;
                            }
                        }
                    }
                    Ordering::Equal => {}
                    // Previously longer, delete new characters.
                    Ordering::Greater => {
                        for (x, (_c_prev, col_prev)) in
                            line_prev.iter().enumerate().skip(line_next.len())
                        {
                            // Previously colored but now erased, explicit reset.
                            if col_prev.is_some() {
                                self.put_styled(term, ' '.reset(), x, y)?;
                            // Otherwise simply erase previous character.
                            } else {
                                self.put(term, ' ', x, y)?;
                            }
                        }
                    }
                }
            }
            // Handle differences in text height.
            match self.prev.len().cmp(&self.next.len()) {
                // Previously shorter in height.
                Ordering::Less => {
                    for (y, next_line) in self.next.iter().enumerate().skip(self.prev.len()) {
                        // Write entire line.
                        for (x, (c_next, col_next)) in next_line.iter().enumerate() {
                            // Write new colored char.
                            if let Some(col) = col_next {
                                self.put_styled(term, c_next.with(*col), x, y)?;
                            // Write new uncolored char.
                            } else {
                                self.put(term, *c_next, x, y)?;
                            }
                        }
                    }
                }
                Ordering::Equal => {}
                // Previously taller, delete excess lines.
                Ordering::Greater => {
                    for (y, prev_line) in self.prev.iter().enumerate().skip(self.next.len()) {
                        // Erase entire line.
                        for (x, (_c_prev, col_prev)) in prev_line.iter().enumerate() {
                            // Previously colored but now erased, explicit reset.
                            if col_prev.is_some() {
                                self.put_styled(term, ' '.reset(), x, y)?;
                            // Otherwise simply erase previous character.
                            } else {
                                self.put(term, ' ', x, y)?;
                            }
                        }
                    }
                }
            }
        }
        // End frame update and flush.
        term.queue(cursor::MoveTo(0, 0))?;
        term.queue(terminal::EndSynchronizedUpdate)?;
        term.flush()?;
        // Clear old.
        self.prev.clear();
        // Swap buffers.
        std::mem::swap(&mut self.prev, &mut self.next);
        Ok(())
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
struct HardDropTile {
    creation_time: InGameTime,
    pos: Coord,
    y_offset: usize,
    tile_id: TileID,
}

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
struct MinoParticle {
    creation_time: InGameTime,
    origin: (usize, usize),
    momentum: (f32, f32),
    acceleration: (f32, f32),
    actually_render: bool,
    tile_id: TileID,
}

#[derive(PartialEq, PartialOrd, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DiffPrintRenderer {
    screen: TerminalScreenBuffer,
    notification_feed_buffer: Vec<(Notification, InGameTime, bool)>,
    buffered_text_msgs: Vec<(InGameTime, String)>,
    hard_drop_tiles: Vec<(HardDropTile, bool)>,
    mino_particles: Vec<(MinoParticle, bool)>,
}

impl Renderer for DiffPrintRenderer {
    fn push_game_notification_feed(
        &mut self,
        feed: impl IntoIterator<Item = (Notification, InGameTime)>,
    ) {
        // Update stored events.
        self.notification_feed_buffer
            .extend(feed.into_iter().map(|(notif, time)| (notif, time, true)));
    }

    fn reset_game_associated_state(&mut self) {
        self.notification_feed_buffer.clear();
        self.buffered_text_msgs.clear();
        self.hard_drop_tiles.clear();
        self.mino_particles.clear();
    }

    fn reset_view_diff_state(&mut self) {
        self.screen.buffer_reset();
    }

    fn set_render_offset(&mut self, x: usize, y: usize) {
        self.screen.set_render_offset(x, y);
    }

    fn render<T>(
        &mut self,
        term: &mut T,
        game: &Game,
        meta_data: &GameMetaData,
        settings: &Settings,
        temp_data: &TemporaryAppData,
        keybinds_legend: &KeybindsLegend,
        replay_extra: Option<(InGameTime, f64)>,
    ) -> io::Result<()>
    where
        T: Write,
    {
        let pieces = game.state().pieces_locked.iter().sum::<u32>();
        let gravity = game.state().fall_delay.as_hertz();
        // Screen: some titles.
        let modename_len = meta_data.title.len().max(14);
        // FIXME: Only displaying the first (of up to four) limits found.
        let (endcond_title, endcond_value) = if let Some((c, _)) = game
            .config
            .game_limits
            .iter()
            .find(|(_stat, to_win)| *to_win)
        {
            match c {
                Stat::TimeElapsed(t) => (
                    "Time left:",
                    fmt_duration(t.saturating_sub(game.state().time)),
                ),
                Stat::PiecesLocked(p) => ("Pieces left:", p.saturating_sub(pieces).to_string()),
                Stat::LinesCleared(l) => (
                    "Lines left:",
                    l.saturating_sub(game.state().lineclears).to_string(),
                ),
                Stat::PointsScored(s) => (
                    "Points left:",
                    s.saturating_sub(game.state().points).to_string(),
                ),
            }
        } else {
            ("", "".to_owned())
        };

        let show_hold = game.state().piece_held.is_some();
        let show_next = !game.state().piece_preview.is_empty();
        let show_lockdelay = game
            .state()
            .fall_delay_lowerbound_hit_at_n_lineclears
            .is_some()
            && !game.config.lock_delay_params.is_constant();

        // Screen: draw.
        #[allow(clippy::useless_format)]
        #[rustfmt::skip]
        let base_screen: &[String] = match settings.graphics().glyphset {
            Glyphset::Elektronika_60 => &[
                format!("                                                              ", ),
                format!("                                                {: ^w$      } ", "mode:", w=modename_len),
                format!("                        <! . . . . . . . . . .!>{: ^w$      } ", meta_data.title, w=modename_len),
                format!("  STATS                 <! . . . . . . . . . .!>{: ^w$      } ", "", w=modename_len),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!(" Time:   {:<15         }<! . . . . . . . . . .!> {           }", fmt_duration(game.state().time), endcond_title),
                format!(" Lines:  {:<15         }<! . . . . . . . . . .!>   {         }", game.state().lineclears, endcond_value),
                format!(" Points: {:<15         }<! . . . . . . . . . .!>              ", game.state().points),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!(" Gravity: {:<14        }<! . . . . . . . . . .!>              ", fmt_hertz(gravity)),
                format!(" {:<23                 }<! . . . . . . . . . .!>              ", if show_lockdelay { format!("Lock delay: {}ms",game.state().lock_delay.saturating_duration().as_millis()) } else { "".to_owned() }),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!("  KEYBINDS              <! . . . . . . . . . .!>              ", ),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!("                        <!====================!>              ", ),
               format!(r"                          \/\/\/\/\/\/\/\/\/\/                ", ),
            ],
            Glyphset::ASCII => &[
                format!("                                                              ", ),
                format!("                  {     }|- - - - - - - - - - +{:-^w$       }+", if show_hold { "+-hold-" } else {"       "}, "mode", w=modename_len),
                format!("                  {}     |                    |{: ^w$       }|", if show_hold { "| " } else {"  "}, meta_data.title, w=modename_len),
                format!("  STATS           {     }|                    +{:-^w$       }+", if show_hold { "+------" } else {"       "}, "", w=modename_len),
                format!(" ----------              |                    |               ", ),
                format!(" Time:   {:<16          }|                    |  {           }", fmt_duration(game.state().time), endcond_title),
                format!(" Lines:  {:<16          }|                    |    {         }", game.state().lineclears, endcond_value),
                format!(" Points: {:<16          }|                    |               ", game.state().points),
                format!("                         |                    |{             }", if show_next { "-----next-----+" } else {"               "}),
                format!(" Gravity: {:<15         }|                    |             {}", fmt_hertz(gravity), if show_next { " |" } else {"  "}),
                format!(" {:<24                  }|                    |             {}", if show_lockdelay { format!("Lock delay: {}ms",game.state().lock_delay.saturating_duration().as_millis()) } else { "".to_owned() }, if show_next { " |" } else {"  "}),
                format!("                         |                    |{             }", if show_next { "--------------+" } else {"               "}),
                format!("                         |                    |               ", ),
                format!("                         |                    |               ", ),
                format!("                         |                    |               ", ),
                format!("  KEYBINDS               |                    |               ", ),
                format!(" ----------              |                    |               ", ),
                format!("                         |                    |               ", ),
                format!("                         |                    |               ", ),
                format!("                         |                    |               ", ),
                format!("                         |                    |               ", ),
                format!("                         |                    |               ", ),
                format!("                        ~#====================#~              ", ),
            ],
        Glyphset::Unicode => &[
                format!("                                                              ", ),
                format!("                  {     }╓╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╥{:─^w$       }┐", if show_hold { "┌─hold─" } else {"       "}, "mode", w=modename_len),
                format!("                  {}     ║                    ║{: ^w$       }│", if show_hold { "│ " } else {"  "}, meta_data.title, w=modename_len),
                format!("  STATS           {     }║                    ╟{:─^w$       }┘", if show_hold { "└──────" } else {"       "}, "", w=modename_len),
                format!(" ─────────╴              ║                    ║               ", ),
                format!(" Time:   {:<16          }║                    ║  {           }", fmt_duration(game.state().time), endcond_title),
                format!(" Lines:  {:<16          }║                    ║    {         }", game.state().lineclears, endcond_value),
                format!(" Points: {:<16          }║                    ║               ", game.state().points),
                format!("                         ║                    ║{             }", if show_next { "─────next─────┐" } else {"               "}),
                format!(" Gravity: {:<15         }║                    ║             {}", fmt_hertz(gravity), if show_next { " │" } else {"  "}),
                format!(" {:<24                  }║                    ║             {}", if show_lockdelay { format!("Lock delay: {}ms",game.state().lock_delay.saturating_duration().as_millis()) } else { "".to_owned() }, if show_next { " │" } else {"  "}),
                format!("                         ║                    ║{             }", if show_next { "──────────────┘" } else {"               "}),
                format!("                         ║                    ║               ", ),
                format!("                         ║                    ║               ", ),
                format!("                         ║                    ║               ", ),
                format!("  KEYBINDS               ║                    ║               ", ),
                format!(" ─────────╴              ║                    ║               ", ),
                format!("                         ║                    ║               ", ),
                format!("                         ║                    ║               ", ),
                format!("                         ║                    ║               ", ),
                format!("                         ║                    ║               ", ),
                format!("                         ║                    ║               ", ),
                format!("                      ░▒▓█▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀█▓▒░            ", ),
            ],
        };

        self.screen
            .buffer_from(base_screen.iter().map(String::as_str));

        // Positioning of dynamically rendered elements.
        let (x_board, y_board) = (26, 1);
        let (x_hold, y_hold) = (20, 2);
        let (x_preview, y_preview) = (50, 10);
        let (x_preview_small, y_preview_small) = (49, 13);
        let (x_preview_mini, y_preview_mini) = (50, 15);
        let (x_messages, y_messages) = (49, 18);
        let (x_keybinds, y_keybinds) = (1, 17);
        let (x_rep_hdr, y_rep_hdr) = (1, 1);
        let (x_rep_spd, y_rep_spd) = (1, 11);
        let (x_rep_len, y_rep_len) = (1, 12);
        let (x_buttonst, y_buttonst) = (48, 17);
        // FIXME: Returning `None` as soon as it is OOB for the rectangle of our custom game screen buffer.
        // But this is wasteful if there's actual space in the TUI above and we cut off 'for no reason'.
        let pos_board = |(x, y)| {
            Some((
                x_board + 2 * (x as usize),
                (y_board + Game::LOCK_OUT_HEIGHT).checked_sub_signed(y)?,
            ))
        };

        // Color helpers.
        let get_color = |tile_id: TileID| settings.palette().get(&tile_id).copied();

        // Print keybinds legend.
        const W_KEYBINDS: usize = 23;
        // FIXME: Kinda inefficient to this iterating each time maybe?
        let desc_len = keybinds_legend
            .iter()
            .map(|s| s.1.chars().count())
            .max()
            .unwrap_or(0);
        let icons_available_len = W_KEYBINDS - desc_len - 1;
        let icons_len = keybinds_legend
            .iter()
            .map(|s| s.0.chars().count())
            .max()
            .unwrap_or(0)
            .min(icons_available_len);
        for (dy, (icons, desc)) in keybinds_legend.iter().enumerate() {
            let pos = (x_keybinds, y_keybinds + dy);
            let icons = icons.chars().take(icons_len).collect::<String>();
            self.screen
                .buffer_str(&format!("{icons: >icons_len$} {desc}"), None, pos);
        }

        if let Some((replay_length, replay_speed)) = replay_extra {
            // Rendering a replay, show additional info.

            // Replay header.
            self.screen
                .buffer_str("(Viewing REPLAY)", None, (x_rep_hdr, y_rep_hdr));

            // Replay speed.
            self.screen.buffer_str(
                &format!("Replay speed: {:.2}x", replay_speed),
                None,
                (x_rep_spd, y_rep_spd),
            );

            // Replay length.
            self.screen.buffer_str(
                &format!("Total time {}", fmt_duration(replay_length)),
                None,
                (x_rep_len, y_rep_len),
            );
        }

        // Draw button state.
        if settings.graphics().show_button_state || replay_extra.is_some() {
            let n253 = NonZeroU8::try_from(253).unwrap();
            let n255 = NonZeroU8::try_from(255).unwrap();
            let bc = |b: Button| {
                get_color(if game.state().active_buttons[b].is_some() {
                    n255
                } else {
                    n253
                })
            };
            let es = [
                Err("("),
                Ok(Button::MoveLeft),
                Ok(Button::DropSoft),
                Ok(Button::MoveRight),
                Err(" "),
                Ok(Button::RotateLeft),
                Ok(Button::Rotate180),
                Ok(Button::RotateRight),
                Err(" "),
                Ok(Button::DropHard),
                Err(" "),
                Ok(Button::HoldPiece),
                Err(" "),
                Ok(Button::TeleLeft),
                Ok(Button::TeleDown),
                Ok(Button::TeleRight),
                Err(")"),
            ];
            for (dx, e) in es.into_iter().enumerate() {
                match e {
                    Ok(b) => self.screen.buffer_str(
                        if settings.graphics().glyphset != Glyphset::Unicode {
                            fmt_button_ascii
                        } else {
                            fmt_button
                        }(b),
                        bc(b),
                        (x_buttonst + dx, y_buttonst),
                    ),
                    Err(s) => self
                        .screen
                        .buffer_str(s, None, (x_buttonst + dx, y_buttonst)),
                }
            }
        }

        let (tile_ground, tile_shadow, tile_active, tile_preview) =
            match settings.graphics().glyphset {
                Glyphset::Elektronika_60 => ("▮▮", " .", "▮▮", "▮▮"),
                Glyphset::ASCII => ("##" /*"$$"*/, "::", "[]", "[]"),
                Glyphset::Unicode => ("██", "░░", "▓▓", "██" /*"▒▒"*/),
            };

        // Draw preview.
        if let Some(next_piece) = game.state().piece_preview.front() {
            let color = get_color(next_piece.tile_id());
            for (x, y) in next_piece.minos(Orientation::N) {
                let pos = (
                    (if *next_piece == Tetromino::O { 2 } else { 0 } + x_preview + 2 * x) as usize,
                    (y_preview - y) as usize,
                );
                self.screen.buffer_str(tile_preview, color, pos);
            }
        }

        // Draw small preview pieces 2,3,4.
        let mut x_offset_small = 0;
        for tet in game.state().piece_preview.iter().skip(1).take(3) {
            let str = if settings.graphics().glyphset == Glyphset::Unicode {
                tet.linestr()
            } else {
                tet.linestr_ascii()
            };
            self.screen.buffer_str(
                str,
                get_color(tet.tile_id()),
                (x_preview_small + x_offset_small, y_preview_small),
            );
            x_offset_small += str.chars().count() + 1;
        }

        // Draw minuscule preview pieces 5,6,7,8...
        let mut x_offset_minuscule = 0;
        for tet in game.state().piece_preview.iter().skip(4) {
            //.take(5) {
            let str = String::from(if settings.graphics().glyphset == Glyphset::Unicode {
                tet.charstr()
            } else {
                tet.charstr_ascii()
            });
            self.screen.buffer_str(
                &str,
                get_color(tet.tile_id()),
                (x_preview_mini + x_offset_minuscule, y_preview_mini),
            );
            x_offset_minuscule += str.chars().count() + 1;
        }

        // Draw held piece.
        if let Some((tet, swap_allowed)) = game.state().piece_held {
            let str = if settings.graphics().glyphset == Glyphset::Unicode {
                tet.linestr()
            } else {
                tet.linestr_ascii()
            };
            let color = get_color(if swap_allowed {
                tet.tile_id()
            } else {
                NonZeroU8::try_from(254).unwrap()
            });
            self.screen.buffer_str(str, color, (x_hold, y_hold));
        }

        // Board: draw hard drop trail.
        for (
            HardDropTile {
                creation_time,
                pos,
                y_offset,
                tile_id,
            },
            active,
        ) in self.hard_drop_tiles.iter_mut()
        {
            let elapsed = game.state().time.saturating_sub(*creation_time);
            let luminance_map = match settings.graphics().glyphset {
                Glyphset::Elektronika_60 => [" .", " .", " .", " .", " .", " .", " .", " ."],
                // FIXME: Make this hard drop effect available independently of Glyphset (i.e. also for ASCII).
                Glyphset::ASCII => ["||", "||", "¦¦", "¦¦", "::", "::", "..", ".."],
                Glyphset::Unicode => ["@@", "$$", "##", "%%", "**", "++", "~~", ".."],
            };
            // let Some(&char) = [50, 60, 70, 80, 90, 110, 140, 180]
            let Some(tile) = [50, 70, 90, 110, 130, 150, 180, 240]
                .iter()
                .enumerate()
                .find_map(|(idx, ms)| (elapsed < Duration::from_millis(*ms)).then_some(idx))
                .and_then(|dt| luminance_map.get(*y_offset * 4 / 7 + dt))
            else {
                *active = false;
                continue;
            };
            if let Some(xy) = pos_board(*pos) {
                self.screen.buffer_str(tile, get_color(*tile_id), xy);
            }
        }

        self.hard_drop_tiles.retain(|elt| elt.1);

        // Board: draw locked tiles.
        if !temp_data.blindfold_enabled {
            for (y, line) in game.state().board.iter().enumerate().rev() {
                for (x, cell) in line.iter().enumerate() {
                    if let Some(tile_id) = cell {
                        if let Some(xy) =
                            pos_board((isize::try_from(x).unwrap(), isize::try_from(y).unwrap()))
                        {
                            let color_locked = settings.palette_lockedtiles().get(tile_id).copied();
                            self.screen.buffer_str(tile_ground, color_locked, xy);
                        }
                    }
                }
            }
        }

        match game.phase() {
            // FIXME: No visual indicator for spawn phase currently.
            Phase::Spawning { spawn_time: _ } => {}

            // If a piece is in play.
            Phase::PieceInPlay { piece, .. } => {
                // Draw shadow piece.
                if settings.graphics().show_shadow_piece {
                    for (tile_pos, tile_id) in
                        piece.teleported(&game.state().board, (0, -1)).tiles()
                    {
                        if let Some(xy) = pos_board(tile_pos) {
                            self.screen.buffer_str(tile_shadow, get_color(tile_id), xy);
                        }
                    }
                }

                // Draw active piece.
                for (tile_pos, tile_id) in piece.tiles() {
                    if let Some(xy) = pos_board(tile_pos) {
                        self.screen.buffer_str(tile_active, get_color(tile_id), xy);
                    }
                }
            }

            // FIXME: No visual indicator for lineclear phase currently.
            Phase::LinesClearing { .. } => {
                // TODO: Hack.
                for m in &self.mino_particles {
                    self.screen.buffer_str("  ", None, m.0.origin);
                }
            }

            Phase::GameEnd { cause, is_win: _ } => {
                match cause {
                    GameEndCause::LockOut { locking_piece } => {
                        for (tile_pos, tile_id) in locking_piece.tiles() {
                            if let Some(xy) = pos_board(tile_pos) {
                                self.screen.buffer_str(
                                    "XX",
                                    get_color(tile_id), /*Some(Color::Red)*/
                                    xy,
                                );
                            }
                        }
                    }
                    GameEndCause::BlockOut { blocked_piece } => {
                        // Special hack to make block-out piece more visible.
                        for (notification, _notif_time, active) in
                            self.notification_feed_buffer.iter_mut()
                        {
                            if matches!(notification, Notification::PieceLocked { .. }) {
                                *active = false;
                            }
                        }

                        for (tile_pos @ (x, y), tile_id) in blocked_piece.tiles() {
                            if let Some(xy) = pos_board(tile_pos) {
                                let (t, c) = if let Some(board_tile) =
                                    game.state().board[y as usize][x as usize]
                                {
                                    ("XX", get_color(board_tile))
                                } else {
                                    ("XX", get_color(tile_id) /*Some(Color::Red)*/)
                                };

                                self.screen.buffer_str(t, c, xy);
                            }
                        }
                    }

                    // FIXME: No visual indicator for topout currently.
                    GameEndCause::TopOut { top_lines: _ } => {}

                    // FIXME: No visual indicator for gameover-by-some-limit currently.
                    GameEndCause::Limit(_) => {}

                    GameEndCause::Forfeit { piece_in_play } => {
                        if let Some(piece) = piece_in_play {
                            for (tile_pos, tile_id) in piece.tiles() {
                                if let Some(xy) = pos_board(tile_pos) {
                                    self.screen.buffer_str(
                                        "XX",
                                        get_color(tile_id), /*Some(Color::Red)*/
                                        xy,
                                    );
                                }
                            }
                        }
                    }

                    // Do not draw special visual indication for custom end cause.
                    GameEndCause::Custom(_) => {}
                }
            }
        }

        let (w_term, h_term) = terminal::size()?; // FIXME: Hack.
        for (
            MinoParticle {
                creation_time,
                origin: (x_o, y_o),
                momentum: (m_x, m_y),
                acceleration: (a_x, a_y),
                actually_render,
                tile_id,
            },
            active,
        ) in &mut self.mino_particles
        {
            if !*actually_render {
                continue;
            }

            let mut t_elapsed = game
                .state()
                .time
                .saturating_sub(*creation_time)
                .as_secs_f32();
            if t_elapsed > 3. {
                *active = false;
                continue;
            }

            // Keep particle at original position for a bit.
            t_elapsed -= 0.005;
            if t_elapsed < 0. {
                t_elapsed = 0.;
            }

            // FIXME: This `as` cast is annoying. Why isn't there a `try_from` at the least?
            let pos_x = (*x_o as f32) + (t_elapsed * *m_x + t_elapsed.powf(2.) * *a_x / 2.);
            let pos_y = (*y_o as f32) - (t_elapsed * *m_y + t_elapsed.powf(2.) * *a_y / 2.);

            // TODO: PLEASE refactor all code not to use `as` casts.
            if !(0..w_term).contains(&(pos_x.round() as u16))
                || !(0..h_term).contains(&(pos_y.round() as u16))
            {
                *active = false;
                continue;
            }

            self.screen.buffer_str(
                tile_ground,
                get_color(*tile_id /*NonZeroU8::new(254).unwrap()*/),
                (pos_x.round() as usize, pos_y.round() as usize),
            );
        }

        self.mino_particles.retain(|elt| elt.1);

        // Handle feedback.
        // TODO: This stuff should be processed before drawing...
        // Ideally we'll have buffers for every type of effect, properly.
        for (notification, notif_time, active) in self.notification_feed_buffer.iter_mut() {
            let elapsed = game.state().time.saturating_sub(*notif_time);
            match notification {
                Notification::PieceLocked { piece } => {
                    if !settings.graphics().show_effects {
                        *active = false;
                        continue;
                    }
                    #[rustfmt::skip]
                    let animation_locking = match settings.graphics().glyphset {
                        Glyphset::Elektronika_60 => [
                            ( 25, "▮▮"),
                            ( 50, "▮▮"),
                            ( 75, "▮▮"),
                            (100, "▮▮"),
                            (125, "▮▮"),
                            (150, "▮▮"),
                        ],
                        Glyphset::ASCII => [
                            ( 25, "()"),
                            ( 50, "()"),
                            ( 75, "{}"),
                            (100, "{}"),
                            (125, "<>"),
                            (150, "<>"),
                        ],
                        Glyphset::Unicode => [
                            ( 25, "██"),
                            ( 50, "▓▓"),
                            ( 75, "▒▒"),
                            (100, "░░"),
                            (125, "▒▒"),
                            (150, "▓▓"),
                        ],
                    };
                    let color_locking = get_color(NonZeroU8::try_from(255).unwrap());
                    // FIXME: Possibly replace these manual find-tile snippets with flexible/parameterized/interpolated-time animations (see lineclear animation).
                    let Some(tile) = animation_locking.iter().find_map(|(ms, tile)| {
                        (elapsed < Duration::from_millis(*ms)).then_some(tile)
                    }) else {
                        *active = false;
                        continue;
                    };

                    for (tile_pos, _tile_id) in piece.tiles() {
                        if let Some(xy) = pos_board(tile_pos) {
                            self.screen.buffer_str(tile, color_locking, xy);
                        }
                    }
                }

                Notification::LinesClearing {
                    y_coords,
                    line_clear_duration,
                } => {
                    if settings.graphics().lineclear_style == 0 {
                        if !settings.graphics().show_effects || line_clear_duration.is_zero() {
                            *active = false;
                            continue;
                        }
                        let animation_lineclear = match settings.graphics().glyphset {
                            Glyphset::Elektronika_60 => [
                                "▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮",
                                "  ▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮",
                                "    ▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮",
                                "      ▮▮▮▮▮▮▮▮▮▮▮▮▮▮",
                                "        ▮▮▮▮▮▮▮▮▮▮▮▮",
                                "          ▮▮▮▮▮▮▮▮▮▮",
                                "            ▮▮▮▮▮▮▮▮",
                                "              ▮▮▮▮▮▮",
                                "                ▮▮▮▮",
                                "                  ▮▮",
                            ],
                            Glyphset::ASCII => [
                                "$$$$$$$$$$$$$$$$$$$$",
                                "$$$$$$$$$$$$$$$$$$$$",
                                "                    ",
                                "                    ",
                                "$$$$$$$$$$$$$$$$$$$$",
                                "$$$$$$$$$$$$$$$$$$$$",
                                "                    ",
                                "                    ",
                                "$$$$$$$$$$$$$$$$$$$$",
                                "$$$$$$$$$$$$$$$$$$$$",
                            ],
                            Glyphset::Unicode => [
                                "████████████████████",
                                " ██████████████████ ",
                                "  ████████████████  ",
                                "   ██████████████   ",
                                "    ████████████    ",
                                "     ██████████     ",
                                "      ████████      ",
                                "       ██████       ",
                                "        ████        ",
                                "         ██         ",
                            ],
                        };
                        let color_lineclear = get_color(NonZeroU8::try_from(255).unwrap());
                        let percent = elapsed.as_secs_f64() / line_clear_duration.as_secs_f64();
                        let max_idx =
                            f64::from(i32::try_from(animation_lineclear.len() - 1).unwrap());
                        let idx = if (0.0..=1.0).contains(&percent) {
                            // SAFETY: `0.0 <= percent && percent <= 1.0`.
                            unsafe { (percent * max_idx).round().to_int_unchecked::<usize>() }
                        } else {
                            *active = false;
                            continue;
                        };
                        for y_line in y_coords {
                            let pos = (x_board, y_board + Game::LOCK_OUT_HEIGHT - *y_line);
                            self.screen
                                .buffer_str(animation_lineclear[idx], color_lineclear, pos);
                        }
                    } else {
                        for y in y_coords.iter().copied() {
                            for x in 0..Game::WIDTH {
                                if let Some(origin) = pos_board((
                                    isize::try_from(x).unwrap(),
                                    isize::try_from(y).unwrap(),
                                )) {
                                    let mult_m_x = rand::rng().random_range(-1.0..1.0);
                                    let mult_m_y = rand::rng().random_range(0.8..1.0);
                                    let new_particle = MinoParticle {
                                        creation_time: *notif_time,
                                        origin,
                                        momentum: (mult_m_x * 60.0, mult_m_y * 50.0),
                                        acceleration: (0.0, -200.0),
                                        actually_render: true, /*(x /*+ rand::rng().random_range(0..=1)*/).is_multiple_of(2),*/
                                        tile_id: game.state().board[y][x]
                                            .unwrap_or(NonZeroU8::new(254).unwrap()),
                                    };

                                    self.mino_particles.push((new_particle, true));
                                }
                            }
                        }

                        *active = false;
                    }
                }

                Notification::HardDrop {
                    height_dropped: _,
                    dropped_piece,
                } => {
                    if !settings.graphics().show_effects {
                        *active = false;
                        continue;
                    }
                    for ((x_tile, y_tile), tile_id) in dropped_piece.tiles() {
                        for dy in (y_tile as usize)..Game::LOCK_OUT_HEIGHT {
                            self.hard_drop_tiles.push((
                                HardDropTile {
                                    creation_time: *notif_time,
                                    pos: (x_tile, isize::try_from(dy).unwrap()),
                                    y_offset: dy - (y_tile as usize),
                                    tile_id,
                                },
                                true,
                            ));
                        }
                    }

                    *active = false;
                }

                Notification::Accolade {
                    points_bonus,
                    tetromino,
                    is_spin,
                    lineclears,
                    is_perfect_clear,
                    combo,
                } => {
                    let mut tokens = Vec::new();

                    tokens.push(format!("+{points_bonus},"));

                    if *is_perfect_clear {
                        tokens.push("Perfect".to_owned());
                    }

                    let clear_action = match lineclears {
                        1 => "Mono",
                        2 => "Duo",
                        3 => "Tri",
                        4 => "Tetra",
                        5 => "Penta",
                        6 => "Hexa",
                        7 => "Hepta",
                        8 => "Octa",
                        9 => "Ennea",
                        10 => "Deca",
                        11 => "Hendeca",
                        12 => "Dodeca",
                        13 => "Triadeca",
                        14 => "Tessaradeca",
                        15 => "Penteeca",
                        16 => "Hexadeca",
                        17 => "Heptadeca",
                        18 => "Octadeca",
                        19 => "Enneadeca",
                        20 => "Eicosa",
                        _ => "Paralogo",
                    }
                    .to_string();
                    tokens.push(clear_action);

                    if *is_spin {
                        tokens.push(format!("{tetromino:?}-spin"));
                    }

                    if *combo > 1 {
                        tokens.push(format!("x{combo}"));
                    }

                    self.buffered_text_msgs
                        .push((*notif_time, tokens.join(" ")));

                    *active = false;
                }

                Notification::Custom(string) => {
                    self.buffered_text_msgs.push((*notif_time, string.clone()));

                    *active = false;
                }

                Notification::Debug(s) => {
                    self.buffered_text_msgs.push((*notif_time, s.clone()));

                    *active = false;
                }

                Notification::GameEnded { is_win } => {
                    let text = if *is_win {
                        "Game Complete!".to_owned()
                    } else {
                        if let Phase::GameEnd { cause, .. } = game.phase() {
                            format!("{cause}...")
                        } else {
                            // FIXME: This should never happen. Change engine to return the cause in the notification?
                            "Game Over...".to_owned()
                        }
                    };

                    self.buffered_text_msgs.push((*notif_time, text));
                    *active = false;
                }
            }
        }

        // Purge
        self.notification_feed_buffer.retain(|elt| elt.2);

        // Draw messages.
        for (dy, (_timestamp, message)) in self.buffered_text_msgs.iter().rev().enumerate() {
            let pos = (x_messages, y_messages + dy);
            self.screen.buffer_str(message, None, pos);
        }

        self.buffered_text_msgs.retain(|(timestamp, _msg)| {
            game.state().time.saturating_sub(*timestamp) < Duration::from_millis(4000)
        });

        self.screen.flush(term)
    }
}
