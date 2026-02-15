/*!
# Tetrs Engine

`tetrs_engine` is an implementation of a tetromino game engine, able to handle numerous modern
mechanics.

# Examples

```
use tetrs_engine::*;

// Starting up a game - note that in-game time starts at 0.0s.
let mut game = Game::builder()
    .seed(42)
    /* ...Further optional configuration possible... */
    .build();

// Updating the game with the info that 'left' should be pressed at second 5.0;
// If a piece is in the game, it will try to move left.
game.update(GameTime::from_secs(5.0), Some(ButtonChange::Press(Button::MoveLeft)));

// ...

// Updating the game with the info that no input change has occurred up to second 7.0;
// This updates the game, e.g., pieces fall.
game.update(GameTime::from_secs(7.0), None);

// Read most recent game state;
// This is how a UI can know how to render the board, etc.
let GameState { board, .. } = game.state();
```

TASK: Document all features (including IRS, etc. - cargo feature `serde`).
*/

#![warn(missing_docs)]

pub mod extduration;
pub mod extnonnegf64;
mod game_builder;
mod game_update;
pub mod rotation_system;
pub mod tetromino_generator;

use std::{collections::VecDeque, fmt, num::NonZeroU8, ops, time::Duration};

use rand_chacha::{rand_core::SeedableRng, ChaCha12Rng};

pub use extduration::ExtDuration;
pub use extnonnegf64::ExtNonNegF64;
pub use game_builder::GameBuilder;
pub use rotation_system::RotationSystem;
pub use tetromino_generator::TetrominoGenerator;

/// Abstract identifier for which type of tile occupies a cell in the grid.
pub type TileTypeID = NonZeroU8;
/// The type of horizontal lines of the playing grid.
pub type Line = [Option<TileTypeID>; Game::WIDTH];
// NOTE: Would've liked to use `impl Game { type Board = ...` (https://github.com/rust-lang/rust/issues/8995)
/// The type of the entire two-dimensional playing grid.
pub type Board = [Line; Game::HEIGHT];
/// Coordinates conventionally used to index into the [`Board`], starting in the bottom left.
pub type Coord = (usize, usize);
/// Coordinates offsets that can be [`add`]ed to [`Coord`]inates.
pub type Offset = (isize, isize);

/// The type used to identify points in time in a game's internal timeline.
pub type InGameTime = Duration;
/// The internal RNG used by a game.
pub type GameRng = ChaCha12Rng;
/// Type of underlying functions at the heart of a [`GameModifier`].
pub type GameModFn = dyn FnMut(
    &mut UpdatePoint<&mut Option<ButtonChange>>,
    &mut Configuration,
    &StateInitialization,
    &mut State,
    &mut Phase,
    &mut Vec<FeedbackMsg>,
);
/// The result of a game that ended.
pub type GameResult = Result<Stat, GameOver>;

/// Convenient type alias to denote a collection of [`Feedback`]s associated with some [`GameTime`].
pub type FeedbackMsg = (InGameTime, Feedback);

/// Represents one of the seven "Tetrominos";
///
/// A *tetromino* is a two-dimensional, geometric shape made by
/// connecting four squares (orthogonally / at along the edges).
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tetromino {
    /// 'O'-Tetromino.
    /// Four squares connected as one big square; `⠶`, `██`.
    ///
    /// 'O' has 90° rotational symmetry + 2 axes of mirror symmetry.
    O = 0,
    /// 'I'-Tetromino.
    /// Four squares connected as one straight line; `⡇`, `▄▄▄▄`.
    ///
    /// 'I' has 180° rotational symmetry + 2 axes of mirror symmetry.
    I = 1,
    /// 'S'-Tetromino.
    /// Four squares connected in an 'S'-snaking manner; `⠳`, `▄█▀`.
    ///
    /// 'S' has 180° rotational symmetry + 0 axes of mirror symmetry.
    S = 2,
    /// 'Z'-Tetromino:
    /// Four squares connected in a 'Z'-snaking manner; `⠞`, `▀█▄`.
    ///
    /// 'Z' has 180° rotational symmetry + 0 axes of mirror symmetry.
    Z = 3,
    /// 'T'-Tetromino:
    /// Four squares connected in a 'T'-junction shape; `⠗`, `▄█▄`.
    ///
    /// 'T' has 360° rotational symmetry + 1 axis of mirror symmetry.
    T = 4,
    /// 'L'-Tetromino:
    /// Four squares connected in an 'L'-shape; `⠧`, `▄▄█`.
    ///
    /// 'L' has 360° rotational symmetry + 0 axes of mirror symmetry.
    L = 5,
    /// 'J'-Tetromino:
    /// Four squares connected in a 'J'-shape; `⠼`, `█▄▄`.
    ///
    /// 'J' has 360° rotational symmetry + 0 axes of mirror symmetry.
    J = 6,
}

/// Represents the orientation an active piece can be in.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Orientation {
    /// North.
    N = 0,
    /// East.
    E,
    /// South.
    S,
    /// West.
    W,
}

/// An active tetromino in play.
///
/// Notably, the [`Game`] additionally stores [`LockingData`] corresponding
/// to the main active piece outside this struct.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Piece {
    /// Type of tetromino the active piece is.
    pub tetromino: Tetromino,
    /// In which way the tetromino is re-oriented.
    pub orientation: Orientation,
    /// The position of the active piece on a playing grid.
    pub position: Coord,
}

/// A struct holding information on how certain time 'delay' values progress.
///
/// # Example
/// The formulation used for calculation of fall delay is conceptually:
/// ```
/// let fall_delay = |lineclears| {
///     initial_fall_delay.mul_ennf64(
///         multiplier.get().powf(lineclears) - subtrahend.get() * lineclears
///     )
/// }
/// ```
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DelayParameters {
    /// The duration at which the delay starts.
    base_delay: ExtDuration,
    /// The base factor that gets exponentiated by number of line clears;
    /// `factor ^ lineclears ...`.
    ///
    /// Should be in the range `0.0 ≤ .. ≤ 1.0`, where
    /// - `0.0` means 'zero-out initial delay at every line clear',
    /// - `0.5` means 'halve initial delay for every line clear',
    /// - `1.0` means 'keep initial delay at 100%'.
    factor: ExtNonNegF64,
    /// The base subtrahend that gets multiplied by number of line clears;
    /// `... - subtrahend * lineclears`.
    ///
    /// Should be in the range `0.0 ≤ .. ≤ 1.0`, where
    /// - `0.0` means 'subtract 0% of initial delay for every line clear',
    /// - `0.5` means 'subtract 50% of initial delay for every line clear',
    /// - `1.0` means 'subtract 100% of initial delay for every line clear'.
    subtrahend: ExtDuration,
    /// The duration below which delay cannot decrease.
    lowerbound: ExtDuration,
}

/// Certain statistics for which an instance of [`Game`] can be checked against.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Stat {
    /// Whether a given amount of total time has elapsed in-game.
    TimeElapsed(InGameTime),
    /// Whether a given number of [`Tetromino`]s have been locked/placed on the game's [`Board`].
    PiecesLocked(u32),
    /// Whether a given number of lines have been cleared from the [`Board`].
    LinesCleared(u32),
    /// Whether a given number of points has been scored already.
    PointsScored(u32),
}

