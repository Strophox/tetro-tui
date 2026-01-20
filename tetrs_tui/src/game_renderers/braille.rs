use std::{
    collections::VecDeque,
    io::{self, Write},
};

use crossterm::{
    cursor::{self, MoveToNextLine},
    style::Print,
    terminal, QueueableCommand,
};
use tetrs_engine::{Feedback, FeedbackMessages, Game, GameTime, State};

use crate::{
    application::{Application, GameMetaData},
    game_renderers::Renderer,
};

// "|⠁|⠂|⠄|⠈|⠐|⠠|⡀|⢀|"
const BRAILLE: &str = "⠀⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯⠰⠱⠲⠳⠴⠵⠶⠷⠸⠹⠺⠻⠼⠽⠾⠿⡀⡁⡂⡃⡄⡅⡆⡇⡈⡉⡊⡋⡌⡍⡎⡏⡐⡑⡒⡓⡔⡕⡖⡗⡘⡙⡚⡛⡜⡝⡞⡟⡠⡡⡢⡣⡤⡥⡦⡧⡨⡩⡪⡫⡬⡭⡮⡯⡰⡱⡲⡳⡴⡵⡶⡷⡸⡹⡺⡻⡼⡽⡾⡿⢀⢁⢂⢃⢄⢅⢆⢇⢈⢉⢊⢋⢌⢍⢎⢏⢐⢑⢒⢓⢔⢕⢖⢗⢘⢙⢚⢛⢜⢝⢞⢟⢠⢡⢢⢣⢤⢥⢦⢧⢨⢩⢪⢫⢬⢭⢮⢯⢰⢱⢲⢳⢴⢵⢶⢷⢸⢹⢺⢻⢼⢽⢾⢿⣀⣁⣂⣃⣄⣅⣆⣇⣈⣉⣊⣋⣌⣍⣎⣏⣐⣑⣒⣓⣔⣕⣖⣗⣘⣙⣚⣛⣜⣝⣞⣟⣠⣡⣢⣣⣤⣥⣦⣧⣨⣩⣪⣫⣬⣭⣮⣯⣰⣱⣲⣳⣴⣵⣶⣷⣸⣹⣺⣻⣼⣽⣾⣿";

#[allow(dead_code)]
#[derive(Clone, Default, Debug)]
pub struct BrailleRenderer {
    feedback_msgs_buffer: VecDeque<(GameTime, Feedback)>,
}

impl Renderer for BrailleRenderer {
    fn render<T: Write>(
        &mut self,
        app: &mut Application<T>,
        game: &Game,
        _meta_data: &GameMetaData,
        _new_feedback_msgs: FeedbackMessages,
        _screen_resized: bool,
    ) -> io::Result<()> {
        let State {
            board,
            active_piece_data,
            ..
        } = game.state();

        let mut bd = board.clone();
        if let Some((active_piece, _)) = active_piece_data {
            for ((x, y), tile_type_id) in active_piece.tiles() {
                bd[y][x] = Some(tile_type_id);
            }
        }

        let braille = BRAILLE.chars().collect::<Vec<char>>();

        app.term
            .queue(cursor::MoveTo(0, 0))?
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;

        let btxt_lines = [
            [19, 18, 17, 16],
            [15, 14, 13, 12],
            [11, 10, 9, 8],
            [7, 6, 5, 4],
            [3, 2, 1, 0],
        ]
        .iter()
        .map(|[i0, i1, i2, i3]| {
            let [l0, l1, l2, l3] = [bd[*i0], bd[*i1], bd[*i2], bd[*i3]];
            [[0, 1], [2, 3], [4, 5], [6, 7], [8, 9]]
                .iter()
                .map(|[j0, j1]| {
                    let b0 = if l0[*j0].is_some() { 1 } else { 0 };
                    let b1 = if l1[*j0].is_some() { 2 } else { 0 };
                    let b2 = if l2[*j0].is_some() { 4 } else { 0 };
                    let b3 = if l3[*j0].is_some() { 64 } else { 0 };
                    let b4 = if l0[*j1].is_some() { 8 } else { 0 };
                    let b5 = if l1[*j1].is_some() { 16 } else { 0 };
                    let b6 = if l2[*j1].is_some() { 32 } else { 0 };
                    let b7 = if l3[*j1].is_some() { 128 } else { 0 };
                    braille[b0 + b1 + b2 + b3 + b4 + b5 + b6 + b7]
                })
                .collect::<String>()
        });
        for b_line in btxt_lines {
            app.term
                .queue(Print(format!("|{b_line}|")))?
                .queue(MoveToNextLine(1))?;
        }

        app.term.flush()?;
        Ok(())
    }
}
