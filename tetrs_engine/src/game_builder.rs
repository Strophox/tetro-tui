/*!
This module handles creation / initialization / building of [`Game`]s.
*/

use super::*;

/// This builder exposes the ability to configure a new [`Game`] to varying degrees.
///
/// Generally speaking, when using `GameBuilder`, you’ll first call [`GameBuilder::new`] or
/// [`Game::builder`], then chain calls to methods to set each field, then call
/// [`GameBuilder::build`] or [`GameBuilder::build_modified`].
/// This will give you a [`Game`] as specified that you can then use as normal.
/// The `GameBuilder` is not used up and its configuration can be re-used to initialize more [`Game`]s.
#[derive(PartialEq, PartialOrd, Clone, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameBuilder {
    /// Many of the configuration options that will be set for the game.
    pub config: Configuration,
    /// The value to seed the game's PRNG with.
    pub seed: Option<u64>,
    /// The method (and internal state) of tetromino generation used.
    pub initial_tetromino_generator: Option<TetrominoGenerator>,
    /// The fall delay at the beginning of the game.
    pub initial_fall_delay: Option<ExtDuration>,
    /// The lock delay at the beginning of the game.
    pub initial_lock_delay: Option<ExtDuration>,
}

impl GameBuilder {
    /// Creates a blank new template representing a yet-to-be-started [`Game`] ready for configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a [`Game`] with the information specified by `self`.
    pub fn build(&self) -> Game {
        self.build_modded([])
    }

    /// Creates a [`Game`] with the information specified by `self` and some one-time `modifiers`.
    pub fn build_modded(&self, modifiers: impl IntoIterator<Item = Modifier>) -> Game {
        let config = self.config.clone();

        let init_vals = InitialValues::default_seeded();
        let init_vals = InitialValues {
            seed: self.seed.unwrap_or(init_vals.seed),
            initial_tetromino_generator: self
                .initial_tetromino_generator
                .unwrap_or(init_vals.initial_tetromino_generator),
            initial_fall_delay: self
                .initial_fall_delay
                .unwrap_or(init_vals.initial_fall_delay),
            initial_lock_delay: self
                .initial_lock_delay
                .unwrap_or(init_vals.initial_lock_delay),
        };

        Game {
            state: State {
                time: Duration::ZERO,
                buttons_pressed: [None; Button::VARIANTS.len()],
                rng: GameRng::seed_from_u64(init_vals.seed),
                piece_generator: init_vals.initial_tetromino_generator,
                piece_preview: VecDeque::new(),
                hold_piece: None,
                board: [Line::default(); Game::HEIGHT],
                fall_delay: config
                    .fall_delay_lowerbound
                    .max(init_vals.initial_fall_delay),
                fall_delay_lowerbound_hit_at_n_lineclears: init_vals
                    .initial_fall_delay
                    .le(&config.fall_delay_lowerbound)
                    .then_some(0),
                lock_delay: config
                    .lock_delay_lowerbound
                    .max(init_vals.initial_lock_delay),
                pieces_locked: [0; Tetromino::VARIANTS.len()],
                lineclears: 0,
                consecutive_line_clears: 0,
                score: 0,
            },
            phase: Phase::Spawning {
                spawn_time: Duration::ZERO,
            },
            modifiers: modifiers.into_iter().collect(),
            config,
            init_vals,
        }
    }

    /// Sets the [`Configuration`] that will be used by [`Game`].
    pub fn config(&mut self, x: Configuration) -> &mut Self {
        self.config = x;
        self
    }

