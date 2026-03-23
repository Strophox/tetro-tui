use std::{collections::VecDeque, num::NonZeroU8, time::Duration};

use falling_tetromino_engine::{
    Button, DelayParameters, Game, GameAccess, GameBuilder, GameEndCause, GameModifier, InGameTime,
    Input, Line, Notification, NotificationFeed, Phase, State, Tetromino,
};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Puzzle {
    init: bool,
    stage_idx: usize,
    stage_tet_count: usize,
    stage_attempts: usize,
    end_post_spawn: Option<bool>,
}

impl Puzzle {
    pub const MOD_ID: &str = stringify!(Puzzle);

    pub fn build(builder: &GameBuilder) -> Game {
        let modifier = Box::new(Self {
            init: false,
            stage_idx: 0,
            stage_tet_count: 0,
            stage_attempts: 0,
            end_post_spawn: None,
        });

        builder
            .clone()
            .fall_delay_params(DelayParameters::constant(
                Duration::from_millis(1000).into(),
            ))
            .piece_preview_count(0)
            .build_modded(vec![modifier])
    }
}

impl GameModifier for Puzzle {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn args(&self) -> String {
        "".to_owned()
    }

    fn try_clone(&self) -> Result<Box<dyn GameModifier>, String> {
        Ok(Box::new(self.clone()))
    }

    fn on_spawn_pre(
        &mut self,
        game: GameAccess,
        feed: &mut NotificationFeed,
        time: &mut InGameTime,
    ) {
        if !self.init {
            self.init = true;

            self.load_stage(game.state);

            // Push notifications in reverse order so they display correctly in newest-to-oldest~top-to-bottom order.
            // feed.push((
            //     Notification::Custom(format!("{:?}", Self::get_stage_data(self.stage_idx).0)),
            //     *time,
            // ));
            feed.push((
                Notification::Custom(format!("Stage {}", self.stage_idx + 1)),
                *time,
            ));
            feed.push((Notification::Custom("Clear to advance!".to_string()), *time));

            return;
        }

        let current_piece_count =
            usize::try_from(game.state.pieces_locked.iter().sum::<u32>()).unwrap();

        // We're only interested in updating the game if the end of a puzzle stage has been reached.
        if current_piece_count < self.stage_tet_count {
            return;
        }

        // From here assume player used up all pieces.

        // A stage has been successfully finished if every line on the board is empty.
        let stage_success = game.state.board.iter().all(|line| *line == Line::default());

        // Failed on last attempt, this is game over.
        if !stage_success && self.stage_attempts == Self::MAX_STAGE_ATTEMPTS {
            self.end_post_spawn = Some(false);

            return;
        }

        // May have failed or succeeded, load in correct puzzle for each case.

        if stage_success {
            // Move on to new stage.
            self.stage_attempts = 0;
            self.stage_idx += 1;

            // Done with all stages, game completed.
            if self.stage_idx == Self::STAGES_LEN {
                self.end_post_spawn = Some(true);

                return;
            }

            // Push notifications in reverse order so they display correctly in newest-to-oldest~top-to-bottom order.
            // feed.push((
            //     Notification::Custom(format!("{:?}", Self::get_stage_data(self.stage_idx).0)),
            //     *time,
            // ));
            feed.push((
                Notification::Custom(format!("Stage {}", self.stage_idx + 1)),
                *time,
            ));
        } else {
            // Reattempt stage.
            self.stage_attempts += 1;

            // Push notification.
            let text = if self.stage_attempts == Self::MAX_STAGE_ATTEMPTS {
                "last attempt".to_owned()
            } else {
                format!(
                    "{} att. left",
                    Self::MAX_STAGE_ATTEMPTS + 1 - self.stage_attempts
                )
            };

            feed.push((Notification::Custom(text), *time));
        }

        self.load_stage(game.state);

        // Reset some game state.
        game.state.score = 0;
        game.state.lineclears = 0;
        game.state.pieces_locked = Default::default();
    }

