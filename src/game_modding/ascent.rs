use std::time::Duration;

use either::Either;
use rand::RngExt;

use crate::core_game_engine::{
    BOARD_WIDTH, Button, DelayParameters, ExtDuration, Game, GameAccess, GameBuilder, GameLimits,
    GameModifier, GameRng, InGameTime, Input, LOCK_OUT_HEIGHT, Line, MiscPceRots, MiscTetGens,
    NotificationFeed, Phase, Piece, Stat, Tetromino, TileType,
};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Ascent {
    // This modifier does not have fields for configuration/reproducibility.

    // Stateful fields.
    height_loaded: usize,
    cached_display_values: [(String, String); 1],
}

impl Ascent {
    pub const MOD_ID: &str = stringify!(Ascent);

    pub fn build(builder: &GameBuilder) -> Game {
        let modifier = Box::new(Self {
            height_loaded: 0,
            cached_display_values: [("Height ascended".to_owned(), 0.to_string())],
        });

        builder
            .clone()
            .rotation_system(crate::core_game_engine::MiscPceRots::Ocular)
            .lock_delay_curve(Some(Either::Left(DelayParameters::constant(
                ExtDuration::Infinite,
            ))))
            .game_limits(GameLimits::single(
                Stat::TimeElapsed(Duration::from_secs(2 * 60)),
                true,
            ))
            .build_modded(vec![modifier])
    }
}

impl GameModifier<MiscTetGens, MiscPceRots, TileType> for Ascent {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn cfg(&self) -> String {
        "".to_owned()
    }

    fn values(&self) -> &[(String, String)] {
        &self.cached_display_values
    }

    fn try_clone(
        &self,
    ) -> Result<Box<dyn GameModifier<MiscTetGens, MiscPceRots, TileType>>, String> {
        Ok(Box::new(self.clone()))
    }

    fn on_game_built(&mut self, game: GameAccess) {
        // Load in board.
        let ascent_lines = Self::prng_ascent_lines(&mut self.height_loaded, &mut game.state.rng);
        game.state
            .board
            .resize(Self::PREGENERATED_HEIGHT, Default::default());
        for ((line, _is_frozen), ascent_line) in game
            .state
            .board
            .iter_mut()
            .take(Self::PREGENERATED_HEIGHT)
            .zip(ascent_lines)
        {
            *line = ascent_line;
        }

        // Manually place active piece.
        let asc_tet_01 = Tetromino::L;
        let asc_tet_02 = Tetromino::J;
        *game.phase = Phase::PieceInPlay {
            piece: Piece {
                tetromino: asc_tet_01,
                orientation: crate::core_game_engine::Orientation::N,
                position: (0, 0),
            },
            autoshift_scheduled: None,
            fall_or_lock_time: Duration::MAX,
            lowest_y: 0,
            lock_cap_time: Duration::MAX,
        };

        // Provide hold piece.
        game.state.tetromino_held = Some((asc_tet_02, true));

        // No other pieces required.
        game.config.generate_piece_preview = 0;
    }

