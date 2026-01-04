use std::{
    collections::VecDeque,
    io::{self, Write},
};

use crossterm::{
    cursor::{self, MoveToNextLine},
    style::{self, Print},
    terminal, QueueableCommand,
};
use tetrs_engine::{Feedback, FeedbackEvents, Game, GameState, GameTime};

use crate::{
    game_renderers::Renderer,
    terminal_user_interface::{Application, RunningGameStats},
};

#[allow(dead_code)]
#[derive(Clone, Default, Debug)]
pub struct DebugRenderer {
    feedback_event_buffer: VecDeque<(GameTime, Feedback)>,
}

impl Renderer for DebugRenderer {
    fn render<T>(
        &mut self,
        app: &mut Application<T>,
        _running_game_stats: &mut RunningGameStats,
        game: &Game,
        new_feedback_events: FeedbackEvents,
        _screen_resized: bool,
    ) -> io::Result<()>
    where
        T: Write,
    {
        // Draw game stuf
        let GameState {
            time: game_time,
            board,
            active_piece_data,
            ..
        } = game.state();
        let mut temp_board = board.clone();
        if let Some((active_piece, _)) = active_piece_data {
            for ((x, y), tile_type_id) in active_piece.tiles() {
                temp_board[y][x] = Some(tile_type_id);
            }
        }
        app.term
            .queue(cursor::MoveTo(0, 0))?
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;
        app.term
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
        for evt in new_feedback_events {
            self.feedback_event_buffer.push_front(evt);
        }
        let mut feed_evt_msgs = Vec::new();
        for (_, feedback_event) in self.feedback_event_buffer.iter() {
            feed_evt_msgs.push(match feedback_event {
                Feedback::Accolade {
                    score_bonus,
                    shape,
                    spin,
                    lineclears,
                    perfect_clear,
                    combo,
                    back_to_back,
                } => {
                    let mut strs = Vec::new();
                    if *spin {
                        strs.push(format!("{shape:?}-Spin"));
                    }
                    let clear_action = match lineclears {
                        1 => "Single",
                        2 => "Double",
                        3 => "Triple",
                        4 => "Quadruple",
                        x => unreachable!("unexpected line clear count {x}"),
                    }
                    .to_string();
                    if *back_to_back > 1 {
                        strs.push(format!("{back_to_back}-B2B"));
                    }
                    strs.push(clear_action);
                    if *combo > 1 {
                        strs.push(format!("[{combo}.combo]"));
                    }
                    if *perfect_clear {
                        strs.push("PERFECT!".to_string());
                    }
                    strs.push(format!("+{score_bonus}"));
                    strs.join(" ")
                }
                Feedback::PieceSpawned(_) => continue,
                Feedback::PieceLocked(_) => continue,
                Feedback::LineClears(..) => continue,
                Feedback::HardDrop(_, _) => continue,
                Feedback::Message(s) => s.clone(),
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
