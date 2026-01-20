use std::{num::NonZeroU8, time::Duration};

use rand::{self, Rng};

use tetrs_engine::{
    ActivePiece, Game, GameBuilder, GameEvent, GameModFn, GameRng, GameTime, Line, LockingData, ModificationPoint, Modifier, Rules, Stat, Tetromino
};

pub const MOD_ID: &str = "ascent";

// Playable width needs to be odd.
const PLAYABLE_WIDTH: usize = Game::WIDTH - (1 - Game::WIDTH % 2);

pub fn build(builder: &GameBuilder) -> Game {
    let ascent_tetromino = if rand::rng().random_bool(0.5) {
        Tetromino::L
    } else {
        Tetromino::J
    };
    
    let timeperiod_camera_adjust = Duration::from_millis(125);
    let mut timepoint_camera_adjusted = GameTime::ZERO;
    let mut height_generated = 0usize;
    let mut init = false;
    let mod_function: Box<GameModFn> =
        Box::new(move |config, _rules, state, modpoint, _messages| {
            // Initialize mod.
            if !init {
                init = true;
                let line_source = random_ascent_lines(&mut state.rng, &mut height_generated);
                for (line, ascent_line) in state
                    .board
                    .iter_mut()
                    .take(Game::HEIGHT)
                    .zip(line_source)
                {
                    *line = ascent_line;
                }
                config.preview_count = 0;
                state.active_piece_data = Some((ActivePiece { shape: ascent_tetromino, orientation: tetrs_engine::Orientation::N, position: (0,0) }, LockingData { touches_ground: true, last_touchdown: None, last_liftoff: None, ground_time_left: Duration::ZERO, lowest_y: 0 }));
                state.events.clear()
            }

            // We can only do things if a piece exists.
            let Some((active_piece, _)) = &mut state.active_piece_data else {
                return;
            };

            let has_camera_adjust_period_elapsed =
                state.time.saturating_sub(timepoint_camera_adjusted) >= timeperiod_camera_adjust;
            let hit_camera_top = Game::SKYLINE - 5 <= active_piece.position.1;

            // Ascending virtual infinite board.
            if hit_camera_top && has_camera_adjust_period_elapsed {
                active_piece.position.1 -= 1;
                state.lines_cleared += 1;
                let mut line_source = random_ascent_lines(&mut state.rng, &mut height_generated);
                state.board.push(line_source.next().unwrap());
                state.board.remove(0);
                timepoint_camera_adjusted = state.time;
            }

            // Update state after each piece rotation, for gem scorekeeping.
            // Also change colors for fun after each rotation.
            if matches!(
                modpoint,
                ModificationPoint::AfterEvent(GameEvent::Rotate(_))
            ) {
                let piece_tiles_coords = active_piece.tiles().map(|(coord, _)| coord);

                for (y, line) in state.board.iter_mut().enumerate() {
                    for (x, tile) in line.iter_mut().take(PLAYABLE_WIDTH).enumerate() {
                        let Some(tiletypeid) = tile else {
                            continue;
                        };
                        let i = tiletypeid.get();
                        if i <= 7 {
                            let j = if piece_tiles_coords
                                .iter()
                                .any(|(x_p, y_p)| x_p.abs_diff(x) + y_p.abs_diff(y) <= 1)
                            {
                                state.score += 1;
                                254
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

            // Remove various events we don't want to happen.
                state.events.remove(&GameEvent::Hold);
                state.events.remove(&GameEvent::HardDrop);
                state.events.remove(&GameEvent::LockTimer);
                // state.events.remove(&GameEvent::Lock);
        });

    let rules = Rules {
        initial_gravity: 0,
        progressive_gravity: false,
        end_conditions: vec![(Stat::TimeElapsed(Duration::from_secs(3 * 60)), true)],
    };

    let ascent_modifier = Modifier {
        descriptor: MOD_ID.to_owned(),
        mod_function,
    };

    builder
        .clone()
        .rules(rules)
        .build_modified([ascent_modifier])
}

pub fn random_ascent_lines<'a>(rng: &'a mut GameRng, height_generated: &'a mut usize) -> impl Iterator<Item = Line> + 'a {
    std::iter::repeat(Line::default()).map(move |mut line| {
        if *height_generated % 2 != 0 {
            // Add hinges.
            for (j, tile) in line.iter_mut().enumerate() {
                if j % 2 == 1 {
                    let white_tile = Some(NonZeroU8::try_from(255).unwrap());
                    *tile = white_tile;
                }
            }

            // Add gem.
            let gem_idx = rng.random_range(0..PLAYABLE_WIDTH);
            if line[gem_idx].is_some() {
                line[gem_idx] = Some(NonZeroU8::try_from(rng.random_range(1..=7)).unwrap());
            }
        }

        // Extra tile for even board width and odd playable width.
        if PLAYABLE_WIDTH != line.len() {
            line[PLAYABLE_WIDTH] = Some(NonZeroU8::try_from(if (*height_generated / 10) % 2 == 0 { 255/*white*/ } else { 2 /*sky*/ }).unwrap());
        }

        *height_generated += 1;
        line
    })
}