    // The Ascent mod must keep scoring after each piece change.
    // It must also adjust the 'camera' - visible board and piece state to simulate 'ascending'.
    fn on_player_action_post(
        &mut self,
        game: GameAccess,
        _feed: &mut NotificationFeed,
        input: Input,
    ) {
        // In this mode, only rotating the pieces can change it.
        // FIXME: We're forgetting 'Hold' could as well (e.g. when swap touches new gem). Experimental gamemode though so no biggie.
        if !matches!(
            input,
            Input::Activate(Button::RotateLeft | Button::Rotate180 | Button::RotateRight)
        ) {
            return;
        }

        // Normally guaranteed to be in `Phase::PieceInPlay`.
        let Some(piece) = game.phase.piece_mut() else {
            return;
        };

        let piece_tiles_coords = piece.coords();

        // Update entire board by cycling colors.
        for (y, (line, _is_frozen)) in game.state.board.iter_mut().enumerate() {
            for (x, tile) in line.iter_mut().take(Self::PLAYABLE_WIDTH).enumerate() {
                let Some(tile_type) = tile else {
                    continue;
                };
                // Modify only certain tiles.
                if matches!(tile_type, TileType::Tet(_)) {
                    // Piece is touching the tile.
                    let new_tile_type = if piece_tiles_coords.iter().any(|&(x_p, y_p)| {
                        (x_p as usize).abs_diff(x) + (y_p as usize).abs_diff(y) <= 1
                    }) {
                        // Increase score.s
                        game.state.points += 1;
                        TileType::Generic
                    } else {
                        match tile_type {
                            TileType::Tet(tet) => TileType::Tet(match tet {
                                Tetromino::O => Tetromino::S,
                                Tetromino::I => Tetromino::J,
                                Tetromino::S => Tetromino::I,
                                Tetromino::Z => Tetromino::L,
                                Tetromino::T => Tetromino::Z,
                                Tetromino::L => Tetromino::O,
                                Tetromino::J => Tetromino::T,
                            }),
                            TileType::Generic => TileType::Generic,
                        }
                    };

                    *tile = Some(new_tile_type);
                }
            }
        }

        // Adjust 'camera' if needed.
        let has_hit_camera_top =
            LOCK_OUT_HEIGHT - Self::CAMERA_MARGIN_TOP <= (piece.position.1 as usize);
        if has_hit_camera_top {
            let mut ascent_lines =
                Self::prng_ascent_lines(&mut self.height_loaded, &mut game.state.rng);
            game.state.board.push((ascent_lines.next().unwrap(), false));
            piece.position.1 -= 1;
        }

        // Ascending virtual infinite board.
        // FIXME: Check if correct.
        self.cached_display_values[0].1 = (piece.position.1 as usize + self.height_loaded
            - Self::PREGENERATED_HEIGHT)
            .to_string();
    }

    // The mod must pre-process: 'hold' to replace with custom hold, and 'drops' to prevent piece locking.
    fn on_receive_player_input(
        &mut self,
        game: GameAccess,
        _feed: &mut NotificationFeed,
        _time: &mut InGameTime,
        player_input: &mut Option<Input>,
    ) {
        match player_input {
            Some(Input::Activate(Button::HoldPiece)) => {
                // Remove hold input to stop engine from processing it.
                player_input.take();

                // Manually swap pieces if available.
                let (Some(piece), Some((held_tetromino, _))) =
                    (game.phase.piece_mut(), game.state.tetromino_held.as_mut())
                else {
                    return;
                };

                (piece.tetromino, *held_tetromino) = (*held_tetromino, piece.tetromino);
            }

            Some(Input::Activate(Button::DropSoft | Button::DropHard)) => {
                // Remove drop inputs to stop engine from locking down the piece.
                player_input.take();
            }

            _ => {}
        }
    }
}

impl Ascent {
    // Playable width needs to be odd.
    const PLAYABLE_WIDTH: usize = BOARD_WIDTH - (1 - BOARD_WIDTH % 2);
    const PREGENERATED_HEIGHT: usize = LOCK_OUT_HEIGHT + 4;

    const CAMERA_MARGIN_TOP: usize = 5;

    fn prng_ascent_lines<'a>(
        height_loaded: &'a mut usize,
        rng: &'a mut GameRng,
    ) -> impl Iterator<Item = Line> + 'a {
        std::iter::repeat(Line::default()).map(|mut line| {
            // Only generate the particular ascent line consisting of mino hinges if it's on an 'odd' height.
            if !height_loaded.is_multiple_of(2) {
                // Add hinges.
                for (j, tile) in line.iter_mut().enumerate() {
                    if j % 2 == 1 {
                        *tile = Some(TileType::Generic);
                    }
                }

                // Add gem.
                let gem_idx = rng.random_range(0..Self::PLAYABLE_WIDTH);
                if line[gem_idx].is_some() {
                    line[gem_idx] = Some(Tetromino::VARIANTS[rng.random_range(1..=7)].into());
                }
            }

            // Extra tile for even board width and odd playable width.
            if Self::PLAYABLE_WIDTH != line.len() {
                let tile_ty = if (*height_loaded / 10).is_multiple_of(2)
                    ^ (height_loaded.is_multiple_of(10) || *height_loaded % 10 == 9)
                {
                    TileType::Generic
                } else {
                    TileType::Tet(Tetromino::I)
                };

                line[Self::PLAYABLE_WIDTH] = Some(tile_ty);
            }

            *height_loaded += 1;
            line
        })
    }
}
