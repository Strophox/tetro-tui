use std::{
    collections::VecDeque,
    io::{self, Write},
    time::Instant,
};

use crossterm::{cursor, style, terminal, QueueableCommand};
use tetrs_lib::{FeedbackEvent, Game, GameStateView};

use crate::terminal_tetrs::TerminalTetrs;

#[derive(Eq, PartialEq, Clone, Hash, Default, Debug)]
pub struct GameRenderer {
    feedback_event_buffer: VecDeque<(Instant, FeedbackEvent)>,
}

impl GameRenderer {
    pub fn render(
        &mut self,
        ctx: &mut TerminalTetrs<impl Write>,
        game: &mut Game,
        new_feedback_events: Vec<(Instant, FeedbackEvent)>,
    ) -> io::Result<()> {
        let (WIDTH, HEIGHT) = terminal::size()?;
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
        Ok(())
    }

    pub fn render_dbg(
        &mut self,
        ctx: &mut TerminalTetrs<impl Write>,
        game: &mut Game,
        new_feedback_events: Vec<(Instant, FeedbackEvent)>,
    ) -> io::Result<()> {
        // Draw game stuf
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
        let mut temp_board = board.clone();
        if let Some(active_piece) = active_piece {
            for ((x, y), tile_type_id) in active_piece.tiles() {
                temp_board[y][x] = Some(tile_type_id);
            }
        }
        ctx.term
            .queue(cursor::MoveTo(0, 0))?
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;
        ctx.term
            .queue(style::Print("   +--------------------+"))?
            .queue(cursor::MoveToNextLine(1))?;
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
                            _ => todo!("formatting unknown tile type"),
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("")
            );
            ctx.term
                .queue(style::Print(txt_line))?
                .queue(cursor::MoveToNextLine(1))?;
        }
        ctx.term
            .queue(style::Print("   +--------------------+"))?
            .queue(cursor::MoveToNextLine(1))?;
        ctx.term
            .queue(style::Print(format!(
                "   {:?}",
                time_updated.saturating_duration_since(game.state().time_started)
            )))?
            .queue(cursor::MoveToNextLine(1))?;
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
                        strs.push(format!("{shape:?}-Spin"))
                    }
                    let accolade = match lineclears {
                        1 => "Single",
                        2 => "Double",
                        3 => "Triple",
                        4 => "Quadruple",
                        x => todo!("unexpected line clear count {}", x),
                    };
                    let excl = match opportunity {
                        1 => "'",
                        2 => "!",
                        3 => "!'",
                        4 => "!!",
                        x => todo!("unexpected opportunity count {}", x),
                    };
                    strs.push(format!("{accolade}{excl}"));
                    if *combo > 1 {
                        strs.push(format!("[{combo}.combo]"));
                    }
                    if *perfect_clear {
                        strs.push("PERFECT!".to_string());
                    }
                    strs.join(" ")
                }
                FeedbackEvent::PieceLocked(_) => continue,
                FeedbackEvent::LineClears(..) => continue,
                FeedbackEvent::HardDrop(_, _) => continue,
                FeedbackEvent::Debug(s) => s.clone(),
            });
        }
        for str in feed_evt_msgs.iter().take(16) {
            ctx.term
                .queue(style::Print(str))?
                .queue(cursor::MoveToNextLine(1))?;
        }
        // Execute draw.
        ctx.term.flush()?;
        Ok(())
    }
}
