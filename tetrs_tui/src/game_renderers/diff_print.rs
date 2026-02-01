use std::{
    cmp::Ordering,
    fmt::{Debug, Display},
    io::{self, Write},
    num::NonZeroU8,
    time::Duration,
};

use crossterm::{
    cursor,
    style::{self, Color, Print, PrintStyledContent, Stylize},
    terminal, QueueableCommand,
};
use tetrs_engine::{
    Button, Coord, Feedback, FeedbackMessages, Game, GameTime, Orientation, Stat, State, TileTypeID,
};

use crate::{
    application::{Application, GameMetaData, Glyphset},
    fmt_utils::{fmt_duration, fmt_keybinds_of, fmt_tet_mini, fmt_tet_small},
    game_renderers::Renderer,
};

#[derive(Clone, Default, Debug)]
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

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
struct HardDropTile {
    creation_time: GameTime,
    pos: Coord,
    y_offset: usize,
    tile_type_id: TileTypeID,
}

#[derive(Clone, Default, Debug)]
pub struct DiffPrintRenderer {
    screen: TerminalScreenBuffer,
    active_feedback: Vec<(GameTime, Feedback, bool)>,
    messages: Vec<(GameTime, String)>,
    hard_drop_tiles: Vec<(HardDropTile, bool)>,
}

