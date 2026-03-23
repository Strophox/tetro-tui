use std::{num::NonZeroU8, time::Duration};

use rand::Rng;

use falling_tetromino_engine::{
    Button, DelayParameters, ExtDuration, Game, GameAccess, GameBuilder, GameLimits, GameModifier,
    GameRng, InGameTime, Input, Line, NotificationFeed, Phase, Piece, Stat, Tetromino,
};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Ascent {
    height_loaded: usize,
}

impl Ascent {
    pub const MOD_ID: &str = stringify!(Ascent);

    pub fn build(builder: &GameBuilder) -> Game {
        let modifier = Box::new(Self { height_loaded: 0 });

        builder
            .clone()
            .lock_delay_params(DelayParameters::constant(ExtDuration::Infinite))
            .game_limits(GameLimits::single(
                Stat::TimeElapsed(Duration::from_secs(2 * 60)),
                true,
            ))
            .build_modded(vec![modifier])
    }
}

impl GameModifier for Ascent {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn args(&self) -> String {
        "".to_owned()
    }

    fn try_clone(&self) -> Result<Box<dyn GameModifier>, String> {
        Ok(Box::new(self.clone()))
    }

    fn on_game_built(&mut self, game: GameAccess) {
        // Load in board.
        let ascent_lines = Self::prng_ascent_lines(&mut self.height_loaded, &mut game.state.rng);
        for (line, ascent_line) in game
            .state
            .board
            .iter_mut()
            .take(Game::HEIGHT)
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
                orientation: falling_tetromino_engine::Orientation::N,
                position: (0, 0),
            },
            auto_move_scheduled: None,
            fall_or_lock_time: Duration::MAX,
            lowest_y: 0,
            lock_time_cap: Duration::MAX,
        };

        // Provide hold piece.
        game.state.piece_held = Some((asc_tet_02, true));

        // No other pieces required.
        game.config.piece_preview_count = 0;
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
        // FIXME: 'Hold' could as well (think: touches new gem!).
        if !matches!(
            input,
            Input::Activate(Button::RotateLeft | Button::Rotate180 | Button::RotateRight)
        ) {
            return;
        }
        // Guaranteed to be in `Phase::PieceInPlay`.
        let piece = game.phase.piece_mut().unwrap();

        let piece_tiles_coords = piece.tiles().map(|(coord, _)| coord);

        // Update entire board by cycling colors.
        for (y, line) in game.state.board.iter_mut().enumerate() {
            for (x, tile) in line.iter_mut().take(Self::PLAYABLE_WIDTH).enumerate() {
                let Some(tiletypeid) = tile else {
                    continue;
                };
                let i = tiletypeid.get();
                // Modify only certain tiles.
                if i <= 7 {
                    // Piece is touching the tile.
                    let tilenum = if piece_tiles_coords.iter().any(|&(x_p, y_p)| {
                        (x_p as usize).abs_diff(x) + (y_p as usize).abs_diff(y) <= 1
                    }) {
                        // Increase score.s
                        game.state.score += 1;
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

                    *tiletypeid = NonZeroU8::try_from(tilenum).unwrap();
                }
            }
        }

        // Adjust 'camera' if needed.
        let has_hit_camera_top =
            Game::LOCK_OUT_HEIGHT - Self::CAMERA_MARGIN_TOP <= (piece.position.1 as usize);
        if !has_hit_camera_top {
            return;
        }

        // Ascending virtual infinite board.
        let mut ascent_lines =
            Self::prng_ascent_lines(&mut self.height_loaded, &mut game.state.rng);
        game.state.board.rotate_left(1);
        game.state.board[Game::HEIGHT - 1] = ascent_lines.next().unwrap();
        piece.position.1 -= 1;

        // Count height in game state.
        game.state.lineclears += 1;
    }

    // The mod must pre-process: 'hold' to replace with custom hold, and 'drops' to prevent piece locking.
    fn on_player_input_received(
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
                    (game.phase.piece_mut(), game.state.piece_held.as_mut())
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
    const PLAYABLE_WIDTH: usize = Game::WIDTH - (1 - Game::WIDTH % 2);

    // FIXME: consider reintroducing: const CAMERA_ADJUST_DELAY: Duration = Duration::from_millis(125);
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
                        let white_tile = Some(NonZeroU8::try_from(255).unwrap());
                        *tile = white_tile;
                    }
                }

                // Add gem.
                let gem_idx = rng.random_range(0..Self::PLAYABLE_WIDTH);
                if line[gem_idx].is_some() {
                    line[gem_idx] = Some(NonZeroU8::try_from(rng.random_range(1..=7)).unwrap());
                }
            }

            // Extra tile for even board width and odd playable width.
            if Self::PLAYABLE_WIDTH != line.len() {
                let color = if (*height_loaded / 10).is_multiple_of(2)
                    ^ (height_loaded.is_multiple_of(10) || *height_loaded % 10 == 9)
                {
                    255 /*white*/
                } else {
                    2 /*sky*/
                };

                line[Self::PLAYABLE_WIDTH] = Some(NonZeroU8::try_from(color).unwrap());
            }

            *height_loaded += 1;
            line
        })
    }
}
