use std::{
    collections::{BinaryHeap, VecDeque},
    fmt::Debug,
    io::{self, Write},
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{self, MoveTo, MoveToNextLine},
    event::KeyCode,
    style::{self, Color, Print, PrintStyledContent, Stylize},
    terminal, QueueableCommand,
};
use tetrs_lib::{
    Button, Coord, FeedbackEvent, Game, GameStateView, MeasureStat, Tetromino, TileTypeID,
};

use crate::terminal_tetrs::TerminalTetrs;

pub trait GameScreenRenderer {
    fn render(
        &mut self,
        ctx: &mut TerminalTetrs<impl Write>,
        game: &mut Game,
        new_feedback_events: Vec<(Instant, FeedbackEvent)>,
    ) -> io::Result<()>;
}

#[derive(Clone, Default, Debug)]
pub struct DebugRenderer {
    feedback_event_buffer: VecDeque<(Instant, FeedbackEvent)>,
}

#[derive(Clone, Default, Debug)]
pub struct UnicodeRenderer {
    events: Vec<(Instant, FeedbackEvent, bool)>,
    accolades: BinaryHeap<(Instant, String)>,
    hard_drop_tiles: Vec<(Instant, Coord, usize, TileTypeID, bool)>,
}

impl GameScreenRenderer for DebugRenderer {
    fn render(
        &mut self,
        ctx: &mut TerminalTetrs<impl Write>,
        game: &mut Game,
        new_feedback_events: Vec<(Instant, FeedbackEvent)>,
    ) -> io::Result<()> {
        // Draw game stuf
        let GameStateView {
            time_updated,
            board,
            active_piece,
            ..
        } = game.state();
        let mut temp_board = board.clone();
        if let Some(active_piece) = active_piece {
            for ((x, y), tile_type_id) in active_piece.tiles() {
                temp_board[y][x] = Some(tile_type_id);
            }
        }
        ctx.term
            .queue(MoveTo(0, 0))?
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;
        ctx.term
            .queue(Print("   +--------------------+"))?
            .queue(MoveToNextLine(1))?;
        for (idx, line) in temp_board.iter().take(20).enumerate().rev() {
            let txt_line = format!(
                "{idx:02} |{}|",
                line.iter()
                    .map(|cell| {
                        cell.map_or(" .", |tile| match tile.get() {
                            1 => "OO",
                            2 => "II",
                            3 => "SS",
                            4 => "ZZ",
                            5 => "TT",
                            6 => "LL",
                            7 => "JJ",
                            t => unimplemented!("formatting unknown tile id {t}"),
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("")
            );
            ctx.term.queue(Print(txt_line))?.queue(MoveToNextLine(1))?;
        }
        ctx.term
            .queue(Print("   +--------------------+"))?
            .queue(MoveToNextLine(1))?;
        ctx.term
            .queue(style::Print(format!(
                "   {:?}",
                time_updated.saturating_duration_since(game.state().time_started)
            )))?
            .queue(MoveToNextLine(1))?;
        // Draw feedback stuf
        for event in new_feedback_events {
            self.feedback_event_buffer.push_front(event);
        }
        let mut feed_evt_msgs = Vec::new();
        for (_, feedback_event) in self.feedback_event_buffer.iter() {
            feed_evt_msgs.push(match feedback_event {
                FeedbackEvent::Accolade {
                    score_bonus,
                    shape,
                    spin,
                    lineclears,
                    perfect_clear,
                    combo,
                    opportunity,
                } => {
                    let mut strs = Vec::new();
                    if *spin {
                        strs.push(format!("{shape:?}-Spin"));
                    }
                    let accolade = match lineclears {
                        1 => "Single",
                        2 => "Double",
                        3 => "Triple",
                        4 => "Quadruple",
                        x => unreachable!("unexpected line clear count {x}"),
                    };
                    let excl = match opportunity {
                        1 => "'",
                        2 => "!",
                        3 => "!'",
                        4 => "!!",
                        x => unreachable!("unexpected opportunity count {x}"),
                    };
                    strs.push(format!("{accolade}{excl}"));
                    if *combo > 1 {
                        strs.push(format!("[{combo}.combo]"));
                    }
                    if *perfect_clear {
                        strs.push("PERFECT!".to_string());
                    }
                    strs.push(format!("+{score_bonus}"));
                    strs.join(" ")
                }
                FeedbackEvent::PieceLocked(_) => continue,
                FeedbackEvent::LineClears(..) => continue,
                FeedbackEvent::HardDrop(_, _) => continue,
                FeedbackEvent::Debug(s) => s.clone(),
            });
        }
        for str in feed_evt_msgs.iter().take(16) {
            ctx.term.queue(Print(str))?.queue(MoveToNextLine(1))?;
        }
        // Execute draw.
        ctx.term.flush()?;
        Ok(())
    }
}

impl GameScreenRenderer for UnicodeRenderer {
    // NOTE: (note) what is the concept of having an ADT but some functions are only defined on some variants (that may contain record data)?
    #[rustfmt::skip]
    fn render(
        &mut self,
        ctx: &mut TerminalTetrs<impl Write>,
        game: &mut Game,
        new_feedback_events: Vec<(Instant, FeedbackEvent)>,
    ) -> io::Result<()> {
        let (console_width, console_height) = terminal::size()?;
        let (w_x, w_y) = (console_width.saturating_sub(80) / 2, console_height.saturating_sub(24) / 2);
        let GameStateView {
            lines_cleared,
            level,
            score,
            time_updated,
            board,
            active_piece,
            next_pieces,
            pieces_played,
            time_started,
            gamemode,
        } = game.state();
        // Screen: some values.
        let lines = lines_cleared.len();
        let time_elapsed = time_updated.saturating_duration_since(time_started);
        // Screen: some helpers.
        let stat_name = |stat| match stat {
            MeasureStat::Lines(_) => "Lines",
            MeasureStat::Level(_) => "Levels",
            MeasureStat::Score(_) => "Score",
            MeasureStat::Pieces(_) => "Pieces",
            MeasureStat::Time(_) => "Time",
        };
        let fmt_time = |dur: Duration| format!("{}:{:02}.{:02}", dur.as_secs()/60, dur.as_secs()%60, dur.as_millis() % 1000 / 10);
        let fmt_key = |key: KeyCode| format!("[{}]", match key {
            KeyCode::Backspace => "BACK".to_string(),
            KeyCode::Enter => "ENTR".to_string(),
            KeyCode::Left => "←".to_string(),
            KeyCode::Right => "→".to_string(),
            KeyCode::Up => "↑".to_string(),
            KeyCode::Down => "↓".to_string(),
            KeyCode::Home => "HOME".to_string(),
            KeyCode::End => "END".to_string(),
            KeyCode::PageUp => "PgUp".to_string(),
            KeyCode::PageDown => "PgDn".to_string(),
            KeyCode::Tab => "TAB".to_string(),
            KeyCode::Delete => "DEL".to_string(),
            KeyCode::F(n) => format!("F{n}"),
            KeyCode::Char(c) => c.to_uppercase().to_string(),
            KeyCode::Esc => "ESC".to_string(),
            _ => "??".to_string(),
        });
        // Screen: some titles.
        let mode_name = gamemode.name.to_ascii_uppercase();
        let mode_name_space = mode_name.len().max(14);
        let opti_name = stat_name(gamemode.optimize);
        let opti_value = match gamemode.optimize {
            MeasureStat::Lines(_) => format!("{}", lines),
            MeasureStat::Level(_) => format!("{}", level),
            MeasureStat::Score(_) => format!("{}", score),
            MeasureStat::Pieces(_) => format!("{}", pieces_played.iter().sum::<u32>()),
            MeasureStat::Time(_) => fmt_time(time_elapsed),
        };
        let (goal_name, goal_value) = if let Some(stat) = gamemode.limit {
            (
                format!("{} left:", stat_name(stat)),
                match stat {
                    MeasureStat::Lines(lns) => format!("{}", lns - lines),
                    MeasureStat::Level(lvl) => format!("{}", lvl.get() - level.get()),
                    MeasureStat::Score(pts) => format!("{}", pts - score),
                    MeasureStat::Pieces(pcs) => format!("{}", pcs - pieces_played.iter().sum::<u32>()),
                    MeasureStat::Time(dur) => fmt_time(dur - time_elapsed),
                },
            )
        } else {
            ("".to_string(), "".to_string())
        };
        let key_icon_pause = fmt_key(KeyCode::Esc);
        let key_icons_moveleft = ctx.settings.keybinds.iter().filter_map(|(&k, &b)| (b==Button::MoveLeft).then_some(fmt_key(k))).collect::<Vec<String>>().join(" ");
        let key_icons_moveright = ctx.settings.keybinds.iter().filter_map(|(&k, &b)| (b==Button::MoveRight).then_some(fmt_key(k))).collect::<Vec<String>>().join(" ");
        let key_icons_move = format!("{key_icons_moveleft} {key_icons_moveright}");
        let key_icons_rotateleft = ctx.settings.keybinds.iter().filter_map(|(&k, &b)| (b==Button::RotateLeft).then_some(fmt_key(k))).collect::<Vec<String>>().join(" ");
        let key_icons_rotateright = ctx.settings.keybinds.iter().filter_map(|(&k, &b)| (b==Button::RotateRight).then_some(fmt_key(k))).collect::<Vec<String>>().join(" ");
        let key_icons_rotate = format!("{key_icons_rotateleft} {key_icons_rotateright}");
        let key_icons_dropsoft = ctx.settings.keybinds.iter().filter_map(|(&k, &b)| (b==Button::DropSoft).then_some(fmt_key(k))).collect::<Vec<String>>().join(" ");
        let key_icons_drophard = ctx.settings.keybinds.iter().filter_map(|(&k, &b)| (b==Button::DropHard).then_some(fmt_key(k))).collect::<Vec<String>>().join(" ");
        let key_icons_drop = format!("{key_icons_dropsoft} {key_icons_drophard}");
        let piececnts_o = format!("{}o", pieces_played[usize::from(Tetromino::O)]);
        let piececnts_i_s_z = [
            format!("{}i", pieces_played[usize::from(Tetromino::I)]),
            format!("{}s", pieces_played[usize::from(Tetromino::S)]),
            format!("{}z", pieces_played[usize::from(Tetromino::Z)]),
        ].join("  ");
        let piececnts_t_l_j = [
            format!("{}t", pieces_played[usize::from(Tetromino::T)]),
            format!("{}l", pieces_played[usize::from(Tetromino::L)]),
            format!("{}j", pieces_played[usize::from(Tetromino::J)]),
        ].join("  ");
        // Screen: draw.
        let mut screen = Vec::new();
        #[allow(clippy::useless_format)]
        {
            screen.push(format!("                                                             ", ));
            screen.push(format!("                        ╓╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╥{:─^w$       }┐", "mode", w=mode_name_space));
            screen.push(format!("     ALL STATS          ║                    ║{: ^w$       }│", mode_name, w=mode_name_space));
            screen.push(format!("     ─────────╴         ║                    ╟{:─^w$       }┘", "", w=mode_name_space));
            screen.push(format!("     Level:{:>7  }      ║                    ║  {          }:", level, opti_name));
            screen.push(format!("     Score:{:>7  }      ║                    ║{:^15         }", score, opti_value));
            screen.push(format!("     Lines:{:>7  }      ║                    ║               ", lines));
            screen.push(format!("                        ║                    ║  {           }", goal_name));
            screen.push(format!("     Time elapsed       ║                    ║{:^15         }", goal_value));
            screen.push(format!("     {:>13       }      ║                    ║               ", fmt_time(time_elapsed)));
            screen.push(format!("                        ║                    ║─────next─────┐", ));
            screen.push(format!("     PIECES             ║                    ║              │", ));
            screen.push(format!("     ──────╴            ║                    ║              │", ));
            screen.push(format!("     {:<19             }║                    ║──────────────┘", piececnts_o));
            screen.push(format!("     {:<19             }║                    ║               ", piececnts_i_s_z));
            screen.push(format!("     {:<19             }║                    ║               ", piececnts_t_l_j));
            screen.push(format!("                        ║                    ║               ", ));
            screen.push(format!("     CONTROLS           ║                    ║               ", ));
            screen.push(format!("     ────────╴          ║                    ║               ", ));
            screen.push(format!("     Pause   {:<11     }║                    ║               ", key_icon_pause));
            screen.push(format!("     Move    {:<11     }║                    ║               ", key_icons_move));
            screen.push(format!("     Rotate  {:<11     }║                    ║               ", key_icons_rotate));
            screen.push(format!("     Drop    {:<11     }╚════════════════════╝               ", key_icons_drop));
            screen.push(format!("                                                             ", ));
        }
        // Begin frame update.
        ctx.term.queue(terminal::BeginSynchronizedUpdate)?
            .queue(terminal::Clear(terminal::ClearType::All))?;
        for (screen_y, str) in screen.iter().enumerate() {
            ctx.term
                .queue(cursor::MoveTo(w_x, w_y+ u16::try_from(screen_y).unwrap()))?
                .queue(Print(str))?;
        }
        // Board: helpers.
        // TODO: Old tile colors.
        let _tile_color = |tile: TileTypeID| match tile.get() {
            1 => Color::Yellow,
            2 => Color::Cyan,
            3 => Color::Green,
            4 => Color::Red,
            5 => Color::DarkMagenta,
            6 => Color::DarkYellow,
            7 => Color::Blue,
            t => unimplemented!("formatting unknown tile id {t}"),
        };
        let tile_color = |tile: TileTypeID| match tile.get() {
            1 => Color::Rgb { r:254, g:203, b:  0 },
            2 => Color::Rgb { r:  0, g:159, b:218 },
            3 => Color::Rgb { r:105, g:190, b: 40 },
            4 => Color::Rgb { r:237, g: 41, b: 57 },
            5 => Color::Rgb { r:149, g: 45, b:152 },
            6 => Color::Rgb { r:255, g:121, b:  0 },
            7 => Color::Rgb { r:  0, g:101, b:189 },
            t => unimplemented!("formatting unknown tile id {t}"),
        };
        let (board_x, board_y) = (25, 1);
        let move_to = |(x, y): Coord| MoveTo(w_x+ board_x + 2 * u16::try_from(x).unwrap(), w_y+ board_y + u16::try_from(Game::SKYLINE - y).unwrap());
        // Board: draw hard drop trail.
        for (event_time, pos, h, tile_type_id, relevant) in self.hard_drop_tiles.iter_mut() {
            // TODO: Hard drop animation polish.
            let elapsed = time_updated.saturating_duration_since(*event_time);
            let luminance_map = "@$#%*+~.".as_bytes();
            // TODO: Old hard drop animation timings.
            // let Some(&char) = [50, 60, 70, 80, 90, 110, 140, 180]
            let Some(&char) = [50, 70, 90, 110, 130, 150, 180, 240]
                .iter()
                .enumerate()
                .find_map(|(idx, ms)| (elapsed < Duration::from_millis(*ms)).then_some(idx))
                .and_then(|dt| luminance_map.get(*h/2 + dt))
            else {
                *relevant = false;
                continue;
            };
            // SAFETY: Valid ASCII bytes.
            let tile = String::from_utf8(vec![char, char]).unwrap();
            ctx.term.queue(move_to(*pos))?.queue(PrintStyledContent(tile.with(tile_color(*tile_type_id))))?;
        }
        self.hard_drop_tiles.retain(|elt| elt.4);
        // Board: draw fixed tiles.
        for (y, line) in board.iter().enumerate().take(21).rev() {
            for (x, cell) in line.iter().enumerate() {
                if let Some(tile_type_id) = cell {
                    ctx.term.queue(move_to((x,y)))?.queue(PrintStyledContent("██".with(tile_color(*tile_type_id))))?;
                }
            }
        }
        // If a piece is in play.
        if let Some(active_piece) = active_piece {
            // Draw ghost piece.
            for (pos, tile_type_id) in active_piece.well_piece(board).tiles() {
                if pos.1 <= Game::SKYLINE {
                    ctx.term.queue(move_to(pos))?.queue(PrintStyledContent("░░".with(tile_color(tile_type_id))))?;
                }
            }
            // Draw active piece.
            for (pos, tile_type_id) in active_piece.tiles() {
                if pos.1 <= Game::SKYLINE {
                    ctx.term.queue(move_to(pos))?.queue(PrintStyledContent("▓▓".with(tile_color(tile_type_id))))?;
                }
            }
        }
        // Draw preview.
        // TODO: Larger preview.
        if game.config().preview_count > 0 {
            let (preview_x, preview_y) = (49, 12);
            // SAFETY: `preview_count > 0`.
            let next_piece = next_pieces.front().unwrap();
            let color = tile_color(next_piece.tiletypeid());
            for (x, y) in next_piece.minos(tetrs_lib::Orientation::N) {
                // SAFETY: We will not exceed the bounds by drawing pieces.
                ctx.term
                    .queue(MoveTo(w_x + preview_x + u16::try_from(2*x).unwrap(), w_y + preview_y - u16::try_from(y).unwrap()))?
                    .queue(PrintStyledContent("▒▒".with(color)))?;
            }
        }
        // Update stored events.
        self.events.extend(new_feedback_events.into_iter().map(|(time,event)| (time,event,true)));
        // Draw events.
        for (event_time, event, relevant) in self.events.iter_mut().rev() {
            match event {
                FeedbackEvent::PieceLocked(piece) => {
                    // TODO: Polish locking animation?
                    let elapsed = time_updated.saturating_duration_since(*event_time);
                    let Some(tile) = [(50,"██"), (75,"▓▓"), (100,"▒▒"), (125,"░░"), (150,"▒▒"), (175,"▓▓")]
                        .iter().find_map(|(ms, tile)| (elapsed < Duration::from_millis(*ms)).then_some(tile))
                    else {
                        *relevant = false;
                        continue;
                    };
                    for (pos, _tile_type_id) in piece.tiles() {
                        if pos.1 <= Game::SKYLINE {
                            ctx.term.queue(move_to(pos))?.queue(PrintStyledContent(tile.with(Color::White)))?;
                        }
                    }
                }
                FeedbackEvent::LineClears(lines_cleared, line_clear_delay) => {
                    // TODO: Polish line clear animation?
                    if line_clear_delay.is_zero() {
                        *relevant = false;
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
                    let percent = time_updated.saturating_duration_since(*event_time).as_secs_f64() / line_clear_delay.as_secs_f64();
                    // SAFETY: `0.0 <= percent && percent <= 1.0`.
                    let idx = if percent < 1.0 { unsafe { (10.0 * percent).to_int_unchecked::<usize>() } } else {
                        *relevant = false;
                        continue;
                    };
                    for line_y in lines_cleared {
                        ctx.term
                            .queue(MoveTo(w_x+ board_x, w_y+ board_y + u16::try_from(Game::SKYLINE - *line_y).unwrap()))?
                            .queue(PrintStyledContent(line_clear_frames[idx].with(Color::White)))?;
                    }
                }
                FeedbackEvent::HardDrop(_top_piece, bottom_piece) => {
                    for ((x,y), tile_type_id) in bottom_piece.tiles() {
                        for y2 in y..Game::SKYLINE {
                            self.hard_drop_tiles.push((*event_time, (x,y2), y2-y, tile_type_id, true));
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
                    opportunity
                } => {
                    let mut strs = Vec::new();
                    strs.push(format!("+{score_bonus}"));
                    if *perfect_clear {
                        strs.push("PERFECT".to_string());
                    }
                    if *spin {
                        strs.push(format!("{shape:?}-Spin"));
                    }
                    let accolade = match lineclears {
                        1 => "Single",
                        2 => "Double",
                        3 => "Triple",
                        4 => "Quadruple",
                        x => unreachable!("unexpected line clear count {x}"),
                    }.to_ascii_uppercase();
                    let excl = match opportunity {
                        1 => "'",
                        2 => "!",
                        3 => "!'",
                        4 => "!!",
                        x => unreachable!("unexpected opportunity count {x}"),
                    };
                    strs.push(format!("{accolade}{excl}"));
                    if *combo > 1 {
                        strs.push(format!("[{combo}.combo]"));
                    }
                    self.accolades.push((*event_time, strs.join(" ")));
                    *relevant = false;
                }
                // TODO: Proper Debug?...
                FeedbackEvent::Debug(msg) => {
                    ctx.term
                        .queue(MoveTo(w_x, w_y+ 24))?
                        .queue(Print(msg))?;
                    if time_updated.saturating_duration_since(*event_time) > Duration::from_secs(20) {
                        *relevant = false;
                    }
                }
            }
        }
        self.events.retain(|elt| elt.2);
        // Draw accolades.
        let (accolade_x, accolade_y) = (48, 15);
        for (dy, (_event_time, accolade)) in self.accolades.iter().enumerate() {
            ctx.term
                .queue(MoveTo(w_x+ accolade_x, w_y+ accolade_y + u16::try_from(dy).expect("too many accolades")))?
                .queue(Print(accolade))?;
        }
        self.accolades.retain(|(event_time, _accolade)| time_updated.saturating_duration_since(*event_time) < Duration::from_millis(6000));
        // Execute draw.
        ctx.term.queue(terminal::EndSynchronizedUpdate)?;
        ctx.term.flush()?;
        Ok(())
    }
}
