use std::{
    collections::VecDeque,
    io::{self, Write},
};

use crossterm::{
    cursor::{self, MoveToNextLine},
    style::{self, Print},
    terminal, QueueableCommand,
};
use tetrs_engine::{Feedback, FeedbackMessages, Game, GameTime, State};

use crate::{
    application::{Application, GameMetaData},
    game_renderers::Renderer,
};

#[allow(dead_code)]
#[derive(Clone, Default, Debug)]
pub struct DebugRenderer {
    feedback_msgs_buffer: VecDeque<(GameTime, Feedback)>,
}

impl Renderer for DebugRenderer {
    fn render<T>(
        &mut self,
        app: &mut Application<T>,
        game: &Game,
        _meta_data: &GameMetaData,
        new_feedback_msgs: FeedbackMessages,
        _screen_resized: bool,
    ) -> io::Result<()>
    where
        T: Write,
    {
        // Draw game stuf
        let State {
            time: game_time,
            board,
            ..
        } = game.state();
        let mut board = *board;
        if let tetrs_engine::Phase::PieceInPlay {
            piece_data: tetrs_engine::PieceData { piece, .. },
            ..
        } = game.phase()
        {
            for ((x, y), tile_type_id) in piece.tiles() {
                board[y][x] = Some(tile_type_id);
            }
        }
        app.term
            .queue(cursor::MoveTo(0, 0))?
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;
        app.term
            .queue(Print("   +--------------------+"))?
            .queue(MoveToNextLine(1))?;
        for (idx, line) in board.iter().take(20).enumerate().rev() {
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
                            253 => "WW",
                            254 => "WW",
                            255 => "WW",
                            t => unimplemented!("formatting unknown tile id {t}"),
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("")
            );
            app.term.queue(Print(txt_line))?.queue(MoveToNextLine(1))?;
        }
        app.term
            .queue(Print("   +--------------------+"))?
            .queue(MoveToNextLine(1))?;
        app.term
            .queue(style::Print(format!("   {:?}", game_time)))?
            .queue(MoveToNextLine(1))?;
        // Draw feedback stuf
        for evt in new_feedback_msgs {
            self.feedback_msgs_buffer.push_front(evt);
        }
        let mut feed_evt_msgs = Vec::new();
        for (_, feedback) in self.feedback_msgs_buffer.iter() {
            feed_evt_msgs.push(match feedback {
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
                    msg.join(" ")
                }
                Feedback::PieceLocked { .. } => continue,
                Feedback::LinesClearing { .. } => continue,
                Feedback::HardDrop { .. } => continue,
                Feedback::Debug(update_point) => format!("{update_point:?}"),
                Feedback::Text(s) => s.clone(),
            });
        }
        for str in feed_evt_msgs.iter().take(16) {
            app.term.queue(Print(str))?.queue(MoveToNextLine(1))?;
        }
        // Execute draw.
        app.term.flush()?;
        Ok(())
    }
}
