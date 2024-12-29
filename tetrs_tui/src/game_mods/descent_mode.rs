use std::{num::NonZeroU8, time::Duration};

use rand::{self, Rng};

use tetrs_engine::{
    FeedbackEvents, FnGameMod, Game, GameConfig, GameEvent, GameMode, GameState, GameTime, Limits,
    Line, ModifierPoint, Tetromino,
};

pub fn random_descent_lines() -> impl Iterator<Item = Line> {
    /*
    We generate quadruple sets of lines like this:
             X
    0O0O O0O0X
     */
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
    let playing_width = Game::WIDTH - (1 - Game::WIDTH % 2);
    let mut rng = rand::thread_rng();
    (0..).map(move |i| {
        let mut line = [None; Game::WIDTH];
        match i % 4 {
            0 | 2 => {}
            r => {
                for (j, cell) in line.iter_mut().enumerate() {
                    if j % 2 == 1 || (r == 1 && rng.gen_bool(0.5)) {
                        *cell = grey_tile;
                    }
                }
                // Make hole if row became completely closed off through rng.
                if line.iter().all(|c| c.is_some()) {
                    let hole_idx = 2 * rng.gen_range(0..playing_width / 2);
                    line[hole_idx] = None;
                }
                let gem_idx = rng.gen_range(0..playing_width);
                if line[gem_idx].is_some() {
                    line[gem_idx] = Some(NonZeroU8::try_from(rng.gen_range(1..=7)).unwrap());
                }
            }
        };
        if playing_width < line.len() {
            line[playing_width] = color_tiles[(i / 10) % 7];
        }
        line
    })
}

pub fn new_game() -> Game {
    let mut line_source = random_descent_lines();
    let descent_tetromino = if rand::thread_rng().gen_bool(0.5) {
        Tetromino::L
    } else {
        Tetromino::J
    };
    let mut instant_last_descent = GameTime::ZERO;
    let base_descent_period = Duration::from_secs(2_000_000);
    let mut instant_camera_adjusted = instant_last_descent;
    let camera_adjust_period = Duration::from_millis(125);
    let mut depth = 1u32;
    let mut init = false;
    let descent_mode: FnGameMod = Box::new(
        move |config: &mut GameConfig,
              _mode: &mut GameMode,
              state: &mut GameState,
              _feedback_events: &mut FeedbackEvents,
              modifier_point: &ModifierPoint| {
            if !init {
                for (line, worm_line) in state
                    .board
                    .iter_mut()
                    .take(Game::SKYLINE)
                    .rev()
                    .zip(&mut line_source)
                {
                    *line = worm_line;
                }
                init = true;
            }
            let Some((active_piece, _)) = &mut state.active_piece_data else {
                return;
            };
            let descent_period_elapsed = state.time.saturating_sub(instant_last_descent)
                >= base_descent_period.div_f64(f64::from(depth).powf(1.0 / 2.5));
            let camera_adjust_elapsed =
                state.time.saturating_sub(instant_camera_adjusted) >= camera_adjust_period;
            let camera_hit_bottom = active_piece.position.1 <= 1;
            if descent_period_elapsed || (camera_hit_bottom && camera_adjust_elapsed) {
                if descent_period_elapsed {
                    instant_last_descent = state.time;
                }
                instant_camera_adjusted = state.time;
                depth += 1;
                active_piece.position.1 += 1;
                state.board.insert(0, line_source.next().unwrap());
                state.board.pop();
                if active_piece.position.1 >= Game::SKYLINE {
                    state.end = Some(Err(tetrs_engine::GameOver::ModeLimit));
                }
            }
            if matches!(
                modifier_point,
                ModifierPoint::AfterEvent(GameEvent::Rotate(_))
            ) {
                let piece_tiles_coords = active_piece.tiles().map(|(coord, _)| coord);
                for (y, line) in state.board.iter_mut().enumerate() {
                    for (x, tile) in line.iter_mut().take(9).enumerate() {
                        if let Some(tiletypeid) = tile {
                            let i = tiletypeid.get();
                            if i <= 7 {
                                let j = if piece_tiles_coords
                                    .iter()
                                    .any(|(x_p, y_p)| x_p.abs_diff(x) + y_p.abs_diff(y) <= 1)
                                {
                                    state.score += 1;
                                    253
                                } else {
                                    match i {
                                        4 => 6,
                                        6 => 1,
                                        1 => 3,
                                        3 => 2,
                                        2 => 7,
                                        7 => 5,
                                        5 => 4,
                                        _ => unreachable!(),
                                    }
                                };
                                *tiletypeid = NonZeroU8::try_from(j).unwrap();
                            }
                        }
                    }
                }
            }
            // Keep custom game state that's also visible to player, but hide it from the game engine that handles gameplay.
            if matches!(
                modifier_point,
                ModifierPoint::BeforeEvent(_) | ModifierPoint::BeforeButtonChange
            ) {
                state.lines_cleared = 0;
                state.next_pieces.clear();
                config.preview_count = 0;
                // state.level = NonZeroU32::try_from(SPEED_LEVEL).unwrap();
            } else {
                state.lines_cleared = usize::try_from(depth).unwrap();
                // state.level =
                //     NonZeroU32::try_from(u32::try_from(current_puzzle_idx + 1).unwrap()).unwrap();
            }
            // Remove ability to hold.
            if matches!(modifier_point, ModifierPoint::AfterButtonChange) {
                state.events.remove(&GameEvent::Hold);
            }
            // FIXME: Remove jank.
            active_piece.shape = descent_tetromino;
        },
    );
    let mut game = Game::new(GameMode {
        name: "Descent".to_string(),
        initial_gravity: 0,
        increase_gravity: false,
        limits: Limits {
            time: Some((true, Duration::from_secs(180))),
            ..Default::default()
        },
    });
    game.config_mut().preview_count = 0;
    game.add_modifier(descent_mode);
    game
}
