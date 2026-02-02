use std::{num::NonZeroU8, time::Duration};

use rand::Rng;

use tetrs_engine::{
    Button, ButtonChange, Game, GameBuilder, GameModFn, GameRng, GameTime, Line, Modifier, Phase,
    Piece, PieceData, Stat, Tetromino, UpdatePoint,
};

pub const MOD_ID: &str = "ascent";

// Playable width needs to be odd.
const PLAYABLE_WIDTH: usize = Game::WIDTH - (1 - Game::WIDTH % 2);

pub fn build(builder: &GameBuilder) -> Game {
    let timeperiod_camera_adjust = Duration::from_millis(125);
    let mut timepoint_camera_adjusted = GameTime::ZERO;
    let mut height_generated = 0usize;
    let mut init = false;
    let mod_function: Box<GameModFn> =
        Box::new(move |point, config, _init_vals, state, phase, _msgs| {
            // Initialize mod.
            if !init {
                init = true;
                let line_source = random_ascent_lines(&mut state.rng, &mut height_generated);
                for (line, ascent_line) in
                    state.board.iter_mut().take(Game::HEIGHT).zip(line_source)
                {
                    *line = ascent_line;
                }
                // Manually place active piece.
                let asc_tet_01 = Tetromino::L;
                let asc_tet_02 = Tetromino::J;
                *phase = Phase::PieceInPlay {
                    piece_data: PieceData {
                        piece: Piece {
                            tetromino: asc_tet_01,
                            orientation: tetrs_engine::Orientation::N,
                            position: (0, 0),
                        },
                        fall_or_lock_time: Duration::MAX,
                        is_fall_not_lock: false,
                        lowest_y: 0,
                        capped_lock_time: Duration::MAX,
                        auto_move_scheduled: None,
                    },
                };
                state.hold_piece = Some((asc_tet_02, true));
                // No further pieces required.
                config.piece_preview_count = 0;
            }

            // We can only do things if a piece exists.
            let Some(piece) = phase.piece_mut() else {
                return;
            };

            let has_camera_adjust_period_elapsed =
                state.time.saturating_sub(timepoint_camera_adjusted) >= timeperiod_camera_adjust;
            let hit_camera_top = Game::SKYLINE - 5 <= piece.position.1;

            // Ascending virtual infinite board.
            if hit_camera_top && has_camera_adjust_period_elapsed {
                piece.position.1 -= 1;
                state.lines_cleared += 1;
                let mut line_source = random_ascent_lines(&mut state.rng, &mut height_generated);
                state.board.rotate_left(1);
                state.board[Game::HEIGHT - 1] = line_source.next().unwrap();
                timepoint_camera_adjusted = state.time;
            }

            // Update state after each piece rotation, for gem scorekeeping.
            // Also change colors for fun after each rotation.
            if matches!(
                point,
                UpdatePoint::PiecePlayed(ButtonChange::Press(
                    Button::RotateLeft | Button::RotateAround | Button::RotateRight
                ))
            ) {
                let piece_tiles_coords = piece.tiles().map(|(coord, _)| coord);

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

            // Replace hold with custom hold.
            if let UpdatePoint::MainLoopHead(button_changes) = point {
                if matches!(button_changes, Some(ButtonChange::Press(Button::HoldPiece))) {
                    // Remove hold input to stop engine from processing it.
                    button_changes.take();
                    // Manually swap pieces.
                    let (tet1, tet2) = (
                        &mut phase.piece_mut().unwrap().tetromino,
                        &mut state.hold_piece.as_mut().unwrap().0,
                    );
                    (*tet1, *tet2) = (*tet2, *tet1);
                } else if matches!(
                    button_changes,
                    Some(ButtonChange::Press(Button::DropSoft | Button::DropHard))
                ) {
                    button_changes.take();
                }
            }

            // Ensure we can always hold.
            state.hold_piece.unwrap().1 = true;
        });

    builder
        .clone()
        .initial_gravity(0)
        .progressive_gravity(false)
        .end_conditions(vec![(Stat::TimeElapsed(Duration::from_secs(2 * 60)), true)])
        .build_modded([Modifier {
            descriptor: MOD_ID.to_owned(),
            mod_function,
        }])
}

pub fn random_ascent_lines<'a>(
    rng: &'a mut GameRng,
    height_generated: &'a mut usize,
) -> impl Iterator<Item = Line> + 'a {
    std::iter::repeat(Line::default()).map(move |mut line| {
        if !height_generated.is_multiple_of(2) {
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
            line[PLAYABLE_WIDTH] = Some(
                NonZeroU8::try_from(if (*height_generated / 10).is_multiple_of(2) {
                    255 /*white*/
                } else {
                    2 /*sky*/
                })
                .unwrap(),
            );
        }

        *height_generated += 1;
        line
    })
}