    /// How many pieces should be pre-generated and accessible/visible in the game state.
    pub fn piece_preview_count(&mut self, x: usize) -> &mut Self {
        self.config.piece_preview_count = x;
        self
    }
    /// Whether holding a rotation button lets a piece be smoothly spawned in a rotated state.
    pub fn allow_prespawn_actions(&mut self, x: bool) -> &mut Self {
        self.config.allow_prespawn_actions = x;
        self
    }
    /// The method of tetromino rotation used.
    pub fn rotation_system(&mut self, x: RotationSystem) -> &mut Self {
        self.config.rotation_system = x;
        self
    }
    /// How long the game should wait *additionally* before spawning a new piece.
    pub fn spawn_delay(&mut self, x: Duration) -> &mut Self {
        self.config.spawn_delay = x;
        self
    }
    /// How long it takes for the active piece to start automatically shifting more to the side
    /// after the initial time a 'move' button has been pressed.
    pub fn delayed_auto_shift(&mut self, x: Duration) -> &mut Self {
        self.config.delayed_auto_shift = x;
        self
    }
    /// How long it takes for automatic side movement to repeat once it has started.
    pub fn auto_repeat_rate(&mut self, x: Duration) -> &mut Self {
        self.config.auto_repeat_rate = x;
        self
    }
    /// Specification of how fall delay gets calculated from the rest of the state.
    pub fn fall_delay_equation(&mut self, x: DelayEquation) -> &mut Self {
        self.config.fall_delay_equation = x;
        self
    }
    /// Specification of where to stop decreasing fall delay and start decreasing lock delay.
    pub fn fall_delay_lowerbound(&mut self, x: ExtDuration) -> &mut Self {
        self.config.fall_delay_lowerbound = x;
        self
    }
    /// How many times faster than normal drop speed a piece should fall while 'soft drop' is being held.
    pub fn soft_drop_divisor(&mut self, x: ExtNonNegF64) -> &mut Self {
        self.config.soft_drop_divisor = x;
        self
    }
    /// Specification of how fall delay gets calculated from the rest of the state.
    pub fn lock_delay_equation(&mut self, x: DelayEquation) -> &mut Self {
        self.config.lock_delay_equation = x;
        self
    }
    /// Specification of where to stop decreasing lock delay.
    pub fn lock_delay_lowerbound(&mut self, x: ExtDuration) -> &mut Self {
        self.config.lock_delay_lowerbound = x;
        self
    }
    /// Whether just pressing a rotation- or movement button is enough to refresh lock delay.
    /// Normally, lock delay only resets if rotation or movement actually succeeds.
    pub fn lenient_lock_delay_reset(&mut self, x: bool) -> &mut Self {
        self.config.lenient_lock_delay_reset = x;
        self
    }
    /// How long each spawned active piece may touch the ground in total until it should lock down
    /// immediately.
    pub fn capped_lock_time_factor(&mut self, x: ExtNonNegF64) -> &mut Self {
        self.config.capped_lock_time_factor = x;
        self
    }
    /// How long the game should wait after clearing a line.
    pub fn line_clear_duration(&mut self, x: Duration) -> &mut Self {
        self.config.line_clear_duration = x;
        self
    }
    /// When to update the fall and lock delays in [`State`].
    pub fn update_delays_every_n_lineclears(&mut self, x: u32) -> &mut Self {
        self.config.update_delays_every_n_lineclears = x;
        self
    }
    /// Stores the ways in which a round of the game should be limited.
    ///
    /// Each limitation may be either of positive ('game completed') or negative ('game over'), as
    /// designated by the `bool` stored with it.
    ///
    /// No limitations may allow for endless games.
    pub fn end_conditions(&mut self, x: Vec<(Stat, bool)>) -> &mut Self {
        self.config.end_conditions = x;
        self
    }
    /// The amount of feedback information that is to be generated.
    pub fn feedback_verbosity(&mut self, x: FeedbackVerbosity) -> &mut Self {
        self.config.feedback_verbosity = x;
        self
    }

    /// Sets the [`InitialValues`] that will be used by [`Game`].
    pub fn init_vals(&mut self, x: InitialValues) -> &mut Self {
        self.seed(x.seed)
            .initial_tetromino_generator(x.initial_tetromino_generator)
            .initial_fall_delay(x.initial_fall_delay)
            .initial_lock_delay(x.initial_lock_delay)
    }

    /// The value to seed the game's PRNG with.
    pub fn seed(&mut self, x: u64) -> &mut Self {
        self.seed = Some(x);
        self
    }
    /// The method (and internal state) of tetromino generation used.
    pub fn initial_tetromino_generator(&mut self, x: TetrominoGenerator) -> &mut Self {
        self.initial_tetromino_generator = Some(x);
        self
    }
    /// The fall delay at the beginning of the game.
    pub fn initial_fall_delay(&mut self, x: ExtDuration) -> &mut Self {
        self.initial_fall_delay = Some(x);
        self
    }
    /// The lock delay at the beginning of the game.
    pub fn initial_lock_delay(&mut self, x: ExtDuration) -> &mut Self {
        self.initial_lock_delay = Some(x);
        self
    }
}
