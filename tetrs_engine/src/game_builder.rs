/*!
This module handles creation / initialization / building of [`Game`]s.
*/

use super::*;

/// This builder exposes the ability to configure a new [`Game`] to varying degrees.
///
/// Generally speaking, when using `GameBuilder`, you’ll first call [`GameBuilder::new`] or
/// [`Game::builder`], then chain calls to methods to set each field, then call
/// [`GameBuilder::build`] or [`GameBuilder::build_modded`].
/// This will give you a [`Game`] as specified that you can then use as normal.
/// The `GameBuilder` is not used up and its configuration can be re-used to initialize more [`Game`]s.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameBuilder {
    seed: Option<u64>,
    tetromino_generator: TetrominoGenerator,
    config: Configuration,
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
        let seed = self
            .seed
            .unwrap_or_else(|| rand::RngCore::next_u64(&mut rand::rng()));
        let tetromino_generator = self.tetromino_generator;
        let config = self.config.clone();

        let fall_delay = config.fall_delay_params.calculate(0);
        let lock_delay = config.lock_delay_params.calculate(0);

        Game {
            modifiers: modifiers.into_iter().collect(),
            phase: Phase::Spawning {
                spawn_time: Duration::ZERO,
            },
            state: State {
                time: Duration::ZERO,
                buttons_pressed: [None; Button::VARIANTS.len()],
                rng: GameRng::seed_from_u64(seed),
                piece_generator: tetromino_generator,
                piece_preview: VecDeque::new(),
                piece_held: None,
                board: [Line::default(); Game::HEIGHT],
                fall_delay,
                fall_delay_lowerbound_hit_at_n_lineclears: fall_delay
                    .le(&config.fall_delay_params.lowerbound)
                    .then_some(0),
                lock_delay,
                pieces_locked: [0; Tetromino::VARIANTS.len()],
                lineclears: 0,
                consecutive_line_clears: 0,
                score: 0,
            },
            state_init: StateInitialization {
                seed,
                tetromino_generator,
            },
            config,
        }
    }
}

// Getting a `GameBuilder` blueprint back from an existing `Game`.
impl Game {
    /// Creates a blueprint [`GameBuilder`] and an iterator over current modifier identifiers ([`&str`]s) from which the exact game can potentially be rebuilt.
    pub fn blueprint(&self) -> (GameBuilder, impl Iterator<Item = &str>) {
        let builder = GameBuilder {
            seed: Some(self.state_init.seed),
            tetromino_generator: self.state_init.tetromino_generator,
            config: self.config.clone(),
        };

        let mod_descriptors = self.modifiers.iter().map(|m| m.descriptor.as_str());

        (builder, mod_descriptors)
    }
}

// Gamebuilder: Setter methods.
impl GameBuilder {
    /// The value to seed the game's PRNG with.
    pub fn seed(&mut self, x: u64) -> &mut Self {
        self.seed = Some(x);
        self
    }

    /// The method (and internal state) of tetromino generation used.
    pub fn tetromino_generator(&mut self, x: TetrominoGenerator) -> &mut Self {
        self.tetromino_generator = x;
        self
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
    /// Whether holding a 'rotate' button lets a piece be smoothly spawned in a rotated state,
    /// or holding the 'hold' button lets a piece be swapped immediately before it evens spawns.
    pub fn allow_prespawn_actions(&mut self, x: bool) -> &mut Self {
        self.config.allow_prespawn_actions = x;
        self
    }
    /// The method of tetromino rotation used.
    pub fn rotation_system(&mut self, x: RotationSystem) -> &mut Self {
        self.config.rotation_system = x;
        self
    }
    /// How long the game should take to spawn a new piece.
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
    pub fn fall_delay_params(&mut self, x: DelayParameters) -> &mut Self {
        self.config.fall_delay_params = x;
        self
    }
    /// How many times faster than normal drop speed a piece should fall while 'soft drop' is being held.
    pub fn soft_drop_divisor(&mut self, x: ExtNonNegF64) -> &mut Self {
        self.config.soft_drop_divisor = x;
        self
    }
    /// Specification of how fall delay gets calculated from the rest of the state.
    pub fn lock_delay_params(&mut self, x: DelayParameters) -> &mut Self {
        self.config.lock_delay_params = x;
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
    pub fn lock_reset_cap_factor(&mut self, x: ExtNonNegF64) -> &mut Self {
        self.config.lock_reset_cap_factor = x;
        self
    }
    /// How long the game should take to clear a line.
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
}
