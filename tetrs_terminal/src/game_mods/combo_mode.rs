use std::num::{NonZeroU32, NonZeroU8};

use tetrs_engine::{
    FeedbackEvents, FnGameMod, Game, GameConfig, GameMode, GameState, InternalEvent, Limits, Line,
    ModifierPoint, Tetromino,
};

pub fn four_well_lines() -> impl Iterator<Item = Line> {
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
    let indices_0 = (0..).map(|i| i % 7);
    let indices_1 = indices_0.clone().skip(1);
    let indices_2 = indices_0.clone().skip(2);
    indices_0
        .zip(indices_1)
        .zip(indices_2)
        .map(move |((i_0, i_1), i_2)| {
            let mut line = [None; 10];
            line[0] = color_tiles[i_0];
            line[1] = color_tiles[i_1];
            line[2] = color_tiles[i_2];
            line[7] = color_tiles[i_2];
            line[8] = color_tiles[i_1];
            line[9] = color_tiles[i_0];
            line
        })
}

pub fn new_game() -> Game {
    let mut line_source = four_well_lines();
    let grey_tile = Some(NonZeroU8::try_from(254).unwrap());
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
                state.board[0][3] = grey_tile;
                state.board[1][3] = grey_tile;
                state.board[1][4] = grey_tile;
                init = true;
            } else if matches!(
                modifier_point,
                ModifierPoint::AfterEvent(InternalEvent::Lock)
            ) {
                // No lineclear, game over.
                if !state.events.contains_key(&InternalEvent::LineClear) {
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
        start_level: NonZeroU32::MIN,
        increment_level: false,
        limits: Limits::default(),
    });
    unsafe { game.add_modifier(combo_mode) };
    game
}
