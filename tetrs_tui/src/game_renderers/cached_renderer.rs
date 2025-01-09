use std::{
    cmp::Ordering,
    fmt::{Debug, Display},
    io::{self, Write},
    num::NonZeroU8,
    time::Duration,
};

use crossterm::{
    cursor,
    event::KeyCode,
    style::{self, Color, Print, PrintStyledContent, Stylize},
    terminal, QueueableCommand,
};
use tetrs_engine::{
    Button, Coord, Feedback, FeedbackEvents, Game, GameState, GameTime, Orientation, Tetromino,
    TileTypeID,
};

use crate::{
    game_renderers::Renderer,
    terminal_app::{
        fmt_duration, fmt_key, fmt_keybinds, GraphicsColor, GraphicsStyle, RunningGameStats,
        TerminalApp,
    },
};

use super::{tet_str_minuscule, tet_str_small, tile_to_color};

#[derive(Clone, Default, Debug)]
struct ScreenBuf {
    prev: Vec<Vec<(char, Option<Color>)>>,
    next: Vec<Vec<(char, Option<Color>)>>,
    x_draw: usize,
    y_draw: usize,
}

impl ScreenBuf {
    fn buffer_reset(&mut self, (x, y): (usize, usize)) {
        self.prev.clear();
        (self.x_draw, self.y_draw) = (x, y);
    }