/// The amount of feedback information that is to be generated.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FeedbackVerbosity {
    /// No feedback generated by base engine.
    /// Note that game modifiers called may choose to generate feedback messages
    /// themselves, which will not again be discarded once received by
    /// the base game engine.
    Silent,
    /// Base level of feedback about in-game events.
    #[default]
    Default,
    /// Highest level of feedback, which includes emitting every
    /// internal game event processed
    Debug,
}

/// Configuration options of the game, which can be modified without hurting internal invariants.
///
/// # Reproducibility
/// Modifying a [`Game`]'s configuration after it was created might not make it easily
/// reproducible anymore.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Configuration {
    /// How many pieces should be pre-generated and accessible/visible in the game state.
    pub piece_preview_count: usize,
    /// Whether holding a 'rotate' button lets a piece be smoothly spawned in a rotated state,
    /// or holding the 'hold' button lets a piece be swapped immediately before it evens spawns.
    pub allow_prespawn_actions: bool,
    /// The method of tetromino rotation used.
    pub rotation_system: RotationSystem,
    /// How long the game should take to spawn a new piece.
    pub spawn_delay: Duration,
    /// How long it takes for the active piece to start automatically shifting more to the side
    /// after the initial time a 'move' button has been pressed.
    pub delayed_auto_shift: Duration,
    /// How long it takes for automatic side movement to repeat once it has started.
    pub auto_repeat_rate: Duration,
    /// Specification of how fall delay gets calculated from the rest of the state.
    pub fall_delay_params: DelayParameters,
    /// How many times faster than normal drop speed a piece should fall while 'soft drop' is being held.
    pub soft_drop_divisor: ExtNonNegF64,
    /// Specification of how fall delay gets calculated from the rest of the state.
    pub lock_delay_params: DelayParameters,
    /// Whether just pressing a rotation- or movement button is enough to refresh lock delay.
    /// Normally, lock delay only resets if rotation or movement actually succeeds.
    pub lenient_lock_delay_reset: bool,
    /// How long each spawned active piece may touch the ground in total until it should lock down
    /// immediately.
    pub lock_reset_cap_factor: ExtNonNegF64,
    /// How long the game should take to clear a line.
    pub line_clear_duration: Duration,
    /// When to update the fall and lock delays in [`State`].
    pub update_delays_every_n_lineclears: u32,
    /// Stores the ways in which a round of the game should be limited.
    ///
    /// Each limitation may be either of positive ('game completed') or negative ('game over'), as
    /// designated by the `bool` stored with it.
    ///
    /// No limitations may allow for endless games.
    pub end_conditions: Vec<(Stat, bool)>,
    /// The amount of feedback information that is to be generated.
    pub feedback_verbosity: FeedbackVerbosity,
}