impl Renderer for DiffPrintRenderer {
    fn render<T>(
        &mut self,
        app: &mut Application<T>,
        game: &Game,
        meta_data: &GameMetaData,
        new_feedback_msgs: FeedbackMessages,
        screen_resized: bool,
    ) -> io::Result<()>
    where
        T: Write,
    {
        if screen_resized {
            let (x_main, y_main) = Application::<T>::fetch_main_xy();
            self.screen
                .buffer_reset((usize::from(x_main), usize::from(y_main)));
        }
        let State {
            time: game_time,
            buttons_pressed: _,
            board,
            hold_piece,
            next_pieces,
            piece_generator: _,
            pieces_locked: pieces_played,
            lines_cleared,
            gravity,
            score,
            consecutive_line_clears: _,
            rng: _,
        } = game.state();
        let pieces = pieces_played.iter().sum::<u32>();
        // Screen: some titles.
        let mode_name_space = meta_data.title.len().max(14);
        let (endcond_title, endcond_value) = if let Some((c, _)) = game
            .config
            .end_conditions
            .iter()
            .find(|(_stat, to_win)| *to_win)
        {
            match c {
                Stat::TimeElapsed(t) => ("Time left:", fmt_duration(&t.saturating_sub(*game_time))),
                Stat::PiecesLocked(p) => ("Pieces left:", p.saturating_sub(pieces).to_string()),
                Stat::LinesCleared(l) => {
                    ("Lines left:", l.saturating_sub(*lines_cleared).to_string())
                }
                Stat::GravityReached(g) => (
                    "Gravity levels left:",
                    g.saturating_sub(*gravity).to_string(),
                ),
                Stat::PointsScored(s) => ("Points left:", s.saturating_sub(*score).to_string()),
            }
        } else {
            ("", "".to_owned())
        };
        let f = |b| fmt_keybinds_of(b, app.settings().keybinds());
        let mut icons_move = format!("{}{}", f(Button::MoveLeft), f(Button::MoveRight));
        let mut icons_rotate = format!(
            "{}{}{}",
            f(Button::RotateLeft),
            f(Button::RotateAround),
            f(Button::RotateRight)
        );
        let mut icons_drop = format!(
            "{}{}{}",
            f(Button::DropSoft),
            f(Button::TeleDown),
            f(Button::DropHard)
        );
        let mut icons_hold = f(Button::HoldPiece);
        // FAIR enough https://users.rust-lang.org/t/truncating-a-string/77903/9 :
        let eleven = icons_move
            .char_indices()
            .map(|(i, _)| i)
            .nth(11)
            .unwrap_or(icons_move.len());
        icons_move.truncate(eleven);
        let eleven = icons_rotate
            .char_indices()
            .map(|(i, _)| i)
            .nth(11)
            .unwrap_or(icons_rotate.len());
        icons_rotate.truncate(eleven);
        let eleven = icons_drop
            .char_indices()
            .map(|(i, _)| i)
            .nth(11)
            .unwrap_or(icons_drop.len());
        icons_drop.truncate(eleven);
        let eleven = icons_hold
            .char_indices()
            .map(|(i, _)| i)
            .nth(11)
            .unwrap_or(icons_hold.len());
        icons_hold.truncate(eleven);
        // Screen: draw.
        #[allow(clippy::useless_format)]
        #[rustfmt::skip]
        let base_screen = match app.settings().graphics().glyphset {
            Glyphset::Electronika60 => vec![
                format!("                                                            ", ),
                format!("                                              {: ^w$      } ", "mode:", w=mode_name_space),
                format!("   ALL STATS          <! . . . . . . . . . .!>{: ^w$      } ", meta_data.title, w=mode_name_space),
                format!("   ----------         <! . . . . . . . . . .!>{: ^w$      } ", "", w=mode_name_space),
                format!("   Score: {:<12      }<! . . . . . . . . . .!>              ", score),
                format!("   Lines: {:<12      }<! . . . . . . . . . .!> {           }", lines_cleared, endcond_title),
                format!("                      <! . . . . . . . . . .!>   {         }", endcond_value),
                format!("   Pieces:  {:<10    }<! . . . . . . . . . .!>              ", pieces),
                format!("   Gravity: {:<10    }<! . . . . . . . . . .!>              ", gravity),
                format!("   Time: {:<13       }<! . . . . . . . . . .!>              ", fmt_duration(game_time)),
                format!("                      <! . . . . . . . . . .!>              ", ),
                format!("                      <! . . . . . . . . . .!>              ", ),
                format!("                      <! . . . . . . . . . .!>              ", ),
                format!("                      <! . . . . . . . . . .!>              ", ),
                format!("   KEYBINDS           <! . . . . . . . . . .!>              ", ),
                format!("   ---------          <! . . . . . . . . . .!>              ", ),
                format!("   Move    {:<11     }<! . . . . . . . . . .!>              ", icons_move),
                format!("   Rotate  {:<11     }<! . . . . . . . . . .!>              ", icons_rotate),
                format!("                      <! . . . . . . . . . .!>              ", ),
                format!("   Drop    {:<11     }<! . . . . . . . . . .!>              ", icons_drop),
                format!("   Hold    {:<11     }<! . . . . . . . . . .!>              ", icons_hold),
                format!("   Pause   [Esc]      <! . . . . . . . . . .!>              ", ),
                format!("                      <!====================!>              ", ),
               format!(r"                        \/\/\/\/\/\/\/\/\/\/                ", ),
            ],
            Glyphset::ASCII => vec![
                format!("                                                            ", ),
                format!("                {     }|- - - - - - - - - - +{:-^w$       }+", if hold_piece.is_some() { "+-hold-" } else {"       "}, "mode", w=mode_name_space),
                format!("   ALL STATS    {}     |                    |{: ^w$       }|", if hold_piece.is_some() { "| " } else {"  "}, meta_data.title, w=mode_name_space),
                format!("   ----------   {     }|                    +{:-^w$       }+", if hold_piece.is_some() { "+------" } else {"       "}, "", w=mode_name_space),
                format!("   Score: {:<13       }|                    |               ", score),
                format!("   Lines: {:<13       }|                    |  {           }", lines_cleared, endcond_title),
                format!("                       |                    |    {         }", endcond_value),
                format!("   Pieces:  {:<11     }|                    |               ", pieces),
                format!("   Gravity: {:<11     }|                    |               ", gravity),
                format!("   Time: {:<14        }|                    |{             }", fmt_duration(game_time), if !next_pieces.is_empty() { "-----next-----+" } else {"               "}),
                format!("                       |                    |             {}", if !next_pieces.is_empty() { " |" } else {"  "}),
                format!("                       |                    |             {}", if !next_pieces.is_empty() { " |" } else {"  "}),
                format!("                       |                    |{             }", if !next_pieces.is_empty() { "--------------+" } else {"               "}),
                format!("                       |                    |               ", ),
                format!("   KEYBINDS            |                    |               ", ),
                format!("   ---------           |                    |               ", ),
                format!("   Move    {:<12      }|                    |               ", icons_move),
                format!("   Rotate  {:<12      }|                    |               ", icons_rotate),
                format!("                       |                    |               ", ),
                format!("   Drop    {:<12      }|                    |               ", icons_drop),
                format!("   Hold    {:<12      }|                    |               ", icons_hold),
                format!("   Pause   [Esc]       |                    |               ", ),
                format!("                      ~#====================#~              ", ),
                format!("                                                            ", ),
            ],
        Glyphset::Unicode => vec![
                format!("                                                            ", ),
                format!("                {     }╓╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╥{:─^w$       }┐", if hold_piece.is_some() { "┌─hold─" } else {"       "}, "mode", w=mode_name_space),
                format!("   ALL STATS    {}     ║                    ║{: ^w$       }│", if hold_piece.is_some() { "│ " } else {"  "}, meta_data.title, w=mode_name_space),
                format!("   ─────────╴   {     }║                    ╟{:─^w$       }┘", if hold_piece.is_some() { "└──────" } else {"       "}, "", w=mode_name_space),
                format!("   Score: {:<13       }║                    ║               ", score),
                format!("   Lines: {:<13       }║                    ║  {           }", lines_cleared, endcond_title),
                format!("                       ║                    ║    {         }", endcond_value),
                format!("   Pieces:  {:<11     }║                    ║               ", pieces),
                format!("   Gravity: {:<11     }║                    ║               ", gravity),
                format!("   Time: {:<14        }║                    ║{             }", fmt_duration(game_time), if !next_pieces.is_empty() { "─────next─────┐" } else {"               "}),
                format!("                       ║                    ║             {}", if !next_pieces.is_empty() { " │" } else {"  "}),
                format!("                       ║                    ║             {}", if !next_pieces.is_empty() { " │" } else {"  "}),
                format!("                       ║                    ║{             }", if !next_pieces.is_empty() { "──────────────┘" } else {"               "}),
                format!("                       ║                    ║               ", ),
                format!("   KEYBINDS            ║                    ║               ", ),
                format!("   ────────╴           ║                    ║               ", ),
                format!("   Move    {:<12      }║                    ║               ", icons_move),
                format!("   Rotate  {:<12      }║                    ║               ", icons_rotate),
                format!("                       ║                    ║               ", ),
                format!("   Drop    {:<12      }║                    ║               ", icons_drop),
                format!("   Hold    {:<12      }║                    ║               ", icons_hold),
                format!("   Pause   [Esc]       ║                    ║               ", ),
                format!("                    ░▒▓█▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀█▓▒░            ", ),
                format!("                                                            ", ),
            ],
        };
        self.screen.buffer_from(base_screen);
        let (x_board, y_board) = (24, 1);
        let (x_hold, y_hold) = (18, 2);
        let (x_preview, y_preview) = (48, 11);
        let (x_preview_small, y_preview_small) = (48, 14);
        let (x_preview_minuscule, y_preview_minuscule) = (50, 16);
        let (x_messages, y_messages) = (47, 18);
        let pos_board = |(x, y)| (x_board + 2 * x, y_board + Game::SKYLINE - y);
        // Color helpers.
        let get_color =
            |tile_type_id: &TileTypeID| app.settings().palette().get(&tile_type_id.get()).copied();
        let get_color_locked = |tile_type_id: &TileTypeID| {
            app.settings()
                .palette_lockedtiles()
                .get(&tile_type_id.get())
                .copied()
        };

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
            let elapsed = game_time.saturating_sub(*creation_time);
            let luminance_map = match app.settings().graphics().glyphset {
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

        let (tile_ground, tile_ghost, tile_active, tile_preview) =
            match app.settings().graphics().glyphset {
                Glyphset::Electronika60 => ("▮▮", " .", "▮▮", "▮▮"),
                Glyphset::ASCII => ("##", "::", "[]", "[]"),
                Glyphset::Unicode => ("██", "░░", "▓▓", "▒▒"),
            };
        // Board: draw locked tiles.
        if !app.settings().graphics().blindfolded {
            for (y, line) in board.iter().enumerate().take(21).rev() {
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
            // Draw ghost piece.
            if app.settings().graphics().show_ghost_piece {
                for (tile_pos, tile_type_id) in piece.teleported(board, (0, -1)).tiles() {
                    if tile_pos.1 <= Game::SKYLINE {
                        self.screen.buffer_str(
                            tile_ghost,
                            get_color(&tile_type_id),
                            pos_board(tile_pos),
                        );
                    }
                }
            }

            // Draw active piece.
            for (tile_pos, tile_type_id) in piece.tiles() {
                if tile_pos.1 <= Game::SKYLINE {
                    self.screen.buffer_str(
                        tile_active,
                        get_color(&tile_type_id),
                        pos_board(tile_pos),
                    );
                }
            }
        }

        // Draw preview.
        if let Some(next_piece) = next_pieces.front() {
            let color = get_color(&next_piece.tiletypeid());
            for (x, y) in next_piece.minos(Orientation::N) {
                let pos = (x_preview + 2 * x, y_preview - y);
                self.screen.buffer_str(tile_preview, color, pos);
            }
        }

        // Draw small preview pieces 2,3,4.
        let mut x_offset_small = 0;
        for tet in next_pieces.iter().skip(1).take(3) {
            let str = fmt_tet_small(tet);
            self.screen.buffer_str(
                str,
                get_color(&tet.tiletypeid()),
                (x_preview_small + x_offset_small, y_preview_small),
            );
            x_offset_small += str.chars().count() + 1;
        }
        // Draw minuscule preview pieces 5,6,7,8...
        let mut x_offset_minuscule = 0;
        for tet in next_pieces.iter().skip(4) {
            //.take(5) {
            let str = fmt_tet_mini(tet);
            self.screen.buffer_str(
                str,
                get_color(&tet.tiletypeid()),
                (
                    x_preview_minuscule + x_offset_minuscule,
                    y_preview_minuscule,
                ),
            );
            x_offset_minuscule += str.chars().count() + 1;
        }
        // Draw held piece.
        if let Some((tet, swap_allowed)) = hold_piece {
            let str = fmt_tet_small(tet);
            let color = get_color(&if *swap_allowed {
                tet.tiletypeid()
            } else {
                NonZeroU8::try_from(254).unwrap()
            });
            self.screen.buffer_str(str, color, (x_hold, y_hold));
        }
        // Update stored events.
        self.active_feedback.extend(
            new_feedback_msgs
                .into_iter()
                .map(|(time, event)| (time, event, true)),
        );
        // Handle feedback.
        for (feedback_time, feedback, active) in self.active_feedback.iter_mut() {
            let elapsed = game_time.saturating_sub(*feedback_time);
            match feedback {
                Feedback::PieceLocked(piece) => {
                    if !app.settings().graphics().render_effects {
                        *active = false;
                        continue;
                    }
                    #[rustfmt::skip]
                    let animation_locking = match app.settings().graphics().glyphset {
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
                    // FIXME: Replace all these 'find tile' implementations with configurable system akin to animation lineclear (interpolated time).
                    let Some(tile) = animation_locking.iter().find_map(|(ms, tile)| {
                        (elapsed < Duration::from_millis(*ms)).then_some(tile)
                    }) else {
                        *active = false;
                        continue;
                    };
                    for (tile_pos, _tile_type_id) in piece.tiles() {
                        if tile_pos.1 <= Game::SKYLINE {
                            self.screen
                                .buffer_str(tile, color_locking, pos_board(tile_pos));
                        }
                    }
                }
                Feedback::LinesClearing(lines_cleared, line_clear_delay) => {
                    if !app.settings().graphics().render_effects || line_clear_delay.is_zero() {
                        *active = false;
                        continue;
                    }
                    let animation_lineclear = match app.settings().graphics().glyphset {
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
                    let percent = elapsed.as_secs_f64() / line_clear_delay.as_secs_f64();
                    let max_idx = f64::from(i32::try_from(animation_lineclear.len() - 1).unwrap());
                    // SAFETY: `0.0 <= percent && percent <= 1.0`.
                    let idx = if (0.0..=1.0).contains(&percent) {
                        unsafe { (percent * max_idx).round().to_int_unchecked::<usize>() }
                    } else {
                        *active = false;
                        continue;
                    };
                    for y_line in lines_cleared {
                        let pos = (x_board, y_board + Game::SKYLINE - *y_line);
                        self.screen
                            .buffer_str(animation_lineclear[idx], color_lineclear, pos);
                    }
                }
                Feedback::HardDrop(_top_piece, bottom_piece) => {
                    if !app.settings().graphics().render_effects {
                        *active = false;
                        continue;
                    }
                    for ((x_tile, y_tile), tile_type_id) in bottom_piece.tiles() {
                        for y in y_tile..Game::SKYLINE {
                            self.hard_drop_tiles.push((
                                HardDropTile {
                                    creation_time: *feedback_time,
                                    pos: (x_tile, y),
                                    y_offset: y - y_tile,
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
                    lines_cleared: lineclears,
                    is_perfect_clear: perfect_clear,
                    combo,
                } => {
                    let mut msg = Vec::new();
                    msg.push(format!("+{score_bonus}"));
                    if *perfect_clear {
                        msg.push("Perfect".to_owned());
                    }
                    if *spin {
                        msg.push(format!("{tetromino:?}-Spin"));
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
                    msg.push(clear_action);
                    if *combo > 1 {
                        msg.push(format!("#{combo}."));
                    }
                    self.messages.push((*feedback_time, msg.join(" ")));
                    *active = false;
                }
                Feedback::Text(msg) => {
                    self.messages.push((*feedback_time, msg.clone()));
                    *active = false;
                }
                Feedback::Debug(update_point) => {
                    self.messages
                        .push((*feedback_time, format!("{update_point:?}")));
                    *active = false;
                }
            }
        }
        self.active_feedback.retain(|elt| elt.2);
        // Draw messages.
        for (y, (_timestamp, message)) in self.messages.iter().rev().enumerate() {
            let pos = (x_messages, y_messages + y);
            self.screen.buffer_str(message, None, pos);
        }
        self.messages.retain(|(timestamp, _message)| {
            game_time.saturating_sub(*timestamp) < Duration::from_millis(7000)
        });
        self.screen.flush(&mut app.term)
    }
}
