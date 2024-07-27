use std::{
    collections::BinaryHeap,
    fmt::Debug,
    io::{self, Write},
    time::Duration,
};

use crossterm::{
    cursor::{self, MoveTo},
    event::KeyCode,
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal, QueueableCommand,
};
use tetrs_engine::{
    Button, Coord, Feedback, FeedbackEvents, Game, GameConfig, GameState, GameTime, Orientation,
    Stat, Tetromino, TileTypeID,
};

use crate::{
    game_renderers::GameScreenRenderer,
    terminal_tetrs::{format_duration, format_key, format_keybinds, App, RunningGameStats},
};

#[derive(Clone, Default, Debug)]
pub struct Renderer {
    visual_events: Vec<(GameTime, Feedback, bool)>,
    messages: BinaryHeap<(GameTime, String)>,
    hard_drop_tiles: Vec<(GameTime, Coord, usize, TileTypeID, bool)>,
}

impl GameScreenRenderer for Renderer {
    // NOTE: (note) what is the concept of having an ADT but some functions are only defined on some variants (that may contain record data)?
    fn render<T>(
        &mut self,
        app: &mut App<T>,
        game: &mut Game,
        action_stats: &mut RunningGameStats,
        new_feedback_events: FeedbackEvents,
        clean_screen: bool,
    ) -> io::Result<()>
    where
        T: Write,
    {
        let (x_main, y_main) = App::<T>::fetch_main_xy();
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
            Stat::Lines(_) => format!("{}", lines_cleared),
            Stat::Level(_) => format!("{}", level),
            Stat::Score(_) => format!("{}", score),
            Stat::Pieces(_) => format!("{}", pieces_played.iter().sum::<u32>()),
            Stat::Time(_) => format_duration(*game_time),
        };
        let (goal_name, goal_value) = if let Some(stat) = gamemode.limit {
            (
                format!("{} left:", stat_name(stat)),
                match stat {
                    Stat::Lines(lns) => format!("{}", lns.saturating_sub(*lines_cleared)),
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
        let screen = [
            format!("                                                            ", ),
            format!("                       ╓╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╥{:─^w$       }┐", "mode", w=mode_name_space),
            format!("    ALL STATS          ║                    ║{: ^w$       }│", mode_name, w=mode_name_space),
            format!("    ─────────╴         ║                    ╟{:─^w$       }┘", "", w=mode_name_space),
            format!("    Level: {:<12      }║                    ║  {          }:", level, opti_name),
            format!("    Score: {:<12      }║                    ║{:^15         }", score, opti_value),
            format!("    Lines: {:<12      }║                    ║               ", lines_cleared),
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
        let (x_board, y_board) = (24, 1);
        let (x_preview, y_preview) = (48, 12);
        let (x_messages, y_messages) = (47, 15);
        // Begin frame update.
        app.term.queue(terminal::BeginSynchronizedUpdate)?;
        if clean_screen {
            app.term.queue(terminal::Clear(terminal::ClearType::All))?;
        }
        for (y_screen, str) in screen.iter().enumerate() {
            app.term
                .queue(cursor::MoveTo(
                    x_main,
                    y_main + u16::try_from(y_screen).unwrap(),
                ))?
                .queue(Print(str))?;
        }
        // Board: helpers.
        let tile_color = |tile: TileTypeID| match tile.get() {
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
            255 => Color::Rgb {
                r: 127,
                g: 127,
                b: 127,
            },
            t => unimplemented!("formatting unknown tile id {t}"),
        };
        let board_move_to = |(x, y): Coord| {
            MoveTo(
                x_main + x_board + 2 * u16::try_from(x).unwrap(),
                y_main + y_board + u16::try_from(Game::SKYLINE - y).unwrap(),
            )
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
            app.term
                .queue(board_move_to(*pos))?
                .queue(PrintStyledContent(tile.with(tile_color(*tile_type_id))))?;
        }
        self.hard_drop_tiles.retain(|elt| elt.4);
        // Board: draw fixed tiles.
        for (y, line) in board.iter().enumerate().take(21).rev() {
            for (x, cell) in line.iter().enumerate() {
                if let Some(tile_type_id) = cell {
                    app.term
                        .queue(board_move_to((x, y)))?
                        .queue(PrintStyledContent("██".with(tile_color(*tile_type_id))))?;
                }
            }
        }
        // If a piece is in play.
        if let Some((active_piece, _)) = active_piece_data {
            // Draw ghost piece.
            for (pos, tile_type_id) in active_piece.well_piece(board).tiles() {
                if pos.1 <= Game::SKYLINE {
                    app.term
                        .queue(board_move_to(pos))?
                        .queue(PrintStyledContent("░░".with(tile_color(tile_type_id))))?;
                }
            }
            // Draw active piece.
            for (pos, tile_type_id) in active_piece.tiles() {
                if pos.1 <= Game::SKYLINE {
                    app.term
                        .queue(board_move_to(pos))?
                        .queue(PrintStyledContent("▓▓".with(tile_color(tile_type_id))))?;
                }
            }
        }
        // Draw preview.
        if game.config().preview_count > 0 {
            // SAFETY: `preview_count > 0`.
            let next_piece = next_pieces.front().unwrap();
            let color = tile_color(next_piece.tiletypeid());
            for (x, y) in next_piece.minos(Orientation::N) {
                // SAFETY: We will not exceed the bounds by drawing pieces.
                app.term
                    .queue(MoveTo(
                        x_main + x_preview + u16::try_from(2 * x).unwrap(),
                        y_main + y_preview - u16::try_from(y).unwrap(),
                    ))?
                    .queue(PrintStyledContent("▒▒".with(color)))?;
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
                Feedback::PieceLocked(piece) => {
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
                    for (pos, _tile_type_id) in piece.tiles() {
                        if pos.1 <= Game::SKYLINE {
                            app.term
                                .queue(board_move_to(pos))?
                                .queue(PrintStyledContent(tile.with(Color::White)))?;
                        }
                    }
                }
                Feedback::LineClears(lines_cleared, line_clear_delay) => {
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
                        app.term
                            .queue(MoveTo(
                                x_main + x_board,
                                y_main + y_board + u16::try_from(Game::SKYLINE - *y_line).unwrap(),
                            ))?
                            .queue(PrintStyledContent(
                                line_clear_frames[idx].with(Color::White),
                            ))?;
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
                    strs.push(clear_action);
                    if *combo > 1 {
                        strs.push(format!("[{combo}.combo]"));
                    }
                    if *back_to_back > 1 {
                        strs.push(format!("{back_to_back}-B2B"));
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
        for (y, (_event_time, message)) in self.messages.iter().enumerate() {
            app.term
                .queue(MoveTo(
                    x_main + x_messages,
                    y_main + y_messages + u16::try_from(y).expect("too many messages"),
                ))?
                .queue(Print(message))?;
        }
        self.messages.retain(|(event_time, _message)| {
            game_time.saturating_sub(*event_time) < Duration::from_millis(6000)
        });
        // Execute draw.
        app.term.queue(terminal::EndSynchronizedUpdate)?;
        app.term.flush()?;
        Ok(())
    }
}