/// Some values that were used to help initialize the game.
///
/// Used for game reproducibility.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StateInitialization {
    /// The value to seed the game's PRNG with.
    pub seed: u64,
    /// The method (and internal state) of tetromino generation used.
    pub tetromino_generator: TetrominoGenerator,
}

/// Represents an abstract game input.
// NOTE: We could consider calling this `Action` judging from its variants, however the Game stores a mapping of whether a given `Button` is active over a period of time. `Intents` could work but `Button` is less abstract and often corresponds directly to IRL player inputs.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Button {
    /// Moves the piece once to the left.
    MoveLeft = 0,
    /// Moves the piece once to the right.
    MoveRight,
    /// Rotate the piece by +90° (clockwise).
    RotateLeft,
    /// Rotate the piece by -90° (counter-clockwise).
    RotateRight,
    /// Rotate the piece by 180° (flip around).
    RotateAround,
    /// "Soft" dropping.
    /// This drops a piece down by one, locking it immediately if it hit a surface,
    /// Otherwise holding this button decreases fall speed by the game [`Configuration`]'s `soft_drop_factor`.
    DropSoft,
    /// "Hard" dropping.
    /// This immediately drops a piece all the way down until it hits a surface,
    /// locking it there (almost) instantly, too.
    DropHard,
    /// Teleport the piece down, also known as "Sonic" dropping.
    /// This immediately drops a piece all the way down until it hits a surface,
    /// but without locking it (unlike [`Button::DropHard`]).
    TeleDown,
    /// Instantly 'teleports' (moves) a piece left until it hits a surface.
    TeleLeft,
    /// Instantly 'teleports' (moves) a piece right until it hits a surface.
    TeleRight,
    /// Holding the current piece; and swapping in a new piece if one was held previously.
    HoldPiece,
}

/// A change in button state, between being held down or unpressed.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ButtonChange {
    /// The signal of a button now being active / 'pressed down'.
    Press(Button),
    /// The signal of a button now being inactive / 'not pressed down'.
    Release(Button),
}

/// Struct storing internal game state that changes over the course of play.
#[derive(Eq, PartialEq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct State {
    /// Current in-game time.
    pub time: InGameTime,
    /// The current state of buttons being pressed in the game.
    pub buttons_pressed: [Option<InGameTime>; Button::VARIANTS.len()],
    /// The internal pseudo random number generator used.
    pub rng: GameRng,
    /// The method (and internal state) of tetromino generation used.
    pub piece_generator: TetrominoGenerator,
    /// Upcoming pieces to be played.
    pub piece_preview: VecDeque<Tetromino>,
    /// Data about the piece being held. `true` denotes that the held piece can be swapped back in.
    pub piece_held: Option<(Tetromino, bool)>,
    /// The main playing grid storing empty (`None`) and filled, fixed tiles (`Some(nz_u32)`).
    pub board: Board,
    /// The current duration a piece takes to fall one unit.
    pub fall_delay: ExtDuration,
    /// The point (number of lines cleared) at which fall delay was updated to zero (possibly capped if formula yielded negative).
    pub fall_delay_lowerbound_hit_at_n_lineclears: Option<u32>,
    /// The current duration a piece takes to try and lock down.
    pub lock_delay: ExtDuration,
    /// Tallies of how many pieces of each type have been played so far.
    pub pieces_locked: [u32; Tetromino::VARIANTS.len()],
    /// The total number of lines that have been cleared.
    pub lineclears: u32,
    /// The number of consecutive pieces that have been played and caused a line clear.
    pub consecutive_line_clears: u32,
    /// The current total score the player has achieved in this round of play.
    pub score: u32,
}

/// Represents how a game can end.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GameOver {
    /// 'Lock out' denotes the most recent piece being completely locked down at
    /// or above the [`Game::SKYLINE`].
    LockOut,
    /// 'Block out' denotes a new piece being unable to spawn due to pre-existing board tile
    /// blocking one or several of the spawn cells.
    BlockOut,
    /// Generic game over by having reached a (negative) game limit.
    Limit(Stat),
    /// Generic game over by player forfeit.
    Forfeit,
}

/// Locking details stored about an active piece in play.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PieceData {
    /// The tetromino game piece itself.
    pub piece: Piece,
    /// The time of the next fall or lock event.
    pub fall_or_lock_time: InGameTime,
    /// Whether `fall_or_lock_time` refers to a fall or lock event.
    pub is_fall_not_lock: bool,
    /// The lowest recorded vertical position of the main piece.
    pub lowest_y: usize,
    /// The time after which the active piece will immediately lock upon touching ground.
    pub capped_lock_time: InGameTime,
    /// Optional time of the next move event.
    pub auto_move_scheduled: Option<InGameTime>,
}