    fn on_spawn_post(&mut self, game: GameAccess, _feed: &mut NotificationFeed) {
        if let Some(is_win) = self.end_post_spawn {
            *game.phase = if is_win {
                Phase::GameEnd {
                    cause: GameEndCause::Custom("All stages completed".to_owned()),
                    is_win: true,
                }
            } else {
                Phase::GameEnd {
                    cause: GameEndCause::Custom("Too many attempts".to_owned()),
                    is_win: false,
                }
            }
        }
    }

    fn on_lines_clear_post(&mut self, _game: GameAccess, feed: &mut NotificationFeed) {
        feed.retain(|(n, _)| !matches!(n, Notification::Accolade { .. }));
    }

    fn on_player_input_received(
        &mut self,
        _game: GameAccess,
        _feed: &mut NotificationFeed,
        _time: &mut InGameTime,
        player_input: &mut Option<Input>,
    ) {
        // Essentially prevent the player from holding pieces.
        if matches!(player_input, Some(Input::Activate(Button::HoldPiece))) {
            player_input.take();
        }
    }
}

impl Puzzle {
    fn load_stage(&mut self, state: &mut State) {
        let (_stage_name, stage_lines, stage_tetrominos) = Self::get_stage_data(self.stage_idx);

        let grey_tile = Some(NonZeroU8::try_from(254).unwrap());
        for (stage_line, game_line) in stage_lines
            .iter()
            .rev()
            .chain(std::iter::repeat(&&[b' '; 10]))
            .zip(state.board.iter_mut())
        {
            *game_line = Line::default();
            if stage_line.iter().any(|c| c != &b' ') {
                for (game_cell, puzzle_tile) in game_line
                    .iter_mut()
                    .zip(stage_line.iter().chain(std::iter::repeat(&b'O')))
                {
                    if puzzle_tile != &b' ' {
                        *game_cell = grey_tile;
                    }
                }
            }
        }

        // Load in stage tetrominos.
        state.piece_preview.clone_from(&stage_tetrominos);

        // Save stage length.
        self.stage_tet_count = stage_tetrominos.len();
    }
}

impl Puzzle {
    const MAX_STAGE_ATTEMPTS: usize = 4;
    const STAGES_LEN: usize = 24;

