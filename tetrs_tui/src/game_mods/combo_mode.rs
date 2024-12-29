use std::num::NonZeroU8;

use tetrs_engine::{
    Board, FeedbackEvents, FnGameMod, Game, GameConfig, GameEvent, GameMode, GameState, Limits,
    Line, ModifierPoint, Tetromino,
};

pub const LAYOUTS: [u16; 5] = [
    0b0000_0000_1100_1000, // "r"
    0b0000_0000_0000_1110, // "_"
    0b0000_1100_1000_1011, // "f _"
    0b0000_1100_1000_1101, // "k ."
    0b1000_1000_1000_1101, // "L ."
                           /*0b0000_1001_1001_1001, // "I I"
                           0b0001_0001_1001_1100, // "l i"
                           0b1000_1000_1100_1100, // "b"
                           0b0000_0000_1110_1011, // "rl"*/
];

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

pub fn new_game(gravity: u32, initial_layout: u16) -> Game {
    let mut line_source = four_wide_lines();
    let mut init = false;
    let combo_mode: FnGameMod = Box::new(
        move |_config: &mut GameConfig,
              _mode: &mut GameMode,
              state: &mut GameState,
              _feedback_events: &mut FeedbackEvents,
              modifier_point: &ModifierPoint| {
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
            } else if matches!(modifier_point, ModifierPoint::AfterEvent(GameEvent::Lock)) {
                // No lineclear, game over.
                if !state.events.contains_key(&GameEvent::LineClear) {
                    state.end = Some(Err(tetrs_engine::GameOver::ModeLimit));
                // Combo continues, prepare new line.
                } else {
                    state.board.push(line_source.next().unwrap());
                }
            }
        },
    );
    let mut game = Game::new(GameMode {
        name: "Combo".to_string(),
        initial_gravity: gravity,
        increase_gravity: false,
        limits: Limits::default(),
    });
    game.add_modifier(combo_mode);
    game
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