/// An event that is scheduled by the game engine to execute some action.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Phase {
    /// The state of the game "taking its time" to spawn a piece.
    /// This is the state the board will have right before attempting to spawn a new piece.
    Spawning {
        /// The in-game time at which the game moves on to the next `Phase.`
        spawn_time: InGameTime,
    },
    /// The state of the game having an active piece in-play, which can be controlled by a player.
    PieceInPlay {
        /// The data required to play a piece in this `Phase.`
        piece_data: PieceData,
    },
    /// The state of the game "taking its time" to clear out lines.
    /// In this state the board is as it was at the time of the piece locking down,
    /// i.e. with some horizontally completed lines.
    /// After exiting this state, the
    LinesClearing {
        /// The in-game time at which the game moves on to the next `Phase.`
        line_clears_finish_time: InGameTime,
    },
    /// The state of the game being irreversibly over, and not playable anymore.
    GameEnd {
        /// The result of how the game ended.
        result: GameResult,
    },
}

/// Represents a specific point which was reached in a call to [`Game::update`].
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UpdatePoint<T> {
    /// Represents a `Game::update` call handling [`Phase::Spawning`].
    PieceSpawned,
    /// Represents a `Game::update` call handling [`Phase::PieceInPlay`], specifically a piece moving autonomously (DAS/ARR).
    PieceAutoMoved,
    /// Represents a `Game::update` call handling [`Phase::PieceInPlay`], specifically a piece falling autonomously.
    PieceFell,
    /// Represents a `Game::update` call handling [`Phase::PieceInPlay`], specifically a piece locking down.
    PieceLocked,
    /// Represents a `Game::update` call handling [`Phase::PieceInPlay`], specifically an update ([`ButtonChange`]) to the state of [`Button`]s by the player.
    PiecePlayed(ButtonChange),
    /// Represents a `Game::update` call handling [`Phase::LinesClearing`].
    LinesCleared,
    /// Represents a `Game::update` call at a general point at the head of the main loop.
    /// Typically:
    /// * `T = &mut Option<ButtonChange>` for [`GameModFn`] purposes, or
    /// * `T = String` for [`Feedback`] purposes.
    ///
    /// # Reproducibility
    /// Note that this update point is called every time [`Game::update`] is called.
    /// Unlike other update points, this makes a modifier not reproducible if its behavior depends on the number of times it processes this update point.
    MainLoopHead(T),
}

/// Type of named modifiers that can be used to mod a game, c.f. [`GameBuilder::build_modified`].
pub struct Modifier {
    /// Given a function which produces a modifier ready to be attached to a game,
    /// ```rust
    /// fn modifier(arg1: T1, ..., argX: TX) -> Modifier;
    /// ```
    /// or alternatively a builder function which builds a modified game,
    /// ```rust
    /// fn build(builder: &GameBuilder, arg1: T1, ..., argX: TX) -> Game;
    /// ```
    /// Then, by convention, the modifier descriptor should be produced as
    /// ```rust
    /// let mod_args = serde_json::to_string(&(arg1, ..., argX)).unwrap();
    /// let descriptor = format!("{MOD_ID}\n{mod_args}");
    /// ``````
    /// In other words, the descriptor is intended to contain information to:
    /// * Identify a modifier.
    /// * Possibly reconstruct the modifier using its original arguments.
    ///
    /// This is mostly relevant if the mod is not only reconstructible,
    /// but also reproducible: In this case not only do the original arguments
    /// have to match to produce the same initial state of the modifier, but the
    /// modifier itself needs to act deterministically (e.g. if it uses random
    /// elements by tapping into [`GameRng`] as opposed to global `thread_rng`).
    ///
    /// This is a convention; There are no restrictions on whether a modifier
    /// is actually reconstructible or reproducible.
    pub descriptor: String,
    /// The function object which will be called at runtime.
    /// ```rust
    /// mod_function = |point, config, init_vals, state, phase, msgs| { /* ... */ };
    /// ```
    ///
    /// See documentation of [`UpdatePoint`].
    pub mod_function: Box<GameModFn>,
}

/// Main game struct representing a round of play.
#[derive(Debug)]
pub struct Game {
    /// Some internal configuration options of the `Game`.
    ///
    /// # Reproducibility
    /// Modifying a `Game`'s configuration after it was created might not make it easily
    /// reproducible anymore.
    pub config: Configuration,
    state_init: StateInitialization,
    state: State,
    phase: Phase,
    /// A list of special modifiers that apply to the `Game`.
    ///
    /// # Reproducibility
    /// Modifying a `Game`'s modifiers after it was created might not make it easily
    /// reproducible anymore.
    pub modifiers: Vec<Modifier>,
}