    #[allow(clippy::type_complexity)]
    #[rustfmt::skip]
    fn get_stage_data(idx: usize) -> (&'static str, Vec<&'static [u8; 10]>, VecDeque<Tetromino>) {
        let stages = [
            /* Puzzle template.
            ("puzzlename", vec![
                b"OOOOOOOOOO",
                b"OOOOOOOOOO",
                b"OOOOOOOOOO",
                b"OOOOOOOOOO",
            ], VecDeque::from([Tetromino::I,])),
            */
            /*("DEBUG L/J", vec![
                b" O O O O O",
                b"         O",
                b" O O O O O",
                b"         O",
                b" O O O O O",
                b"         O",
                b" O O O O O",
                b"         O",
            ], VecDeque::from([Tetromino::L,Tetromino::J])),*/
            // 4 I-spins.
            ("I-spin", vec![
                b"OOOOO OOOO",
                b"OOOOO OOOO",
                b"OOOOO OOOO",
                b"OOOOO OOOO",
                b"OOOO    OO",
                ], VecDeque::from([Tetromino::I,Tetromino::I])),
            ("I-spin II", vec![
                b"OOOOO  OOO",
                b"OOOOO OOOO",
                b"OOOOO OOOO",
                b"OO    OOOO",
                ], VecDeque::from([Tetromino::I,Tetromino::J])),
            ("I-spin III", vec![
                b"OO  O   OO",
                b"OO    OOOO",
                b"OOOO OOOOO",
                b"OOOO OOOOO",
                b"OOOO OOOOO",
                ], VecDeque::from([Tetromino::I,Tetromino::L,Tetromino::O,])),
            ("I-spin trial", vec![
                b"OOOOO  OOO",
                b"OOO OO OOO",
                b"OOO OO OOO",
                b"OOO     OO",
                b"OOO OOOOOO",
                ], VecDeque::from([Tetromino::I,Tetromino::I,Tetromino::L,])),
            // 4 S/Z-spins.
            ("S-spin", vec![
                b"OOOO  OOOO",
                b"OOO  OOOOO",
                ], VecDeque::from([Tetromino::S,])),
            ("S-spin II", vec![
                b"OOOO    OO",
                b"OOO    OOO",
                b"OOOOO  OOO",
                b"OOOO  OOOO",
                ], VecDeque::from([Tetromino::S,Tetromino::S,Tetromino::S,])),
            ("Z-spin galore", vec![
                b"O  OOOOOOO",
                b"OO  OOOOOO",
                b"OOO  OOOOO",
                b"OOOO  OOOO",
                b"OOOOO  OOO",
                b"OOOOOO  OO",
                b"OOOOOOO  O",
                b"OOOOOOOO  ",
                ], VecDeque::from([Tetromino::Z,Tetromino::Z,Tetromino::Z,Tetromino::Z,])),
            ("SuZ-spins", vec![
                b"OOOO  OOOO",
                b"OOO  OOOOO",
                b"OO    OOOO",
                b"OO    OOOO",
                b"OOO    OOO",
                b"OO  OO  OO",
                ], VecDeque::from([Tetromino::S,Tetromino::S,Tetromino::I,Tetromino::I,Tetromino::Z,])),
            // 4 L/J-spins.
            ("J-spin", vec![
                b"OO     OOO",
                b"OOOOOO OOO",
                b"OOOOO  OOO",
                ], VecDeque::from([Tetromino::J,Tetromino::I,])),
            ("L/J-spins", vec![
                b"OO      OO",
                b"OO OOOO OO",
                b"OO  OO  OO",
                ], VecDeque::from([Tetromino::J,Tetromino::L,Tetromino::I])),
            ("L-spin", vec![
                b"OOOOO OOOO",
                b"OOO   OOOO",
                ], VecDeque::from([Tetromino::L,])),
            ("L/J-spins II", vec![
                b"O   OO   O",
                b"O O OO O O",
                b"O   OO   O",
                ], VecDeque::from([Tetromino::J,Tetromino::L,Tetromino::J,Tetromino::L,])),
            // 4 L/J-turns.
            ("7-7", vec![
                b"OOOO  OOOO",
                b"OOOOO OOOO",
                b"OOO   OOOO",
                b"OOOO OOOOO",
                b"OOOO OOOOO",
                ], VecDeque::from([Tetromino::L,Tetromino::L,])),
            ("7-turn", vec![
                b"OOOOO  OOO",
                b"OOO    OOO",
                b"OOOO OOOOO",
                b"OOOO OOOOO",
                ], VecDeque::from([Tetromino::L,Tetromino::O,])),
            ("L-turn", vec![
                b"OOOO  OOOO",
                b"OOOO  OOOO",
                b"OOOO   OOO",
                b"OOOO OOOOO",
                ], VecDeque::from([Tetromino::L,Tetromino::O,])),
            ("L-turn trial", vec![
                b"OOOO  OOOO",
                b"OOOO  OOOO",
                b"OO     OOO",
                b"OOO  OOOOO",
                b"OOO OOOOOO",
                ], VecDeque::from([Tetromino::L,Tetromino::L,Tetromino::O,])),
            // 7 T-spins.
            ("T-spin", vec![
                b"OOOO    OO",
                b"OOO   OOOO",
                b"OOOO OOOOO",
                ], VecDeque::from([Tetromino::T,Tetromino::I])),
            ("T-spin II", vec![
                b"OOOO    OO",
                b"OOO   OOOO",
                b"OOOO OOOOO",
                ], VecDeque::from([Tetromino::T,Tetromino::L])),
            ("T-tuck", vec![
                b"OO   OOOOO",
                b"OOO  OOOOO",
                b"OOO   OOOO",
                ], VecDeque::from([Tetromino::T,Tetromino::T])),
            ("T-insert", vec![
                b"OOOO  OOOO",
                b"OOOO  OOOO",
                b"OOOOO OOOO",
                b"OOOO   OOO",
                ], VecDeque::from([Tetromino::T,Tetromino::O])),
            ("T-go-round", vec![
                b"OOO  OOOOO",
                b"OOO   OOOO",
                b"OOOOO  OOO",
                b"OOOOO OOOO",
                ], VecDeque::from([Tetromino::T,Tetromino::O])),
            ("T-spin Tri. setup", vec![
                b"OOOOO  OOO",
                b"OOOOO  OOO",
                b"OOO   OOOO",
                b"OOOO OOOOO",
                ], VecDeque::from([Tetromino::T,Tetromino::O])),
            ("T-spin Triple", vec![
                b"OOOO   OOO",
                b"OOOOO  OOO",
                b"OOO   OOOO",
                b"OOOO OOOOO",
                b"OOO  OOOOO",
                b"OOOO OOOOO",
                ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::J])),
            ("Final Crossover", vec![ // v2.2.1
                b"OOOO  OOOO",
                b"O  O  OOOO",
                b"  OOO OOOO",
                b"OOO    OOO",
                b"OOOOOO   O",
                b"  O    OOO",
                b"OOOOO OOOO",
                b"O  O  OOOO",
                b"OOOOO OOOO",
                ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::O,Tetromino::S,Tetromino::I,Tetromino::J,Tetromino::Z])),
            // ("T-spin FINALE v2.3", vec![
            //     b"OOOO  OOOO",
            //     b"OOOO  O  O",
            //     b"OOOO OOO  ",
            //     b"OOO    OOO",
            //     b"O   OOOOOO",
            //     b"OOO    OOO",
            //     b"OOOO OOO  ",
            //     b"OOOO  O  O",
            //     b"OOOO OOOOO",
            //     ], VecDeque::from([Tetromino::T,Tetromino::J,Tetromino::O,Tetromino::Z,Tetromino::I,Tetromino::L,Tetromino::S])),
            // ("T-spin FINALE v2.2", vec![
            //     b"OOOO  OOOO",
            //     b"O  O  OOOO",
            //     b"  OOO OOOO",
            //     b"OOO    OOO",
            //     b"OOOOOO   O",
            //     b"OOO    OOO",
            //     b"  OOO OOOO",
            //     b"O  O  OOOO",
            //     b"OOOOO OOOO",
            //     ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::O,Tetromino::S,Tetromino::I,Tetromino::J,Tetromino::Z])),
            // ("T-spin FINALE v2.1", vec![
            //     b"OOOO  OOOO",
            //     b"OOOO  OOOO",
            //     b"OOOOO OOOO",
            //     b"OOO    OOO",
            //     b"OOOOOO   O",
            //     b"OOO    OOO",
            //     b"  OOO OO  ",
            //     b"O  O  OOOO",
            //     b"OOOOO O  O",
            //     ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::O,Tetromino::I,Tetromino::J,Tetromino::Z,Tetromino::S])),
            // ("T-spin FINALE v3", vec![
            //     b"OOOO  OOOO",
            //     b"OOOO  OOOO",
            //     b"OOOOO OOOO",
            //     b"OOO    OOO",
            //     b"OOOOOO   O",
            //     b"OOO    OOO",
            //     b"OOOOO OOOO",
            //     b"O  O  O  O",
            //     b"O  OO OO  ",
            //     ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::S,Tetromino::I,Tetromino::J,Tetromino::O,Tetromino::Z])),
            // ("T-spin FINALE v2", vec![
            //     b"OOOO  OOOO",
            //     b"OOOO  OOOO",
            //     b"OOOOO OOOO",
            //     b"OOO    OOO",
            //     b"OOOOOO   O",
            //     b"OOO    OOO",
            //     b"OOOOO OOOO",
            //     b"O  O  O  O",
            //     b"  OOO OO  ",
            //     ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::O,Tetromino::I,Tetromino::J,Tetromino::Z,Tetromino::S])),
            // ("T-spin FINALE v1", vec![
            //     b"OOOO  OOOO",
            //     b"OOOO  OOOO",
            //     b"OOOOO OOOO",
            //     b"OOO     OO",
            //     b"OOOOOO   O",
            //     b"OO     O  ",
            //     b"OOOOO OOOO",
            //     b"O  O  OOOO",
            //     b"  OOO OOOO",
            //     ], VecDeque::from([Tetromino::T,Tetromino::O,Tetromino::L,Tetromino::I,Tetromino::J,Tetromino::Z,Tetromino::S])),
        ];

        stages[idx].clone()
    }
}
