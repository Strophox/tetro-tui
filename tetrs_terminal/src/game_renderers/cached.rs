use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    fmt::Debug,
    io::{self, Write},
    time::Duration,
};

use crossterm::{
    cursor,
    event::KeyCode,
    style::{self, Color, Print, Stylize},
    terminal, QueueableCommand,
};
use tetrs_engine::{
    Button, Coord, FeedbackEvent, Game, GameConfig, GameState, GameTime, Orientation, Stat,
    Tetromino, TileTypeID,
};

use crate::{
    game_renderers::GameScreenRenderer,
    terminal_tetrs::{format_duration, format_key, format_keybinds, App, GameRunningStats},
};

#[derive(Clone, Default, Debug)]
struct ScreenBuf {
    prev: Vec<Vec<(char, Option<Color>)>>,
    next: Vec<Vec<(char, Option<Color>)>>,
    x_draw: usize,
    y_draw: usize,
}

#[derive(Clone, Default, Debug)]
pub struct Renderer {
    screen: ScreenBuf,
    visual_events: Vec<(GameTime, FeedbackEvent, bool)>,
    messages: BinaryHeap<(GameTime, String)>,
    hard_drop_tiles: Vec<(GameTime, Coord, usize, TileTypeID, bool)>,
}

impl ScreenBuf {
    fn buf_from_strs(&mut self, base_screen: Vec<String>) {
        self.next = base_screen
            .iter()
            .map(|str| str.chars().zip(std::iter::repeat(None)).collect())
            .collect();
    }