/// A number of feedback events that can be returned by the game.
///
/// These can be used to more easily render visual feedback to the player.
/// The [`EngineEvent`] and [`EngineInput`] variants are currently accessible if [`FeedbackVerbosity::Debug`] is toggled.
/// All other events are generally variants of [`EngineEvents`] but providing additional info to reconstruct
/// a visual effect (e.g. location of where a lock actually occurred).
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Feedback {
    /// A piece was locked down in a certain configuration.
    PieceLocked {
        /// Information about the [`Piece`] that was locked.
        piece: Piece,
    },
    /// A number of lines were cleared.
    ///
    /// The duration indicates the line clear delay the game was configured with at the time.
    LinesClearing {
        /// A list of height coordinates/indices signifying where lines where cleared.
        y_coords: Vec<usize>,
        /// Game time where lines started clearing.
        /// Starts simultaneously to when a piece was locked and successfully completed some horizontal [`Line`]s,
        /// therefore this will coincide with the time same value in a nearby [`Feedback::PieceLocked`].
        line_clear_start: InGameTime,
    },
    /// A piece was quickly dropped from its original position to a new one.
    HardDrop {
        /// Information about the old state of the hard-dropped piece.
        old_piece: Piece,
        /// Information about the new state of the hard-dropped piece.
        new_piece: Piece,
    },
    /// The player cleared some lines with a number of other stats that might have increased their
    /// score bonus.
    Accolade {
        /// The final computed score bonus caused by the action.
        score_bonus: u32,
        /// How many lines were cleared by the piece simultaneously
        lineclears: u32,
        /// The number of consecutive pieces played that caused a lineclear.
        combo: u32,
        /// Whether the piece was spun into place.
        is_spin: bool,
        /// Whether the entire board was cleared empty by this action.
        is_perfect_clear: bool,
        /// The tetromino type that was locked.
        tetromino: Tetromino,
    },
    /// Message that the game has ended.
    GameEnded {
        /// Outcome of the game.
        result: GameResult,
    },
    /// A message containing an exact in-engine `UpdatePoint` that was processed.
    Debug(UpdatePoint<String>),
    /// Generic text feedback message.
    ///
    /// This is currently unused in the base engine.
    Text(String),
}

/// An error that can be thrown by [`Game::update`].
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
pub enum UpdateGameError {
    /// Error variant caused by an attempt to update the game with a requested `update_time` that lies in
    /// the game's past (` < game.state().time`).
    TargetTimeInPast,
    /// Error variant caused by an attempt to update a game that has ended (`game.ended() == true`).
    GameEnded,
}

impl Tetromino {
    /// All `Tetromino` enum variants in order.
    ///
    /// Note that `Tetromino::VARIANTS[t as usize] == t` always holds.
    pub const VARIANTS: [Self; 7] = {
        use Tetromino::*;
        [O, I, S, Z, T, L, J]
    };

    /// Returns the mino offsets of a tetromino shape, given an orientation.
    pub const fn minos(&self, oriented: Orientation) -> [Coord; 4] {
        use Orientation::*;
        match self {
            Tetromino::O => [(0, 0), (1, 0), (0, 1), (1, 1)], // ⠶
            Tetromino::I => match oriented {
                N | S => [(0, 0), (1, 0), (2, 0), (3, 0)], // ⠤⠤
                E | W => [(0, 0), (0, 1), (0, 2), (0, 3)], // ⡇
            },
            Tetromino::S => match oriented {
                N | S => [(0, 0), (1, 0), (2, 1), (1, 1)], // ⠴⠂
                E | W => [(1, 0), (0, 1), (1, 1), (0, 2)], // ⠳
            },
            Tetromino::Z => match oriented {
                N | S => [(1, 0), (2, 0), (0, 1), (1, 1)], // ⠲⠄
                E | W => [(0, 0), (1, 1), (0, 1), (1, 2)], // ⠞
            },
            Tetromino::T => match oriented {
                N => [(0, 0), (1, 0), (2, 0), (1, 1)], // ⠴⠄
                E => [(0, 0), (1, 1), (0, 1), (0, 2)], // ⠗
                S => [(1, 0), (0, 1), (2, 1), (1, 1)], // ⠲⠂
                W => [(1, 0), (0, 1), (1, 1), (1, 2)], // ⠺
            },
            Tetromino::L => match oriented {
                N => [(0, 0), (1, 0), (2, 0), (2, 1)], // ⠤⠆
                E => [(0, 0), (1, 0), (0, 1), (0, 2)], // ⠧
                S => [(0, 0), (1, 1), (2, 1), (0, 1)], // ⠖⠂
                W => [(1, 0), (0, 2), (1, 1), (1, 2)], // ⠹
            },
            Tetromino::J => match oriented {
                N => [(0, 0), (1, 0), (2, 0), (0, 1)], // ⠦⠄
                E => [(0, 0), (1, 2), (0, 1), (0, 2)], // ⠏
                S => [(2, 0), (0, 1), (1, 1), (2, 1)], // ⠒⠆
                W => [(0, 0), (1, 0), (1, 1), (1, 2)], // ⠼
            },
        }
    }

    /// Returns the convened-on standard tile id corresponding to the given tetromino.
    pub const fn tiletypeid(&self) -> TileTypeID {
        use Tetromino::*;
        let u8 = match self {
            O => 1,
            I => 2,
            S => 3,
            Z => 4,
            T => 5,
            L => 6,
            J => 7,
        };
        // SAFETY: Ye, `u8 > 0`;
        unsafe { NonZeroU8::new_unchecked(u8) }
    }
}

impl Orientation {
    /// All `Orientation` enum variants in order.
    ///
    /// Note that `Orientation::VARIANTS[o as usize] == o` always holds.
    pub const VARIANTS: [Self; 4] = {
        use Orientation::*;
        [N, E, S, W]
    };

