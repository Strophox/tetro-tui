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

use tetrs_engine::{Button, Coord, Feedback, InGameTime, Orientation, Stat, Tetromino, TileTypeID};

use super::*;

use crate::{
    application::{Application, Glyphset},
    fmt_helpers::{fmt_button, fmt_duration, fmt_hertz, fmt_tet_mini, fmt_tet_small},
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
    fn buffer_reset(&mut self, (x, y): (usize, usize)) {
        self.prev.clear();
        (self.x_draw, self.y_draw) = (x, y);
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
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
struct HardDropTile {
    creation_time: InGameTime,
    pos: Coord,
    y_offset: usize,
    tile_type_id: TileTypeID,
}

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
pub struct DiffPrintRenderer {
    screen: TerminalScreenBuffer,
    buffered_feedback_msgs: Vec<(InGameTime, Feedback, bool)>,
    buffered_text_msgs: Vec<(InGameTime, String)>,
    hard_drop_tiles: Vec<(HardDropTile, bool)>,
}

impl Renderer for DiffPrintRenderer {
    fn push_game_feedback_msgs(
        &mut self,
        new_msgs: impl IntoIterator<Item = (InGameTime, Feedback)>,
    ) {
        // Update stored events.
        self.buffered_feedback_msgs.extend(
            new_msgs
                .into_iter()
                .map(|(time, event)| (time, event, true)),
        );
    }

    fn render<T>(
        &mut self,
        game: &Game,
        meta_data: &GameMetaData,
        settings: &Settings,
        keybinds_legend: &KeybindsLegend,
        replay_extra: Option<(InGameTime, f64)>,
        term: &mut T,
        rerender_entire_view: bool,
    ) -> io::Result<()>
    where
        T: Write,
    {
        if rerender_entire_view {
            let (x_main, y_main) = Application::<T>::fetch_main_xy();
            self.screen
                .buffer_reset((usize::from(x_main), usize::from(y_main)));
        }
        let pieces = game.state().pieces_locked.iter().sum::<u32>();
        let gravity = game.state().fall_delay.as_hertz();
        // Screen: some titles.
        let modename_len = meta_data.title.len().max(14);
        let (endcond_title, endcond_value) = if let Some((c, _)) = game
            .config
            .end_conditions
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
                    s.saturating_sub(game.state().score).to_string(),
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
            Glyphset::Electronika60 => &[
                format!("                                                              ", ),
                format!("                                                {: ^w$      } ", "mode:", w=modename_len),
                format!("                        <! . . . . . . . . . .!>{: ^w$      } ", meta_data.title, w=modename_len),
                format!("  STATS                 <! . . . . . . . . . .!>{: ^w$      } ", "", w=modename_len),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!(" Time:  {:<16          }<! . . . . . . . . . .!> {           }", fmt_duration(game.state().time), endcond_title),
                format!(" Lines: {:<16          }<! . . . . . . . . . .!>   {         }", game.state().lineclears, endcond_value),
                format!(" Score: {:<16          }<! . . . . . . . . . .!>              ", game.state().score),
                format!("                        <! . . . . . . . . . .!>              ", ),
                format!(" Gravity: {:<14        }<! . . . . . . . . . .!>              ", fmt_hertz(gravity)),
                format!(" {:<23                 }<! . . . . . . . . . .!>              ", if show_lockdelay { format!("Lock delay: {}", format!("{}ms",game.state().lock_delay.saturating_duration().as_millis())) } else { "".to_owned() }),
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
                format!(" Time:  {:<17           }|                    |  {           }", fmt_duration(game.state().time), endcond_title),
                format!(" Lines: {:<17           }|                    |    {         }", game.state().lineclears, endcond_value),
                format!(" Score: {:<17           }|                    |               ", game.state().score),
                format!("                         |                    |{             }", if show_next { "-----next-----+" } else {"               "}),
                format!(" Gravity: {:<15         }|                    |             {}", fmt_hertz(gravity), if show_next { " |" } else {"  "}),
                format!(" {:<24                  }|                    |             {}", if show_lockdelay { format!("Lock delay: {}", format!("{}ms",game.state().lock_delay.saturating_duration().as_millis())) } else { "".to_owned() }, if show_next { " |" } else {"  "}),
                format!("                         |                    |{             }", if show_next { "--------------+" } else {"               "}),
                format!("                         |                    |               ", ),
                format!("                         |                    |               ", ),
                format!("  KEYBINDS               |                    |               ", ),
                format!(" ----------              |                    |               ", ),
                format!("                         |                    |               ", ),
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
                format!(" Time:  {:<17           }║                    ║  {           }", fmt_duration(game.state().time), endcond_title),
                format!(" Lines: {:<17           }║                    ║    {         }", game.state().lineclears, endcond_value),
                format!(" Score: {:<17           }║                    ║               ", game.state().score),
                format!("                         ║                    ║{             }", if show_next { "─────next─────┐" } else {"               "}),
                format!(" Gravity: {:<15         }║                    ║             {}", fmt_hertz(gravity), if show_next { " │" } else {"  "}),
                format!(" {:<24                  }║                    ║             {}", if show_lockdelay { format!("Lock delay: {}", format!("{}ms",game.state().lock_delay.saturating_duration().as_millis())) } else { "".to_owned() }, if show_next { " │" } else {"  "}),
                format!("                         ║                    ║{             }", if show_next { "──────────────┘" } else {"               "}),
                format!("                         ║                    ║               ", ),
                format!("                         ║                    ║               ", ),
                format!("  KEYBINDS               ║                    ║               ", ),
                format!(" ─────────╴              ║                    ║               ", ),
                format!("                         ║                    ║               ", ),
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
        let (x_keybinds, y_keybinds) = (1, 16);
        let (x_rep_hdr, y_rep_hdr) = (1, 1);
        let (x_rep_spd, y_rep_spd) = (1, 11);
        let (x_rep_len, y_rep_len) = (1, 12);
        let (x_buttonst, y_buttonst) = (48, 17);
        let pos_board = |(x, y)| (x_board + 2 * x, y_board + Game::SKYLINE_HEIGHT - y);

        // Color helpers.
        let get_color =
            |tile_type_id: &TileTypeID| settings.palette().get(&tile_type_id.get()).copied();
        let get_color_locked = |tile_type_id: &TileTypeID| {
            settings
                .palette_lockedtiles()
                .get(&tile_type_id.get())
                .copied()
        };

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
                &format!("Replay speed: {:.2}", replay_speed),
                None,
                (x_rep_spd, y_rep_spd),
            );

            // Replay length.
            self.screen.buffer_str(
                &format!("Game len.: {}", fmt_duration(replay_length)),
                None,
                (x_rep_len, y_rep_len),
            );
        }

        // Draw button state.
        if settings.graphics().show_button_state || replay_extra.is_some() {
            let n253 = NonZeroU8::try_from(253).unwrap();
            let n255 = NonZeroU8::try_from(255).unwrap();
            let bc = |b: Button| {
                get_color(if game.state().buttons_pressed[b].is_some() {
                    &n255
                } else {
                    &n253
                })
            };
            let es = [
                Err("("),
                Ok(Button::MoveLeft),
                Ok(Button::DropSoft),
                Ok(Button::MoveRight),
                Err(" "),
                Ok(Button::RotateLeft),
                Ok(Button::RotateAround),
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
                    Ok(b) => {
                        self.screen
                            .buffer_str(fmt_button(b), bc(b), (x_buttonst + dx, y_buttonst))
                    }
                    Err(s) => self
                        .screen
                        .buffer_str(s, None, (x_buttonst + dx, y_buttonst)),
                }
            }
        }

        // Board: draw hard drop trail.
        for (
            HardDropTile {
                creation_time,
                pos,
                y_offset,
                tile_type_id,
            },
            active,
        ) in self.hard_drop_tiles.iter_mut()
        {
            let elapsed = game.state().time.saturating_sub(*creation_time);
            let luminance_map = match settings.graphics().glyphset {
                Glyphset::Electronika60 => [" .", " .", " .", " .", " .", " .", " .", " ."],
                Glyphset::ASCII | Glyphset::Unicode => {
                    ["@@", "$$", "##", "%%", "**", "++", "~~", ".."]
                }
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
            self.screen
                .buffer_str(tile, get_color(tile_type_id), pos_board(*pos));
        }
        self.hard_drop_tiles.retain(|elt| elt.1);

        let (tile_ground, tile_shadow, tile_active, tile_preview) =
            match settings.graphics().glyphset {
                Glyphset::Electronika60 => ("▮▮", " .", "▮▮", "▮▮"),
                Glyphset::ASCII => ("##", "::", "[]", "[]"),
                Glyphset::Unicode => ("██", "░░", "▓▓", "██" /*"▒▒"*/),
            };

        // Board: draw locked tiles.
        if !settings.graphics().blindfolded {
            for (y, line) in game.state().board.iter().enumerate().take(21).rev() {
                for (x, cell) in line.iter().enumerate() {
                    if let Some(tile_type_id) = cell {
                        self.screen.buffer_str(
                            tile_ground,
                            get_color_locked(tile_type_id),
                            pos_board((x, y)),
                        );
                    }
                }
            }
        }

        // If a piece is in play.
        if let tetrs_engine::Phase::PieceInPlay {
            piece_data: tetrs_engine::PieceData { piece, .. },
            ..
        } = game.phase()
        {
            // Draw shadow piece.
            if settings.graphics().show_shadow_piece {
                for (tile_pos, tile_type_id) in
                    piece.teleported(&game.state().board, (0, -1)).tiles()
                {
                    if tile_pos.1 <= Game::SKYLINE_HEIGHT {
                        self.screen.buffer_str(
                            tile_shadow,
                            get_color(&tile_type_id),
                            pos_board(tile_pos),
                        );
                    }
                }
            }

            // Draw active piece.
            for (tile_pos, tile_type_id) in piece.tiles() {
                if tile_pos.1 <= Game::SKYLINE_HEIGHT {
                    self.screen.buffer_str(
                        tile_active,
                        get_color(&tile_type_id),
                        pos_board(tile_pos),
                    );
                }
            }
        }

        // Draw preview.
        if let Some(next_piece) = game.state().piece_preview.front() {
            let color = get_color(&next_piece.tiletypeid());
            for (x, y) in next_piece.minos(Orientation::N) {
                let pos = (
                    if *next_piece == Tetromino::O { 2 } else { 0 } + x_preview + 2 * x,
                    y_preview - y,
                );
                self.screen.buffer_str(tile_preview, color, pos);
            }
        }

        // Draw small preview pieces 2,3,4.
        let mut x_offset_small = 0;
        for tet in game.state().piece_preview.iter().skip(1).take(3) {
            let str = fmt_tet_small(*tet);
            self.screen.buffer_str(
                str,
                get_color(&tet.tiletypeid()),
                (x_preview_small + x_offset_small, y_preview_small),
            );
            x_offset_small += str.chars().count() + 1;
        }
        // Draw minuscule preview pieces 5,6,7,8...
        let mut x_offset_minuscule = 0;
        for tet in game.state().piece_preview.iter().skip(4) {
            //.take(5) {
            let str = fmt_tet_mini(*tet);
            self.screen.buffer_str(
                str,
                get_color(&tet.tiletypeid()),
                (x_preview_mini + x_offset_minuscule, y_preview_mini),
            );
            x_offset_minuscule += str.chars().count() + 1;
        }
        // Draw held piece.
        if let Some((tet, swap_allowed)) = game.state().piece_held {
            let str = fmt_tet_small(tet);
            let color = get_color(&if swap_allowed {
                tet.tiletypeid()
            } else {
                NonZeroU8::try_from(254).unwrap()
            });
            self.screen.buffer_str(str, color, (x_hold, y_hold));
        }

        // Handle feedback.
        for (feedback_time, feedback, active) in self.buffered_feedback_msgs.iter_mut() {
            let elapsed = game.state().time.saturating_sub(*feedback_time);
            match feedback {
                Feedback::PieceLocked { piece } => {
                    if !settings.graphics().show_effects {
                        *active = false;
                        continue;
                    }
                    #[rustfmt::skip]
                    let animation_locking = match settings.graphics().glyphset {
                        Glyphset::Electronika60 => [
                            ( 50, "▮▮"),
                            ( 75, "▮▮"),
                            (100, "▮▮"),
                            (125, "▮▮"),
                            (150, "▮▮"),
                            (175, "▮▮"),
                        ],
                        Glyphset::ASCII => [
                            ( 50, "()"),
                            ( 75, "()"),
                            (100, "{}"),
                            (125, "{}"),
                            (150, "<>"),
                            (175, "<>"),
                        ],
                        Glyphset::Unicode => [
                            ( 50, "██"),
                            ( 75, "▓▓"),
                            (100, "▒▒"),
                            (125, "░░"),
                            (150, "▒▒"),
                            (175, "▓▓"),
                        ],
                    };
                    let color_locking = get_color(&NonZeroU8::try_from(255).unwrap());
                    // FIXME: Possibly replace these manual find-tile snippets with flexible/parameterized/interpolated-time animations (see lineclear animation).
                    let Some(tile) = animation_locking.iter().find_map(|(ms, tile)| {
                        (elapsed < Duration::from_millis(*ms)).then_some(tile)
                    }) else {
                        *active = false;
                        continue;
                    };

                    for (tile_pos, _tile_type_id) in piece.tiles() {
                        if tile_pos.1 <= Game::SKYLINE_HEIGHT {
                            self.screen
                                .buffer_str(tile, color_locking, pos_board(tile_pos));
                        }
                    }
                }

                Feedback::LinesClearing {
                    y_coords,
                    line_clear_start: line_clear_duration,
                } => {
                    if !settings.graphics().show_effects || line_clear_duration.is_zero() {
                        *active = false;
                        continue;
                    }
                    let animation_lineclear = match settings.graphics().glyphset {
                        Glyphset::Electronika60 => [
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
                    let color_lineclear = get_color(&NonZeroU8::try_from(255).unwrap());
                    let percent = elapsed.as_secs_f64() / line_clear_duration.as_secs_f64();
                    let max_idx = f64::from(i32::try_from(animation_lineclear.len() - 1).unwrap());
                    let idx = if (0.0..=1.0).contains(&percent) {
                        // SAFETY: `0.0 <= percent && percent <= 1.0`.
                        unsafe { (percent * max_idx).round().to_int_unchecked::<usize>() }
                    } else {
                        *active = false;
                        continue;
                    };
                    for y_line in y_coords {
                        let pos = (x_board, y_board + Game::SKYLINE_HEIGHT - *y_line);
                        self.screen
                            .buffer_str(animation_lineclear[idx], color_lineclear, pos);
                    }
                }

                Feedback::HardDrop {
                    old_piece: _,
                    new_piece,
                } => {
                    if !settings.graphics().show_effects {
                        *active = false;
                        continue;
                    }
                    for ((x_tile, y_tile), tile_type_id) in new_piece.tiles() {
                        for dy in y_tile..Game::SKYLINE_HEIGHT {
                            self.hard_drop_tiles.push((
                                HardDropTile {
                                    creation_time: *feedback_time,
                                    pos: (x_tile, dy),
                                    y_offset: dy - y_tile,
                                    tile_type_id,
                                },
                                true,
                            ));
                        }
                    }

                    *active = false;
                }

                Feedback::Accolade {
                    score_bonus,
                    tetromino,
                    is_spin: spin,
                    lineclears,
                    is_perfect_clear: perfect_clear,
                    combo,
                } => {
                    let mut text = Vec::new();
                    text.push(format!("+{score_bonus}"));
                    if *perfect_clear {
                        text.push("Perfect".to_owned());
                    }
                    if *spin {
                        text.push(format!("{tetromino:?}-Spin"));
                    }
                    let clear_action = match lineclears {
                        1 => "Single",
                        2 => "Double",
                        3 => "Triple",
                        4 => "Quadruple",
                        5 => "Quintuple",
                        6 => "Sextuple",
                        7 => "Septuple",
                        8 => "Octuple",
                        9 => "Nonuple",
                        10 => "Decuple",
                        11 => "Undecuple",
                        12 => "Duodecuple",
                        13 => "Tredecuple",
                        14 => "Quattuordecuple",
                        15 => "Quindecuple",
                        16 => "Sexdecuple",
                        17 => "Septendecuple",
                        18 => "Octodecuple",
                        19 => "Novemdecuple",
                        20 => "Vigintuple",
                        21 => "Kirbtris",
                        _ => "Unreachable",
                    }
                    .to_string();
                    text.push(clear_action);
                    if *combo > 1 {
                        text.push(format!("#{combo}."));
                    }
                    self.buffered_text_msgs
                        .push((*feedback_time, text.join(" ")));

                    *active = false;
                }

                Feedback::Text(string) => {
                    self.buffered_text_msgs
                        .push((*feedback_time, string.clone()));

                    *active = false;
                }

                Feedback::Debug(update_point) => {
                    self.buffered_text_msgs
                        .push((*feedback_time, format!("{update_point:?}")));

                    *active = false;
                }

                Feedback::GameEnded { result } => {
                    let text = match result {
                        Ok(_stat) => "Game Complete!".to_owned(),
                        Err(cause) => format!("{cause:?}..."),
                    };

                    self.buffered_text_msgs.push((*feedback_time, text));
                    *active = false;
                }
            }
        }

        // Purge
        self.buffered_feedback_msgs.retain(|elt| elt.2);

        // Draw messages.
        for (dy, (_timestamp, message)) in self.buffered_text_msgs.iter().rev().enumerate() {
            let pos = (x_messages, y_messages + dy);
            self.screen.buffer_str(message, None, pos);
        }

        self.buffered_text_msgs.retain(|(timestamp, _msg)| {
            game.state().time.saturating_sub(*timestamp) < Duration::from_millis(5000)
        });

        self.screen.flush(term)
    }
}
