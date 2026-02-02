use std::num::NonZeroU8;

use tetrs_engine::{Board, Game, GameOver, Line, Modifier, Phase, Stat, Tetromino, UpdatePoint};

pub const MOD_ID: &str = "combo_board";

pub const LAYOUTS: [u16; 5] = [
    0b0000_0000_1100_1000, // "r "
    0b0000_0000_0000_1110, // "_ "
    0b0000_1100_1000_1011, // "f _"
    0b0000_1100_1000_1101, // "k ."
    0b1000_1000_1000_1101, // "L ."
                           /*0b0000_1001_1001_1001, // "I I"
                           0b0001_0001_1001_1100, // "l i"
                           0b1000_1000_1100_1100, // "b"
                           0b0000_0000_1110_1011, // "rl"*/
];

pub fn modifier(initial_layout: u16) -> Modifier {
    let mut line_source = four_wide_lines();
    let mut init = false;
    Modifier {
        descriptor: format!(
            "{MOD_ID}\n{}",
            serde_json::to_string(&initial_layout).unwrap()
        ),
        mod_function: Box::new(move |point, _config, _init_vals, state, phase, _msgs| {
            if !init {
                for (line, four_well) in state
                    .board
                    .iter_mut()
                    .take(Game::HEIGHT)
                    .zip(&mut line_source)
                {
                    *line = four_well;
                }
                init_board(&mut state.board, initial_layout);
                init = true;
            } else if matches!(point, UpdatePoint::PieceLocked)
                && !matches!(phase, Phase::LinesClearing { .. })
            {
                *phase = Phase::GameEnded {
                    result: Err(GameOver::Limit(Stat::LinesCleared(0))),
                };
            // Combo continues, prepare new line.
            } else if matches!(point, UpdatePoint::LinesCleared) {
                state.board[Game::HEIGHT - 1] = line_source.next().unwrap();
            }
        }),
    }
}

fn init_board(board: &mut Board, mut init_layout: u16) {
    let grey_tile = Some(NonZeroU8::try_from(254).unwrap());
    let mut y = 0;
    while init_layout != 0 {
        if init_layout & 0b1000 != 0 {
            board[y][3] = grey_tile;
        }
        if init_layout & 0b0100 != 0 {
            board[y][4] = grey_tile;
        }
        if init_layout & 0b0010 != 0 {
            board[y][5] = grey_tile;
        }
        if init_layout & 0b0001 != 0 {
            board[y][6] = grey_tile;
        }
        init_layout /= 0b1_0000;
        y += 1;
    }
}

fn four_wide_lines() -> impl Iterator<Item = Line> {
    let color_tiles = [
        Tetromino::Z,
        Tetromino::L,
        Tetromino::O,
        Tetromino::S,
        Tetromino::I,
        Tetromino::J,
        Tetromino::T,
    ]
    .map(|tet| Some(tet.tiletypeid()));
    let grey_tile = Some(NonZeroU8::try_from(254).unwrap());
    let indices_0 = (0..).map(|i| i % 7);
    let indices_1 = indices_0.clone().skip(1);
    indices_0.zip(indices_1).map(move |(i_0, i_1)| {
        let mut line = [None; Game::WIDTH];
        line[0] = color_tiles[i_0];
        line[1] = color_tiles[i_1];
        line[2] = grey_tile;
        line[7] = grey_tile;
        line[8] = color_tiles[i_1];
        line[9] = color_tiles[i_0];
        line
    })
}
