use std::collections::VecDeque;

use crossterm::{
    QueueableCommand,
    cursor::{self, MoveToNextLine},
    style::{self, Print},
    terminal,
};

use crate::tetromino_engine::{InGameTime, Notification, State, Tetromino};

use super::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Default)]
pub struct PrototypeRenderer {
    notification_feed_buffer: VecDeque<(Notification, InGameTime)>,
}

impl Renderer for PrototypeRenderer {
    fn update_feed(
        &mut self,
        feed: impl IntoIterator<Item = (Notification, InGameTime)>,
        _settings: &Settings,
    ) {
        for x in feed {
            self.notification_feed_buffer.push_front(x);
        }
    }

    fn reset_veffects_state(&mut self) {
        self.notification_feed_buffer.clear();
    }

    fn reset_viewport_state_with_offset_and_area(
        &mut self,
        _offsets: (u16, u16),
        _dimensions: (u16, u16),
    ) {
        // We do not implement any special viewport handling here.
    }

    fn render<T>(
        &mut self,
        term: &mut T,
        game: &Game,
        _meta_data: &GameMetaData,
        _settings: &Settings,
        _temp_data: &TemporaryAppData,
        _keybinds_legend: &KeybindsLegend,
        _replay_extra: Option<(InGameTime, f64)>,
    ) -> io::Result<()>
    where
        T: Write,
    {
        // Draw game stuf
        let game_time = game.state().time;
        let mut board = game.state().board.clone();
        if let Some(piece) = game.phase().piece() {
            for (x, y) in piece.coords() {
                board[y as usize].0[x as usize] = Some(piece.tetromino.into());
            }
        }
        term.queue(cursor::MoveTo(0, 0))?
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;
        term.queue(Print("   +--------------------+"))?
            .queue(MoveToNextLine(1))?;
        for (idx, (line, _is_frozen)) in board.iter().take(20).enumerate().rev() {
            let txt_line = format!(
                "{idx:02} |{}|",
                line.iter()
                    .map(|cell| {
                        cell.map_or(" .", |tile| match tile {
                            crate::tetromino_engine::TileType::Tet(t) => match t {
                                Tetromino::O => "OO",
                                Tetromino::I => "II",
                                Tetromino::S => "SS",
                                Tetromino::Z => "ZZ",
                                Tetromino::T => "TT",
                                Tetromino::L => "LL",
                                Tetromino::J => "JJ",
                            },
                            crate::tetromino_engine::TileType::Generic => "WW",
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("")
            );
            term.queue(Print(txt_line))?.queue(MoveToNextLine(1))?;
        }
        term.queue(Print("   +--------------------+"))?
            .queue(MoveToNextLine(1))?;
        term.queue(style::Print(format!("   {:?}", game_time)))?
            .queue(MoveToNextLine(1))?;

        // Draw feedback stuf
        let mut feed_evt_msgs = Vec::new();
        for (notif, _) in self.notification_feed_buffer.iter() {
            feed_evt_msgs.push(match notif {
                Notification::Accolade {
                    point_bonus,
                    tetromino,
                    is_spin,
                    lineclears,
                    is_perfect,
                    combo,
                } => {
                    let mut msg = Vec::new();
                    msg.push(format!("+{point_bonus}"));
                    if *is_perfect {
                        msg.push("Perfect".to_owned());
                    }
                    if *is_spin {
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
                        _ => "Absurduple",
                    }
                    .to_string();
                    msg.push(clear_action);
                    if *combo > 1 {
                        msg.push(format!("#{combo}."));
                    }
                    msg.join(" ")
                }

                feedback => format!("{feedback:?}"),
            });
        }

        for str in feed_evt_msgs.iter().take(16) {
            term.queue(Print(str))?.queue(MoveToNextLine(1))?;
        }

        // Execute draw.
        term.flush()?;

        Ok(())
    }
}