    /// Find a new direction by turning right some number of times.
    ///
    /// This accepts `i32` to allow for left rotation.
    pub const fn reorient_right(&self, right_turns: i8) -> Self {
        Orientation::VARIANTS[((*self as i8 + right_turns) as usize).rem_euclid(4)]
    }
}

impl Piece {
    /// Returns the coordinates and tile types for he piece on the board.
    pub fn tiles(&self) -> [(Coord, TileTypeID); 4] {
        let Self {
            tetromino,
            orientation,
            position: (x, y),
        } = self;
        let tile_type_id = tetromino.tiletypeid();
        tetromino
            .minos(*orientation)
            .map(|(dx, dy)| ((x + dx, y + dy), tile_type_id))
    }

    /// Checks whether the piece fits at its current location onto the board.
    pub fn fits(&self, board: &Board) -> bool {
        self.tiles()
            .iter()
            .all(|&((x, y), _)| x < Game::WIDTH && y < Game::HEIGHT && board[y][x].is_none())
    }

    /// Checks whether the piece fits a given offset from its current location onto the board.
    pub fn fits_at(&self, board: &Board, offset: Offset) -> Option<Piece> {
        let mut new_piece = *self;
        new_piece.position = add(self.position, offset)?;
        new_piece.fits(board).then_some(new_piece)
    }

    /// Checks whether the piece fits a given offset from its current location onto the board, with
    /// its rotation changed by some number of right turns.
    pub fn fits_at_reoriented(
        &self,
        board: &Board,
        offset: Offset,
        right_turns: i8,
    ) -> Option<Piece> {
        let mut new_piece = *self;
        new_piece.orientation = new_piece.orientation.reorient_right(right_turns);
        new_piece.position = add(self.position, offset)?;
        new_piece.fits(board).then_some(new_piece)
    }

    /// Given an iterator over some offsets, checks whether the rotated piece fits at any offset
    /// location onto the board.
    pub fn first_fit(
        &self,
        board: &Board,
        offsets: impl IntoIterator<Item = Offset>,
        right_turns: i8,
    ) -> Option<Piece> {
        let mut new_piece = *self;
        new_piece.orientation = new_piece.orientation.reorient_right(right_turns);
        let old_pos = self.position;
        offsets.into_iter().find_map(|offset| {
            new_piece.position = add(old_pos, offset)?;
            new_piece.fits(board).then_some(new_piece)
        })
    }

    /// Returns the position the piece would hit if it kept moving at `offset` steps.
    /// For offset `(0,0)` this function return immediately.
    pub fn teleported(&self, board: &Board, offset: Offset) -> Piece {
        let mut piece = *self;
        if offset != (0, 0) {
            // Move piece as far as possible.
            while let Some(new_piece) = piece.fits_at(board, offset) {
                piece = new_piece;
            }
        }
        piece
    }
}

impl DelayParameters {
    /// The duration at which the delay starts.
    pub fn base_delay(&self) -> ExtDuration {
        self.base_delay
    }

    /// The base factor that gets exponentiated by number of line clears;
    /// `factor ^ lineclears ...`.
    ///
    /// Should be in the range `0.0 ≤ .. ≤ 1.0`, where
    /// - `0.0` means 'zero-out initial delay at every line clear',
    /// - `0.5` means 'halve initial delay for every line clear',
    /// - `1.0` means 'keep initial delay at 100%'.
    pub fn factor(&self) -> ExtNonNegF64 {
        self.factor
    }

    /// The base subtrahend that gets multiplied by number of line clears;
    /// `... - subtrahend * lineclears`.
    ///
    /// Should be in the range `0.0 ≤ .. ≤ 1.0`, where
    /// - `0.0` means 'subtract 0% of initial delay for every line clear',
    /// - `0.5` means 'subtract 50% of initial delay for every line clear',
    /// - `1.0` means 'subtract 100% of initial delay for every line clear'.
    pub fn subtrahend(&self) -> ExtDuration {
        self.subtrahend
    }

    /// The duration below which delay cannot decrease.
    pub fn lowerbound(&self) -> ExtDuration {
        self.lowerbound
    }

    /// Delay equation which decreases/decays exponentially in number of linescleared.
    pub fn new(
        base_delay: ExtDuration,
        lowerbound: ExtDuration,
        factor: ExtNonNegF64,
        subtrahend: ExtDuration,
    ) -> Option<Self> {
        Self::constant(Default::default())
            .with_bounds(base_delay, lowerbound)?
            .with_coefficients(factor, subtrahend)
    }

    /// Create a modified delay parameters where only the bounds are changed.
    pub fn with_bounds(&self, base_delay: ExtDuration, lowerbound: ExtDuration) -> Option<Self> {
        let correct_bounds = lowerbound <= base_delay;
        correct_bounds.then_some(Self {
            base_delay,
            lowerbound,
            ..*self
        })
    }

    /// Create a modified delay parameters where only the coefficients are changed.
    pub fn with_coefficients(&self, factor: ExtNonNegF64, subtrahend: ExtDuration) -> Option<Self> {
        let correct_coefficients = factor <= 1.into();
        correct_coefficients.then_some(Self {
            factor,
            subtrahend,
            ..*self
        })
    }