    fn buffer_from(&mut self, base_screen: Vec<String>) {
        self.next = base_screen
            .iter()
            .map(|str| str.chars().zip(std::iter::repeat(None)).collect())
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

#[derive(Clone, Default, Debug)]
pub struct CachedRenderer {
    screen: ScreenBuf,
    visual_events: Vec<(GameTime, Feedback, bool)>,
    messages: Vec<(GameTime, String)>,
    hard_drop_tiles: Vec<(GameTime, Coord, usize, TileTypeID, bool)>,
}

impl Renderer for CachedRenderer {
    // NOTE self: what is the concept of having an ADT but some functions are only defined on some variants (that may contain record data)?
    fn render<T>(
        &mut self,
        app: &mut TerminalApp<T>,
        running_game_stats: &mut RunningGameStats,
        game: &Game,
        new_feedback_events: FeedbackEvents,
        screen_resized: bool,
    ) -> io::Result<()>
    where
        T: Write,
    {
        if screen_resized {
            let (x_main, y_main) = TerminalApp::<T>::fetch_main_xy();
            self.screen
                .buffer_reset((usize::from(x_main), usize::from(y_main)));
        }
        let GameState {
            seed: _,
            end: _,
            time: game_time,
            events: _,
            buttons_pressed: _,
            board,
            active_piece_data,
            hold_piece,
            next_pieces,
            pieces_played,
            lines_cleared,
            gravity,
            score,
            consecutive_line_clears: _,
            back_to_back_special_clears: _,
        } = game.state();
        // Screen: some titles.
        let mode_name = game.mode().name.to_ascii_uppercase();
        let mode_name_space = mode_name.len().max(14);
        let (goal_name, goal_value) = [
            game.mode().limits.time.map(|(_, max_dur)| {
                (
                    "Time left:",
                    fmt_duration(max_dur.saturating_sub(*game_time)),
                )
            }),
            game.mode().limits.pieces.map(|(_, max_pcs)| {
                (
                    "Pieces remaining:",
                    max_pcs
                        .saturating_sub(pieces_played.iter().sum::<u32>())
                        .to_string(),
                )
            }),
            game.mode().limits.lines.map(|(_, max_lns)| {
                (
                    "Lines left to clear:",
                    max_lns.saturating_sub(*lines_cleared).to_string(),
                )
            }),
            game.mode().limits.gravity.map(|(_, max_lvl)| {
                (
                    "Gravity levels to advance:",
                    max_lvl.saturating_sub(*gravity).to_string(),
                )
            }),
            game.mode().limits.score.map(|(_, max_pts)| {
                (
                    "Points to score:",
                    max_pts.saturating_sub(*score).to_string(),
                )
            }),
        ]
        .into_iter()
        .find_map(|limit_text| limit_text)
        .unwrap_or_default();
        let (focus_name, focus_value) = match game.mode().name.as_str() {
            "Marathon" => ("Score:", score.to_string()),
            "40-Lines" => ("Time taken:", fmt_duration(*game_time)),
            "Time Trial" => ("Score:", score.to_string()),
            "Master" => ("", "".to_string()),
            "Puzzle" => ("", "".to_string()),
            _ => ("Lines cleared:", lines_cleared.to_string()),
        };
        let key_icons_moveleft = fmt_keybinds(Button::MoveLeft, &app.settings().keybinds);
        let key_icons_moveright = fmt_keybinds(Button::MoveRight, &app.settings().keybinds);
        let mut key_icons_move = format!("{key_icons_moveleft}{key_icons_moveright}");
        let key_icons_rotateleft = fmt_keybinds(Button::RotateLeft, &app.settings().keybinds);
        let key_icons_rotatearound = fmt_keybinds(Button::RotateAround, &app.settings().keybinds);
        let key_icons_rotateright = fmt_keybinds(Button::RotateRight, &app.settings().keybinds);
        let mut key_icons_rotate =
            format!("{key_icons_rotateleft}{key_icons_rotatearound}{key_icons_rotateright}");
        let key_icons_dropsoft = fmt_keybinds(Button::DropSoft, &app.settings().keybinds);
        let key_icons_dropsonic = fmt_keybinds(Button::DropSonic, &app.settings().keybinds);
        let key_icons_drophard = fmt_keybinds(Button::DropHard, &app.settings().keybinds);
        let mut key_icons_drop =
            format!("{key_icons_dropsoft}{key_icons_dropsonic}{key_icons_drophard}");
        let key_icon_pause = fmt_key(KeyCode::Esc);
        // FAIR enough https://users.rust-lang.org/t/truncating-a-string/77903/9 :
        let eleven = key_icons_move
            .char_indices()
            .map(|(i, _)| i)
            .nth(11)
            .unwrap_or(key_icons_move.len());
        key_icons_move.truncate(eleven);
        let eleven = key_icons_rotate
            .char_indices()
            .map(|(i, _)| i)
            .nth(11)
            .unwrap_or(key_icons_rotate.len());
        key_icons_rotate.truncate(eleven);
        let eleven = key_icons_drop
            .char_indices()
            .map(|(i, _)| i)
            .nth(11)
            .unwrap_or(key_icons_drop.len());
        key_icons_drop.truncate(eleven);
        let piececnts_o_i_s_z = [
            format!("{}o", pieces_played[Tetromino::O]),
            format!("{}i", pieces_played[Tetromino::I]),
            format!("{}s", pieces_played[Tetromino::S]),
            format!("{}z", pieces_played[Tetromino::Z]),
        ]
        .join(" ");
        let piececnts_t_l_j_sum = [
            format!("{}t", pieces_played[Tetromino::T]),
            format!("{}l", pieces_played[Tetromino::L]),
            format!("{}j", pieces_played[Tetromino::J]),
            format!("={}", pieces_played.iter().sum::<u32>()),
        ]
        .join(" ");
        // Screen: draw.
        #[allow(clippy::useless_format)]
        #[rustfmt::skip]
        let base_screen = match app.settings().graphics_style {
            GraphicsStyle::Electronika60 => vec![
                format!("                                                            ", ),
                format!("                                              {: ^w$      } ", "mode:", w=mode_name_space),
                format!("   ALL STATS          <! . . . . . . . . . .!>{: ^w$      } ", mode_name, w=mode_name_space),
                format!("   ----------         <! . . . . . . . . . .!>{: ^w$      } ", "", w=mode_name_space),
                format!("   Gravity: {:<10    }<! . . . . . . . . . .!>  {          }", gravity, goal_name),
                format!("   Lines: {:<12      }<! . . . . . . . . . .!>{:^14        }", lines_cleared, goal_value),
                format!("   Score: {:<12      }<! . . . . . . . . . .!>              ", score),
                format!("                      <! . . . . . . . . . .!>  {          }", focus_name),
                format!("   Time elapsed       <! . . . . . . . . . .!>{:^14        }", focus_value),
                format!("    {:<18            }<! . . . . . . . . . .!>              ", fmt_duration(*game_time)),
                format!("                      <! . . . . . . . . . .!>              ", ),
                format!("   Pieces played      <! . . . . . . . . . .!>              ", ),
                format!("    {:<18            }<! . . . . . . . . . .!>              ", piececnts_o_i_s_z),
                format!("    {:<18            }<! . . . . . . . . . .!>              ", piececnts_t_l_j_sum),
                format!("                      <! . . . . . . . . . .!>              ", ),
                format!("                      <! . . . . . . . . . .!>              ", ),
                format!("   CONTROLS           <! . . . . . . . . . .!>              ", ),
                format!("   ---------          <! . . . . . . . . . .!>              ", ),
                format!("   Move    {:<11     }<! . . . . . . . . . .!>              ", key_icons_move),
                format!("   Rotate  {:<11     }<! . . . . . . . . . .!>              ", key_icons_rotate),
                format!("   Drop    {:<11     }<! . . . . . . . . . .!>              ", key_icons_drop),
                format!("   Pause   {:<11     }<! . . . . . . . . . .!>              ", key_icon_pause),
                format!("                      <!====================!>              ", ),
               format!(r"                        \/\/\/\/\/\/\/\/\/\/                ", ),
            ],
            GraphicsStyle::ASCII => vec![
                format!("                                                            ", ),
                format!("                {     }|- - - - - - - - - - +{:-^w$       }+", if hold_piece.is_some() { "+-hold-" } else {"       "}, "mode", w=mode_name_space),
                format!("   ALL STATS    {}     |                    |{: ^w$       }|", if hold_piece.is_some() { "| " } else {"  "}, mode_name, w=mode_name_space),
                format!("   ----------   {     }|                    +{:-^w$       }+", if hold_piece.is_some() { "+------" } else {"       "}, "", w=mode_name_space),
                format!("   Gravity: {:<11     }|                    |  {           }", gravity, goal_name),
                format!("   Lines: {:<13       }|                    |{:^15         }", lines_cleared, goal_value),
                format!("   Score: {:<13       }|                    |               ", score),
                format!("                       |                    |  {           }", focus_name),
                format!("   Time elapsed        |                    |{:^15         }", focus_value),
                format!("    {:<19             }|                    |               ", fmt_duration(*game_time)),
                format!("                       |                    |{             }", if !next_pieces.is_empty() { "-----next-----+" } else {"               "}),
                format!("   Pieces played       |                    |             {}", if !next_pieces.is_empty() { " |" } else {"  "}),
                format!("    {:<19             }|                    |             {}", piececnts_o_i_s_z, if !next_pieces.is_empty() { " |" } else {"  "}),
                format!("    {:<19             }|                    |{             }", piececnts_t_l_j_sum, if !next_pieces.is_empty() { "--------------+" } else {"               "}),
                format!("                       |                    |               ", ),
                format!("                       |                    |               ", ),
                format!("   CONTROLS            |                    |               ", ),
                format!("   ---------           |                    |               ", ),
                format!("   Move    {:<12      }|                    |               ", key_icons_move),
                format!("   Rotate  {:<12      }|                    |               ", key_icons_rotate),
                format!("   Drop    {:<12      }|                    |               ", key_icons_drop),
                format!("   Pause   {:<12      }|                    |               ", key_icon_pause),
                format!("                      ~#====================#~              ", ),
                format!("                                                            ", ),
            ],
        GraphicsStyle::Unicode => vec![
                format!("                                                            ", ),
                format!("                {     }╓╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╥{:─^w$       }┐", if hold_piece.is_some() { "┌─hold─" } else {"       "}, "mode", w=mode_name_space),
                format!("   ALL STATS    {}     ║                    ║{: ^w$       }│", if hold_piece.is_some() { "│ " } else {"  "}, mode_name, w=mode_name_space),
                format!("   ─────────╴   {     }║                    ╟{:─^w$       }┘", if hold_piece.is_some() { "└──────" } else {"       "}, "", w=mode_name_space),
                format!("   Gravity: {:<11     }║                    ║  {           }", gravity, goal_name),
                format!("   Lines: {:<13       }║                    ║{:^15         }", lines_cleared, goal_value),
                format!("   Score: {:<13       }║                    ║               ", score),
                format!("                       ║                    ║  {           }", focus_name),
                format!("   Time elapsed        ║                    ║{:^15         }", focus_value),
                format!("    {:<19             }║                    ║               ", fmt_duration(*game_time)),
                format!("                       ║                    ║{             }", if !next_pieces.is_empty() { "─────next─────┐" } else {"               "}),
                format!("   Pieces played       ║                    ║             {}", if !next_pieces.is_empty() { " │" } else {"  "}),
                format!("    {:<19             }║                    ║             {}", piececnts_o_i_s_z, if !next_pieces.is_empty() { " │" } else {"  "}),
                format!("    {:<19             }║                    ║{             }", piececnts_t_l_j_sum, if !next_pieces.is_empty() { "──────────────┘" } else {"               "}),
                format!("                       ║                    ║               ", ),
                format!("                       ║                    ║               ", ),
                format!("   CONTROLS            ║                    ║               ", ),
                format!("   ────────╴           ║                    ║               ", ),
                format!("   Move    {:<12      }║                    ║               ", key_icons_move),
                format!("   Rotate  {:<12      }║                    ║               ", key_icons_rotate),
                format!("   Drop    {:<12      }║                    ║               ", key_icons_drop),
                format!("   Pause   {:<12      }║                    ║               ", key_icon_pause),
                format!("                    ░▒▓█▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀█▓▒░            ", ),
                format!("                                                            ", ),
            ],
        };
        self.screen.buffer_from(base_screen);
        let (x_board, y_board) = (24, 1);
        let (x_hold, y_hold) = (18, 2);
        let (x_preview, y_preview) = (48, 12);
        let (x_preview_small, y_preview_small) = (48, 14);
        let (x_preview_minuscule, y_preview_minuscule) = (50, 16);
        let (x_messages, y_messages) = (47, 18);
        let pos_board = |(x, y)| (x_board + 2 * x, y_board + Game::SKYLINE - y);
        // Board: helpers.
        let color = tile_to_color(app.settings().graphics_color);
        let color_locked = tile_to_color(app.settings().graphics_color_locked);
        // Board: draw hard drop trail.
        for (event_time, pos, h, tile_type_id, relevant) in self.hard_drop_tiles.iter_mut() {
            let elapsed = game_time.saturating_sub(*event_time);
            let luminance_map = match app.settings().graphics_style {
                GraphicsStyle::Electronika60 => [" .", " .", " .", " .", " .", " .", " .", " ."],
                GraphicsStyle::ASCII | GraphicsStyle::Unicode => {
                    ["@@", "$$", "##", "%%", "**", "++", "~~", ".."]
                }
            };
            // let Some(&char) = [50, 60, 70, 80, 90, 110, 140, 180]
            let Some(tile) = [50, 70, 90, 110, 130, 150, 180, 240]
                .iter()
                .enumerate()
                .find_map(|(idx, ms)| (elapsed < Duration::from_millis(*ms)).then_some(idx))
                .and_then(|dt| luminance_map.get(*h * 4 / 7 + dt))
            else {
                *relevant = false;
                continue;
            };
            self.screen
                .buffer_str(tile, color(*tile_type_id), pos_board(*pos));
        }
        self.hard_drop_tiles.retain(|elt| elt.4);
        // Board: draw fixed tiles.
        let (tile_ground, tile_ghost, tile_active, tile_preview) =
            match app.settings().graphics_style {
                GraphicsStyle::Electronika60 => ("▮▮", " .", "▮▮", "▮▮"),
                GraphicsStyle::ASCII => ("##", "::", "[]", "[]"),
                GraphicsStyle::Unicode => ("██", "░░", "▓▓", "▒▒"),
            };
        for (y, line) in board.iter().enumerate().take(21).rev() {
            for (x, cell) in line.iter().enumerate() {
                if let Some(tile_type_id) = cell {
                    self.screen.buffer_str(
                        tile_ground,
                        color_locked(*tile_type_id),
                        pos_board((x, y)),
                    );
                }
            }
        }
        // If a piece is in play.
        if let Some((active_piece, _)) = active_piece_data {
            // Draw ghost piece.
            for (tile_pos, tile_type_id) in active_piece.well_piece(board).tiles() {
                if tile_pos.1 <= Game::SKYLINE {
                    self.screen
                        .buffer_str(tile_ghost, color(tile_type_id), pos_board(tile_pos));
                }
            }
            // Draw active piece.
            for (tile_pos, tile_type_id) in active_piece.tiles() {
                if tile_pos.1 <= Game::SKYLINE {
                    self.screen
                        .buffer_str(tile_active, color(tile_type_id), pos_board(tile_pos));
                }
            }
        }
        // Draw preview.
        if let Some(next_piece) = next_pieces.front() {
            let color = color(next_piece.tiletypeid());
            for (x, y) in next_piece.minos(Orientation::N) {
                let pos = (x_preview + 2 * x, y_preview - y);
                self.screen.buffer_str(tile_preview, color, pos);
            }
        }
        // Draw small preview pieces 2,3,4.
        let mut x_offset_small = 0;
        for tet in next_pieces.iter().skip(1).take(3) {
            let str = tet_str_small(tet);
            self.screen.buffer_str(
                str,
                color(tet.tiletypeid()),
                (x_preview_small + x_offset_small, y_preview_small),
            );
            x_offset_small += str.chars().count() + 1;
        }
        // Draw minuscule preview pieces 5,6,7,8...
        let mut x_offset_minuscule = 0;
        for tet in next_pieces.iter().skip(4) {
            //.take(5) {
            let str = tet_str_minuscule(tet);
            self.screen.buffer_str(
                str,
                color(tet.tiletypeid()),
                (
                    x_preview_minuscule + x_offset_minuscule,
                    y_preview_minuscule,
                ),
            );
            x_offset_minuscule += str.chars().count() + 1;
        }
        // Draw held piece.
        if let Some((tet, swap_allowed)) = hold_piece {
            let str = tet_str_small(tet);
            let color = color(if *swap_allowed {
                tet.tiletypeid()
            } else {
                NonZeroU8::try_from(254).unwrap()
            });
            self.screen.buffer_str(str, color, (x_hold, y_hold));
        }
        // Update stored events.
        self.visual_events.extend(
            new_feedback_events
                .into_iter()
                .map(|(time, event)| (time, event, true)),
        );
        // Draw events.
        for (event_time, event, relevant) in self.visual_events.iter_mut() {
            let elapsed = game_time.saturating_sub(*event_time);
            match event {
                Feedback::PieceSpawned(_piece) => {
                    *relevant = false;
                }
                Feedback::PieceLocked(piece) => {
                    #[rustfmt::skip]
                    let animation_locking = match app.settings().graphics_style {
                        GraphicsStyle::Electronika60 => [
                            ( 50, "▮▮"),
                            ( 75, "▮▮"),
                            (100, "▮▮"),
                            (125, "▮▮"),
                            (150, "▮▮"),
                            (175, "▮▮"),
                        ],
                        GraphicsStyle::ASCII => [
                            ( 50, "()"),
                            ( 75, "()"),
                            (100, "{}"),
                            (125, "{}"),
                            (150, "<>"),
                            (175, "<>"),
                        ],
                        GraphicsStyle::Unicode => [
                            ( 50, "██"),
                            ( 75, "▓▓"),
                            (100, "▒▒"),
                            (125, "░░"),
                            (150, "▒▒"),
                            (175, "▓▓"),
                        ],
                    };
                    let color_locking = match app.settings().graphics_color {
                        GraphicsColor::Monochrome => None,
                        GraphicsColor::Color16 | GraphicsColor::Fullcolor => Some(Color::White),
                        GraphicsColor::Experimental => Some(Color::Rgb {
                            r: 207,
                            g: 207,
                            b: 207,
                        }),
                    };
                    let Some(tile) = animation_locking.iter().find_map(|(ms, tile)| {
                        (elapsed < Duration::from_millis(*ms)).then_some(tile)
                    }) else {
                        *relevant = false;
                        continue;
                    };
                    for (tile_pos, _tile_type_id) in piece.tiles() {
                        if tile_pos.1 <= Game::SKYLINE {
                            self.screen
                                .buffer_str(tile, color_locking, pos_board(tile_pos));
                        }
                    }
                }
                Feedback::LineClears(lines_cleared, line_clear_delay) => {
                    if line_clear_delay.is_zero() {
                        *relevant = false;
                        continue;
                    }
                    let animation_lineclear = match app.settings().graphics_style {
                        GraphicsStyle::Electronika60 => [
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
                        GraphicsStyle::ASCII => [
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
                        GraphicsStyle::Unicode => [
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
                    let color_lineclear = match app.settings().graphics_color {
                        GraphicsColor::Monochrome => None,
                        GraphicsColor::Color16
                        | GraphicsColor::Fullcolor
                        | GraphicsColor::Experimental => Some(Color::White),
                    };
                    let percent = elapsed.as_secs_f64() / line_clear_delay.as_secs_f64();
                    // SAFETY: `0.0 <= percent && percent <= 1.0`.
                    let idx = if percent < 1.0 {
                        unsafe { (10.0 * percent).to_int_unchecked::<usize>() }
                    } else {
                        *relevant = false;
                        continue;
                    };
                    for y_line in lines_cleared {
                        let pos = (x_board, y_board + Game::SKYLINE - *y_line);
                        self.screen
                            .buffer_str(animation_lineclear[idx], color_lineclear, pos);
                    }
                }
                Feedback::HardDrop(_top_piece, bottom_piece) => {
                    for ((x_tile, y_tile), tile_type_id) in bottom_piece.tiles() {
                        for y in y_tile..Game::SKYLINE {
                            self.hard_drop_tiles.push((
                                *event_time,
                                (x_tile, y),
                                y - y_tile,
                                tile_type_id,
                                true,
                            ));
                        }
                    }
                    *relevant = false;
                }
                Feedback::Accolade {
                    score_bonus,
                    shape,
                    spin,
                    lineclears,
                    perfect_clear,
                    combo,
                    back_to_back,
                } => {
                    running_game_stats.1.push(*score_bonus);
                    let mut strs = Vec::new();
                    strs.push(format!("+{score_bonus}"));
                    if *perfect_clear {
                        strs.push("Perfect".to_string());
                    }
                    if *spin {
                        strs.push(format!("{shape:?}-Spin"));
                        running_game_stats.0[0] += 1;
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
                    if *lineclears <= 4 {
                        running_game_stats.0[usize::try_from(*lineclears).unwrap()] += 1;
                    } else {
                        // FIXME: Record higher lineclears, if even possible.
                    }
                    strs.push(clear_action);
                    if *combo > 1 {
                        strs.push(format!("({combo}.combo)"));
                    }
                    if *back_to_back > 1 {
                        strs.push(format!("({back_to_back}.B2B)"));
                    }
                    self.messages.push((*event_time, strs.join(" ")));
                    *relevant = false;
                }
                Feedback::Message(msg) => {
                    self.messages.push((*event_time, msg.clone()));
                    *relevant = false;
                }
            }
        }
        self.visual_events.retain(|elt| elt.2);
        // Draw messages.
        for (y, (_event_time, message)) in self.messages.iter().rev().enumerate() {
            let pos = (x_messages, y_messages + y);
            self.screen.buffer_str(message, None, pos);
        }
        self.messages.retain(|(timestamp, _message)| {
            game_time.saturating_sub(*timestamp) < Duration::from_millis(7000)
        });
        self.screen.flush(&mut app.term)
    }
}