    fn buf_str(&mut self, str: &str, fg_color: Option<Color>, (x, y): (usize, usize)) {
        for (x_c, c) in str.chars().enumerate() {
            // Lazy: just fill up until desired thing row exists.
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

    fn buf_reset(&mut self, (x, y): (usize, usize)) {
        self.prev.clear();
        (self.x_draw, self.y_draw) = (x, y);
    }

    fn flush(&mut self, term: &mut impl Write) -> io::Result<()> {
        // Begin frame update.
        term.queue(terminal::BeginSynchronizedUpdate)?;
        if self.prev.is_empty() {
            // Redraw entire screen.
            term.queue(terminal::Clear(terminal::ClearType::All))?;
            for (y, line) in self.next.iter().enumerate() {
                for (x, cell) in line.iter().enumerate() {
                    self.put(term, cell, x, y)?;
                }
            }
        } else {
            // Compare two frames and only write differences.
            for (y, (prev_line, next_line)) in self.prev.iter().zip(self.next.iter()).enumerate() {
                for (x, (prev_cell, next_cell)) in
                    prev_line.iter().zip(next_line.iter()).enumerate()
                {
                    if next_cell != prev_cell {
                        self.put(term, next_cell, x, y)?;
                    }
                }
                match prev_line.len().cmp(&next_line.len()) {
                    Ordering::Less => {
                        for (x, next_cell) in next_line.iter().enumerate().skip(prev_line.len()) {
                            self.put(term, next_cell, x, y)?;
                        }
                    }
                    Ordering::Equal => {}
                    Ordering::Greater => {
                        for x in next_line.len()..prev_line.len() {
                            self.put(term, &(' ', None), x, y)?;
                        }
                    }
                }
            }
            match self.prev.len().cmp(&self.next.len()) {
                Ordering::Less => {
                    for (y, next_line) in self.next.iter().enumerate().skip(self.prev.len()) {
                        for (x, next_cell) in next_line.iter().enumerate() {
                            self.put(term, next_cell, x, y)?;
                        }
                    }
                }
                Ordering::Equal => {}
                Ordering::Greater => {
                    for (y, prev_line) in self.prev.iter().enumerate().skip(self.next.len()) {
                        for (x, _) in prev_line.iter().enumerate() {
                            self.put(term, &(' ', None), x, y)?;
                        }
                    }
                }
            }
        }
        // End frame update and flush.
        term.queue(terminal::EndSynchronizedUpdate)?;
        term.flush()?;
        // Clear old.
        self.prev = Vec::new();
        // Swap buffers.
        std::mem::swap(&mut self.prev, &mut self.next);
        Ok(())
    }

    fn put(
        &self,
        term: &mut impl Write,
        (c, col): &(char, Option<Color>),
        x: usize,
        y: usize,
    ) -> io::Result<()> {
        term.queue(cursor::MoveTo(
            u16::try_from(self.x_draw + x).unwrap(),
            u16::try_from(self.y_draw + y).unwrap(),
        ))?;
        if let Some(color) = col {
            term.queue(style::PrintStyledContent(c.with(*color)))?;
        } else {
            term.queue(Print(c))?;
        }
        Ok(())
    }
}

impl GameScreenRenderer for Renderer {
    // NOTE self: what is the concept of having an ADT but some functions are only defined on some variants (that may contain record data)?
    fn render<T>(
        &mut self,
        app: &mut App<T>,
        game: &mut Game,
        action_stats: &mut GameRunningStats,
        new_feedback_events: Vec<(GameTime, FeedbackEvent)>,
        screen_resized: bool,
    ) -> io::Result<()>
    where
        T: Write,
    {
        if screen_resized {
            let (x_main, y_main) = App::<T>::fetch_main_xy();
            self.screen
                .buf_reset((usize::from(x_main), usize::from(y_main)));
        }
        let GameState {
            game_time,
            finished: _,
            events: _,
            buttons_pressed: _,
            board,
            active_piece_data,
            next_pieces,
            pieces_played,
            lines_cleared,
            level,
            score,
            consecutive_line_clears: _,
            back_to_back_special_clears: _,
        } = game.state();
        let GameConfig { gamemode, .. } = game.config();
        // Screen: some values.
        let lines = lines_cleared.len();
        // Screen: some helpers.
        let stat_name = |stat| match stat {
            Stat::Lines(_) => "Lines",
            Stat::Level(_) => "Levels",
            Stat::Score(_) => "Score",
            Stat::Pieces(_) => "Pieces",
            Stat::Time(_) => "Time",
        };
        // Screen: some titles.
        let mode_name = gamemode.name.to_ascii_uppercase();
        let mode_name_space = mode_name.len().max(14);
        let opti_name = stat_name(gamemode.optimize);
        let opti_value = match gamemode.optimize {
            Stat::Lines(_) => format!("{}", lines),
            Stat::Level(_) => format!("{}", level),
            Stat::Score(_) => format!("{}", score),
            Stat::Pieces(_) => format!("{}", pieces_played.iter().sum::<u32>()),
            Stat::Time(_) => format_duration(*game_time),
        };
        let (goal_name, goal_value) = if let Some(stat) = gamemode.limit {
            (
                format!("{} left:", stat_name(stat)),
                match stat {
                    Stat::Lines(lns) => format!("{}", lns.saturating_sub(lines)),
                    Stat::Level(lvl) => format!("{}", lvl.get().saturating_sub(level.get())),
                    Stat::Score(pts) => format!("{}", pts.saturating_sub(*score)),
                    Stat::Pieces(pcs) => {
                        format!("{}", pcs.saturating_sub(pieces_played.iter().sum::<u32>()))
                    }
                    Stat::Time(dur) => format_duration(dur.saturating_sub(*game_time)),
                },
            )
        } else {
            ("".to_string(), "".to_string())
        };
        let key_icon_pause = format_key(KeyCode::Esc);
        let key_icons_moveleft = format_keybinds(Button::MoveLeft, &app.settings.keybinds);
        let key_icons_moveright = format_keybinds(Button::MoveRight, &app.settings.keybinds);
        let mut key_icons_move = format!("{key_icons_moveleft} {key_icons_moveright}");
        let key_icons_rotateleft = format_keybinds(Button::RotateLeft, &app.settings.keybinds);
        let key_icons_rotateright = format_keybinds(Button::RotateRight, &app.settings.keybinds);
        let mut key_icons_rotate = format!("{key_icons_rotateleft} {key_icons_rotateright}");
        let key_icons_dropsoft = format_keybinds(Button::DropSoft, &app.settings.keybinds);
        let key_icons_drophard = format_keybinds(Button::DropHard, &app.settings.keybinds);
        let mut key_icons_drop = format!("{key_icons_dropsoft} {key_icons_drophard}");
        // JESUS Christ https://users.rust-lang.org/t/truncating-a-string/77903/9 :
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
        let piececnts_o = format!("{}o", pieces_played[Tetromino::O]);
        let piececnts_i_s_z = [
            format!("{}i", pieces_played[Tetromino::I]),
            format!("{}s", pieces_played[Tetromino::S]),
            format!("{}z", pieces_played[Tetromino::Z]),
        ]
        .join("  ");
        let piececnts_t_l_j = [
            format!("{}t", pieces_played[Tetromino::T]),
            format!("{}l", pieces_played[Tetromino::L]),
            format!("{}j", pieces_played[Tetromino::J]),
        ]
        .join("  ");
        // Screen: draw.
        #[allow(clippy::useless_format)]
        #[rustfmt::skip]
        let base_screen = vec![
            format!("                                                            ", ),
            format!("                       ╓╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╥{:─^w$       }┐", "mode", w=mode_name_space),
            format!("    ALL STATS          ║                    ║{: ^w$       }│", mode_name, w=mode_name_space),
            format!("    ─────────╴         ║                    ╟{:─^w$       }┘", "", w=mode_name_space),
            format!("    Level: {:<12      }║                    ║  {          }:", level, opti_name),
            format!("    Score: {:<12      }║                    ║{:^15         }", score, opti_value),
            format!("    Lines: {:<12      }║                    ║               ", lines),
            format!("                       ║                    ║  {           }", goal_name),
            format!("    Time elapsed       ║                    ║{:^15         }", goal_value),
            format!("     {:<18            }║                    ║               ", format_duration(*game_time)),
            format!("                       ║                    ║─────next─────┐", ),
            format!("    PIECES             ║                    ║              │", ),
            format!("    ──────╴            ║                    ║              │", ),
            format!("    {:<19             }║                    ║──────────────┘", piececnts_o),
            format!("    {:<19             }║                    ║               ", piececnts_i_s_z),
            format!("    {:<19             }║                    ║               ", piececnts_t_l_j),
            format!("                       ║                    ║               ", ),
            format!("    CONTROLS           ║                    ║               ", ),
            format!("    ────────╴          ║                    ║               ", ),
            format!("    Pause   {:<11     }║                    ║               ", key_icon_pause),
            format!("    Move    {:<11     }║                    ║               ", key_icons_move),
            format!("    Rotate  {:<11     }║                    ║               ", key_icons_rotate),
            format!("    Drop    {:<11     }╚════════════════════╝               ", key_icons_drop),
            format!("                                                            ", ),
        ];
        self.screen.buf_from_strs(base_screen);
        let (x_board, y_board) = (24, 1);
        let (x_preview, y_preview) = (48, 12);
        let (x_messages, y_messages) = (47, 15);
        let pos_board = |(x, y)| (x_board + 2 * x, y_board + Game::SKYLINE - y);
        // Board: helpers.
        let tile_color = |tile: TileTypeID| {
            Some(match tile.get() {
                1 => Color::Rgb {
                    r: 254,
                    g: 203,
                    b: 0,
                },
                2 => Color::Rgb {
                    r: 0,
                    g: 159,
                    b: 218,
                },
                3 => Color::Rgb {
                    r: 105,
                    g: 190,
                    b: 40,
                },
                4 => Color::Rgb {
                    r: 237,
                    g: 41,
                    b: 57,
                },
                5 => Color::Rgb {
                    r: 149,
                    g: 45,
                    b: 152,
                },
                6 => Color::Rgb {
                    r: 255,
                    g: 121,
                    b: 0,
                },
                7 => Color::Rgb {
                    r: 0,
                    g: 101,
                    b: 189,
                },
                t => unimplemented!("formatting unknown tile id {t}"),
            })
        };
        // Board: draw hard drop trail.
        for (event_time, pos, h, tile_type_id, relevant) in self.hard_drop_tiles.iter_mut() {
            let elapsed = game_time.saturating_sub(*event_time);
            let luminance_map = "@$#%*+~.".as_bytes();
            // let Some(&char) = [50, 60, 70, 80, 90, 110, 140, 180]
            let Some(&char) = [50, 70, 90, 110, 130, 150, 180, 240]
                .iter()
                .enumerate()
                .find_map(|(idx, ms)| (elapsed < Duration::from_millis(*ms)).then_some(idx))
                .and_then(|dt| luminance_map.get(*h / 2 + dt))
            else {
                *relevant = false;
                continue;
            };
            // SAFETY: Valid ASCII bytes.
            let tile = String::from_utf8(vec![char, char]).unwrap();
            self.screen
                .buf_str(&tile, tile_color(*tile_type_id), pos_board(*pos));
        }
        self.hard_drop_tiles.retain(|elt| elt.4);
        // Board: draw fixed tiles.
        for (y, line) in board.iter().enumerate().take(21).rev() {
            for (x, cell) in line.iter().enumerate() {
                if let Some(tile_type_id) = cell {
                    self.screen
                        .buf_str("██", tile_color(*tile_type_id), pos_board((x, y)));
                }
            }
        }
        // If a piece is in play.
        if let Some((active_piece, _)) = active_piece_data {
            // Draw ghost piece.
            for (tile_pos, tile_type_id) in active_piece.well_piece(board).tiles() {
                if tile_pos.1 <= Game::SKYLINE {
                    self.screen
                        .buf_str("░░", tile_color(tile_type_id), pos_board(tile_pos));
                }
            }
            // Draw active piece.
            for (tile_pos, tile_type_id) in active_piece.tiles() {
                if tile_pos.1 <= Game::SKYLINE {
                    self.screen
                        .buf_str("▓▓", tile_color(tile_type_id), pos_board(tile_pos));
                }
            }
        }
        // Draw preview.
        // TODO: Possibly implement more preview.
        if game.config().preview_count > 0 {
            // SAFETY: `preview_count > 0`.
            let next_piece = next_pieces.front().unwrap();
            let color = tile_color(next_piece.tiletypeid());
            for (x, y) in next_piece.minos(Orientation::N) {
                let pos = (x_preview + 2 * x, y_preview - y);
                self.screen.buf_str("▒▒", color, pos);
            }
        }
        // Update stored events.
        self.visual_events.extend(
            new_feedback_events
                .into_iter()
                .map(|(time, event)| (time, event, true)),
        );
        // Draw events.
        for (event_time, event, relevant) in self.visual_events.iter_mut().rev() {
            let elapsed = game_time.saturating_sub(*event_time);
            match event {
                FeedbackEvent::PieceLocked(piece) => {
                    let Some(tile) = [
                        (50, "██"),
                        (75, "▓▓"),
                        (100, "▒▒"),
                        (125, "░░"),
                        (150, "▒▒"),
                        (175, "▓▓"),
                    ]
                    .iter()
                    .find_map(|(ms, tile)| (elapsed < Duration::from_millis(*ms)).then_some(tile)) else {
                        *relevant = false;
                        continue;
                    };
                    for (tile_pos, _tile_type_id) in piece.tiles() {
                        if tile_pos.1 <= Game::SKYLINE {
                            self.screen
                                .buf_str(tile, Some(Color::White), pos_board(tile_pos));
                        }
                    }
                }
                FeedbackEvent::LineClears(lines_cleared, line_clear_delay) => {
                    if line_clear_delay.is_zero() {
                        *relevant = false;
                        continue;
                    }
                    let line_clear_frames = [
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
                    ];
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
                            .buf_str(line_clear_frames[idx], Some(Color::White), pos);
                    }
                }
                FeedbackEvent::HardDrop(_top_piece, bottom_piece) => {
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
                FeedbackEvent::Accolade {
                    score_bonus,
                    shape,
                    spin,
                    lineclears,
                    perfect_clear,
                    combo,
                    opportunity,
                } => {
                    action_stats.1.push(*score_bonus);
                    let mut strs = Vec::new();
                    strs.push(format!("+{score_bonus}"));
                    if *perfect_clear {
                        strs.push("PERFECT".to_string());
                    }
                    if *spin {
                        strs.push(format!("{shape:?}-Spin"));
                        action_stats.0[0] += 1;
                    }
                    let clear_action = match lineclears {
                        1 => "Single",
                        2 => "Double",
                        3 => "Triple",
                        4 => "Quadruple",
                        x => unreachable!("unexpected line clear count {x}"),
                    }
                    .to_ascii_uppercase();
                    action_stats.0[usize::try_from(*lineclears).unwrap()] += 1;
                    let excl = match opportunity {
                        1 => "'",
                        2 => "!",
                        3 => "!'",
                        4 => "!!",
                        x => unreachable!("unexpected opportunity count {x}"),
                    };
                    strs.push(format!("{clear_action}{excl}"));
                    if *combo > 1 {
                        strs.push(format!("[{combo}.combo]"));
                    }
                    self.messages.push((*event_time, strs.join(" ")));
                    *relevant = false;
                }
                FeedbackEvent::Debug(msg) => {
                    self.messages.push((*event_time, msg.clone()));
                    *relevant = false;
                }
            }
        }
        self.visual_events.retain(|elt| elt.2);
        // Draw messages.
        for (y, (_event_time, message)) in self.messages.iter().enumerate() {
            let pos = (x_messages, y_messages + y);
            self.screen.buf_str(message, None, pos);
        }
        self.messages.retain(|(event_time, _message)| {
            game_time.saturating_sub(*event_time) < Duration::from_millis(6000)
        });
        self.screen.flush(&mut app.term)
    }
}