    /// Delay equation which does not change at all with number of linescleared.
    pub fn constant(delay: ExtDuration) -> Self {
        Self {
            base_delay: delay,
            factor: 1.into(),
            subtrahend: ExtDuration::ZERO,
            lowerbound: delay,
        }
    }

    /// Whether the delay curve is invariant to number of lineclears.
    pub fn is_constant(&self) -> bool {
        self.factor == 1.into() && self.subtrahend.is_zero()
    }

    /// Delay equation which implements guideline-like fall delays:
    /// *   0.0  lineclears ~> 20s to fall 20 units (1s/unit).
    /// *  28.8_ lineclears ~> 10s to fall 20 units.
    /// *  94.4_ lineclears ~>  2s to fall 20 units.
    /// * 120.9_ lineclears ~>  1s to fall 20 units.
    /// * 156.8_ lineclears ~> 1/3s to fall 20 units (NES max; 1 unit/frame).
    /// * 196.1_ lineclears ~> 1/60s to fall 20 units (1frame/20units).
    /// * 199.4_ lineclears ~>  0s to fall (instant gravity).
    pub fn standard_fall() -> Self {
        Self {
            base_delay: Duration::from_millis(1000).into(),
            factor: ExtNonNegF64::new(0.9763).unwrap(),
            subtrahend: Duration::from_secs_f64(0.000042).into(),
            lowerbound: Duration::ZERO.into(),
        }
    }

    /// Delay equation which implements guideline-like lock delays:
    /// * 0 lineclears ~> 500ms lock delay.
    /// * Decrease lock_delay by 20 ms every 10 lineclears (= 2 ms every lineclear).
    /// * End at 200ms lock delay.
    pub fn standard_lock() -> Self {
        Self {
            base_delay: Duration::from_millis(500).into(),
            factor: 1.into(),
            subtrahend: Duration::from_millis(2).into(),
            lowerbound: Duration::from_millis(200).into(),
        }
    }

    /// Calculates an actual delay value given a number of lineclears to determine progression.
    pub fn calculate(&self, lineclears: u32) -> ExtDuration {
        // Multiplicative factor computed from lineclears;
        let raw_mul = self.factor.get().powf(f64::from(lineclears));
        // Wrap it back in ExtNonNegF64.
        let mul = ExtNonNegF64::new(raw_mul).unwrap();

        // Subtractive offset computed from lineclears.
        let sub = self.subtrahend.mul_ennf64(lineclears.into());

        // Calculate intended delay;
        let raw_delay = self.base_delay.mul_ennf64(mul).saturating_sub(sub);
        // Return delay capped by lower bound.
        self.lowerbound.max(raw_delay)
    }
}

impl Button {
    /// All `Button` enum variants.
    ///
    /// Note that `Button::VARIANTS[b as usize] == b` always holds.
    pub const VARIANTS: [Self; 11] = {
        use Button as B;
        [
            B::MoveLeft,
            B::MoveRight,
            B::RotateLeft,
            B::RotateRight,
            B::RotateAround,
            B::DropSoft,
            B::DropHard,
            B::TeleDown,
            B::TeleLeft,
            B::TeleRight,
            B::HoldPiece,
        ]
    };
}

impl<T> ops::Index<Button> for [T; Button::VARIANTS.len()] {
    type Output = T;

    fn index(&self, idx: Button) -> &Self::Output {
        &self[idx as usize]
    }
}

impl<T> ops::IndexMut<Button> for [T; Button::VARIANTS.len()] {
    fn index_mut(&mut self, idx: Button) -> &mut Self::Output {
        &mut self[idx as usize]
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            piece_preview_count: 4,
            allow_prespawn_actions: true,
            rotation_system: RotationSystem::default(),
            spawn_delay: Duration::from_millis(50),
            delayed_auto_shift: Duration::from_millis(167),
            auto_repeat_rate: Duration::from_millis(33),
            fall_delay_params: DelayParameters::constant(Duration::from_millis(1000).into()),
            soft_drop_divisor: ExtNonNegF64::new(10.0).unwrap(),
            lock_delay_params: DelayParameters::constant(Duration::from_millis(500).into()),
            lenient_lock_delay_reset: false,
            lock_reset_cap_factor: ExtNonNegF64::new(8.0).unwrap(),
            line_clear_duration: Duration::from_millis(200),
            update_delays_every_n_lineclears: 10,
            end_conditions: Default::default(),
            feedback_verbosity: FeedbackVerbosity::default(),
        }
    }
}

impl Phase {
    /// Read accessor to a `Phase`'s possible [`Piece`].
    pub fn piece(&self) -> Option<&Piece> {
        if let Phase::PieceInPlay {
            piece_data: PieceData { piece, .. },
            ..
        } = self
        {
            Some(piece)
        } else {
            None
        }
    }

