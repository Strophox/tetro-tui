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
    /// The gravity at which a game should start.
    pub initial_gravity: Option<u32>,
    /// The method (and internal state) of tetromino generation used.
    pub initial_tetromino_generator: Option<TetrominoGenerator>,
    /// The value to seed the game's PRNG with.
    pub seed: Option<u64>,
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
        let init_vals = InitialValues {
            initial_gravity: self.initial_gravity.unwrap_or(1),
            initial_tetromino_generator: self
                .initial_tetromino_generator
                .clone()
                .unwrap_or_default(),
            seed: self.seed.unwrap_or_else(|| rand::rng().next_u64()),
        };
        Game {
            config: self.config.clone(),
            state: State {
                time: Duration::ZERO,
                buttons_pressed: Default::default(),
                board: Board::default(),
                hold_piece: None,
                piece_preview: VecDeque::default(),
                piece_generator: init_vals.initial_tetromino_generator.clone(),
                pieces_locked: [0; 7],
                lines_cleared: 0,
                gravity: init_vals.initial_gravity,
                score: 0,
                consecutive_line_clears: 0,
                rng: GameRng::seed_from_u64(init_vals.seed),
            },
            phase: Phase::Spawning {
                spawn_time: Duration::ZERO,
            },
            init_vals,
            modifiers: modifiers.into_iter().collect(),
        }
    }

    /// Sets the [`Configuration`] that will be used by [`Game`].
    pub fn config(&mut self, x: Configuration) -> &mut Self {
        self.config = x;
        self
    }

    /// Sets the [`InitialValues`] that will be used by [`Game`].
    pub fn init_vals(&mut self, x: InitialValues) -> &mut Self {
        self.seed(x.seed)
            .initial_gravity(x.initial_gravity)
            .initial_tetromino_generator(x.initial_tetromino_generator)
    }

    /// The value to seed the game's PRNG with.
    pub fn seed(&mut self, x: u64) -> &mut Self {
        self.seed = Some(x);
        self
    }

    /// The gravity at which a game should start.
    pub fn initial_gravity(&mut self, x: u32) -> &mut Self {
        self.initial_gravity = Some(x);
        self
    }

    /// The method (and internal state) of tetromino generation used.
    pub fn initial_tetromino_generator(&mut self, x: TetrominoGenerator) -> &mut Self {
        self.initial_tetromino_generator = Some(x);
        self
    }

    /// How many pieces should be pre-generated and accessible/visible in the game state.
    pub fn piece_preview_size(&mut self, x: usize) -> &mut Self {
        self.config.piece_preview_size = x;
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
    /// How much faster than normal drop speed a piece should fall while 'soft drop' is being held.
    pub fn soft_drop_factor(&mut self, x: f64) -> &mut Self {
        self.config.soft_drop_factor = x;
        self
    }
    /// How long each spawned active piece may touch the ground in total until it should lock down
    /// immediately.
    pub fn lock_time_cap_factor(&mut self, x: f64) -> &mut Self {
        self.config.capped_lock_time_factor = x;
        self
    }
    /// How long the game should wait after clearing a line.
    pub fn line_clear_duration(&mut self, x: Duration) -> &mut Self {
        self.config.line_clear_duration = x;
        self
    }
    /// How long the game should wait *additionally* before spawning a new piece.
    pub fn spawn_delay(&mut self, x: Duration) -> &mut Self {
        self.config.spawn_delay = x;
        self
    }
    /// Whether the gravity should be automatically incremented while the game plays.
    pub fn progressive_gravity(&mut self, x: bool) -> &mut Self {
        self.config.progressive_gravity = x;
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
}