    /// Mutable accessor to a `Phase`'s possible [`Piece`].
    pub fn piece_mut(&mut self) -> Option<&mut Piece> {
        if let Phase::PieceInPlay {
            piece_data: PieceData { piece, .. },
            ..
        } = self
        {
            Some(piece)
        } else {
            None
        }
    }
}

impl fmt::Debug for Modifier {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Modifier")
            .field("descriptor", &self.descriptor)
            .field(
                "mod_function",
                &std::any::type_name_of_val(&self.mod_function),
            )
            .finish()
    }
}

impl Game {
    /// The maximum height *any* piece tile could reach before [`GameOver::LockOut`] occurs.
    pub const HEIGHT: usize = Self::SKYLINE_HEIGHT + 7;
    /// The game field width.
    pub const WIDTH: usize = 10;
    /// The height of the (conventionally visible) playing grid that can be played in.
    /// No piece may be locked entirely above the `SKYLINE`, although it may do so partially.
    pub const SKYLINE_HEIGHT: usize = 20;

    /// Creates a blank new template representing a yet-to-be-started [`Game`] ready for configuration.
    pub fn builder() -> GameBuilder {
        GameBuilder::default()
    }

    /// Read accessor for the game's initial values.
    pub const fn state_init(&self) -> &StateInitialization {
        &self.state_init
    }

    /// Read accessor for the current game state.
    pub const fn state(&self) -> &State {
        &self.state
    }

    /// Read accessor for the current game state.
    pub const fn phase(&self) -> &Phase {
        &self.phase
    }

    /// Whether the game has ended, and whether it can continue to update.
    pub const fn result(&self) -> Option<GameResult> {
        match self.phase {
            Phase::GameEnd { result } => Some(result),
            _ => None,
        }
    }

    /// Retrieve the when the next *autonomous* in-game update is scheduled.
    /// I.e., compute the next time the game would change state assuming no button updates
    ///
    /// Returns `None` when game ended.
    ///
    /// # Modifiers
    /// Note that this only predicts what an unmodded game would do;
    /// [`Modifier`]s may arbitrarily change game state and change or prevent precise update predictions.
    pub fn peek_next_update_time(&self) -> Option<InGameTime> {
        // Find the next autonomous game update.
        let action_time = match self.phase {
            Phase::GameEnd { .. } => return None,
            Phase::LinesClearing {
                line_clears_finish_time,
            } => line_clears_finish_time,
            Phase::Spawning { spawn_time } => spawn_time,
            Phase::PieceInPlay { piece_data } => {
                if let Some(move_time) = piece_data.auto_move_scheduled {
                    if move_time < piece_data.fall_or_lock_time {
                        move_time
                    } else {
                        piece_data.fall_or_lock_time
                    }
                } else {
                    piece_data.fall_or_lock_time
                }
            }
        };

        // Find all time-related end conditions.
        let time_limits = self.config.end_conditions.iter().filter_map(|(stat, _)| {
            if let Stat::TimeElapsed(cap_time) = stat {
                Some(cap_time)
            } else {
                None
            }
        });

        let mut min_update_time = action_time;
        for &time_limit in time_limits {
            if min_update_time > time_limit {
                min_update_time = time_limit;
            }
        }

        Some(min_update_time)
    }

    /// Check whether a certain stat value has been met or exceeded.
    pub fn check_stat_met(&self, stat: &Stat) -> bool {
        match stat {
            Stat::TimeElapsed(t) => *t <= self.state.time,
            Stat::PiecesLocked(p) => *p <= self.state.pieces_locked.iter().sum(),
            Stat::LinesCleared(l) => *l <= self.state.lineclears,
            Stat::PointsScored(s) => *s <= self.state.score,
        }
    }

    /// Immediately end a game by forfeiting the current round.
    ///
    /// This can be used so `game.ended()` returns true and prevents future
    /// calls to `update` from continuing to advance the game.
    pub const fn forfeit(&mut self) -> FeedbackMsg {
        self.phase = Phase::GameEnd {
            result: Err(GameOver::Forfeit),
        };

        (
            self.state.time,
            Feedback::GameEnded {
                result: Err(GameOver::Forfeit),
            },
        )
    }

    /// Creates an identical, independent copy of the game but without any modifiers.
    pub fn clone_unmodded(&self) -> Self {
        Self {
            config: self.config.clone(),
            state_init: self.state_init,
            state: self.state.clone(),
            phase: self.phase,
            modifiers: Vec::new(),
        }
    }
}

impl std::fmt::Display for UpdateGameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UpdateGameError::TargetTimeInPast => {
                "attempt to update game to timestamp it already passed"
            }
            UpdateGameError::GameEnded => "attempt to update game after it ended",
        };
        write!(f, "{s}")
    }
}

impl std::error::Error for UpdateGameError {}

/// Adds an offset to a board coordinate, failing if the result is out of bounds
/// (negative or positive overflow in either direction).
pub fn add((x, y): Coord, (dx, dy): Offset) -> Option<Coord> {
    Some((x.checked_add_signed(dx)?, y.checked_add_signed(dy)?))
}

/*#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let res = add((1,2),(3,4));
        assert_eq!(res, (4,6));
    }
}*/
