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

// Receive the information from I/O that 'left' was pressed.
let mut button_state = PressedButtons::default();
button_state[Button::MoveLeft] = true;

// Updating the game with the info that 'left' should be pressed at second 5.0;
// If a piece is in the game, it will try to move left.
game.update(Some(button_state), GameTime::from_secs(5.0));

// ...

// Updating the game with the info that no input change has occurred up to second 7.0;
// This updates the game, e.g., pieces fall.
game.update(None, GameTime::from_secs(7.0));

// Read most recent game state;
// This is how a UI can know how to render the board, etc.
let GameState { board, .. } = game.state();
```

TASK: Document all features (including IRS, etc. - cargo feature `serde`).
*/

#![warn(missing_docs)]

pub mod game_update_step;
pub mod rotation_system;
pub mod tetromino_generator;

use std::{
    collections::{HashMap, VecDeque},
    fmt,
    num::{NonZeroU32, NonZeroU8},
    ops,
    time::Duration,
};

use rand_chacha::{
    rand_core::{RngCore, SeedableRng},
    ChaCha12Rng,
};
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
pub type GameTime = Duration;
/// The internal RNG used by a game.
pub type GameRng = ChaCha12Rng;
/// Type of underlying functions at the heart of a [`GameModifier`].
pub type GameModFn = dyn FnMut(
    &mut Configuration,
    &mut InitialValues,
    &mut State,
    &UpdatePoint,
    &mut FeedbackMessages,
);
/// A set of conditions to determine how a game specially ends and whether it results in a win (otherwise loss).
pub type EndConditions = Vec<(Stat, bool)>;
/// The result of a game that ended.
pub type GameResult = Result<(), GameOver>;
/// A mapping for buttons, usable through `impl Index<Button>`.
type ButtonsArray<T> = [T; Button::VARIANTS.len()];
/// A mapping for which buttons were pressed.
pub type PressedButtons = ButtonsArray<bool>;
/// Convenient type alias to denote a collection of [`Feedback`]s associated with some [`GameTime`].
pub type FeedbackMessages = Vec<(GameTime, Feedback)>;

/// Represents an abstract game input.
// NOTE: We could consider calling this `Action` judging from its variants, however the Game stores a mapping of whether a given `Button` is active over a period of time. `Intents` could work but `Button` is less abstract and often corresponds directly to IRL player inputs.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Button {
    /// Movement to the left.
    MoveLeft,
    /// Movement to the right.
    MoveRight,
    /// Rotation by 90° counter-clockwise.
    RotateLeft,
    /// Rotation by 90° clockwise.
    RotateRight,
    /// Rotation by 180°.
    RotateAround,
    /// "Soft" dropping.
    /// This conventionally drops a piece down by one, afterwards continuing to
    /// drop at sped-up rate while held.
    DropSoft,
    /// "Hard" dropping.
    /// This conventionally drops a piece straight down until it hits a surface,
    /// locking it there (almost) immediately.
    DropHard,
    /// "Sonic" dropping.
    /// This conventionally drops a piece straight down until it hits a surface,
    /// **without** locking it immediately or performing any other special handling
    /// with respect to locking.
    DropSonic,
    /// Holding and swapping in a held piece.
    HoldPiece,
}

/// Represents the orientation an active piece can be in.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Orientation {
    /// North.
    N,
    /// East.
    E,
    /// South.
    S,
    /// West.
    W,
}

/// Represents one of the seven playable piece shapes.
///
/// A "Tetromino" is a two-dimensional shape made from connecting exactly
/// four square tiles into one rigid piece.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tetromino {
    /// 'O'-Tetromino: Four tiles arranged in one big square; '⠶', `██`.
    O,
    /// 'I'-Tetromino: Four tiles arranged in one straight line; '⡇', `▄▄▄▄`.
    I,
    /// 'S'-Tetromino: Four tiles arranged in a left-snaking manner; '⠳', `▄█▀`.
    S,
    /// 'Z'-Tetromino: Four tiles arranged in a right-snaking manner; '⠞', `▀█▄`.
    Z,
    /// 'T'-Tetromino: Four tiles arranged in a 'T'-shape; '⠗', `▄█▄`.
    T,
    /// 'L'-Tetromino: Four tiles arranged in a 'L'-shape; '⠧', `▄▄█`.
    L,
    /// 'J'-Tetromino: Four tiles arranged in a 'J'-shape; '⠼', `█▄▄`.
    J,
}

/// An active tetromino in play.
///
/// Notably, the [`Game`] additionally stores [`LockingData`] corresponding
/// to the main active piece outside this struct.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActivePiece {
    /// Type of tetromino the active piece is.
    pub shape: Tetromino,
    /// In which way the tetromino is re-oriented.
    pub orientation: Orientation,
    /// The position of the active piece on a playing grid.
    pub position: Coord,
}

/// Locking details stored about an active piece in play.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LockingData {
    /// Whether the main piece currently touches a surface below.
    pub touches_ground: bool,
    /// The last time the main piece was recorded to touching ground after not having done previously.
    pub last_touchdown: Option<GameTime>,
    /// The last time the main piece was recorded to be afloat after not having been previously.
    pub last_liftoff: Option<GameTime>,
    /// The total duration the main piece is allowed to touch ground until it should immediately lock down.
    pub ground_time_left: Duration,
    /// The lowest recorded vertical position of the main piece.
    pub lowest_y: usize,
}

/// Certain statistics for which an instance of [`Game`] can be checked against.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Stat {
    /// Whether a given amount of total time has elapsed in-game.
    TimeElapsed(GameTime),
    /// Whether a given number of [`Tetromino`]s have been locked/placed on the game's [`Board`].
    PiecesLocked(u32),
    /// Whether a given number of lines have been cleared from the [`Board`].
    LinesCleared(usize),
    /// Whether a certain level of gravity has been reached already.
    GravityReached(u32),
    /// Whether a given number of points has been scored already.
    PointsScored(u64),
}

/// Some values that were used to help initialize the game.
///
/// Used for game reproducibility.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InitialValues {
    /// The gravity at which a game should start.
    pub initial_gravity: u32,
    /// The method (and internal state) of tetromino generation used.
    pub start_generator: TetrominoGenerator,
    /// The value to seed the game's PRNG with.
    pub seed: u64,
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
#[derive(PartialEq, PartialOrd, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Configuration {
    /// How many pieces should be pre-generated and accessible/visible in the game state.
    pub piece_preview_count: usize,
    /// Whether holding a rotation button lets a piece be smoothly spawned in a rotated state.
    pub allow_prespawn_actions: bool,
    /// The method of tetromino rotation used.
    pub rotation_system: RotationSystem,
    /// How long it takes for the active piece to start automatically shifting more to the side
    /// after the initial time a 'move' button has been pressed.
    pub delayed_auto_shift: Duration,
    /// How long it takes for automatic side movement to repeat once it has started.
    pub auto_repeat_rate: Duration,
    /// How much faster than normal drop speed a piece should fall while 'soft drop' is being held.
    pub soft_drop_factor: f64,
    /// How long it takes a piece to attempt locking down after 'hard drop' has landed the piece on
    /// the ground.
    pub hard_drop_delay: Duration,
    /// How long each spawned active piece may touch the ground in total until it should lock down
    /// immediately.
    pub ground_time_max: Duration,
    /// How long the game should wait after clearing a line.
    pub line_clear_delay: Duration,
    /// How long the game should wait *additionally* before spawning a new piece.
    pub appearance_delay: Duration,
    /// Whether the gravity should be automatically incremented while the game plays.
    pub progressive_gravity: bool,
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

/// An event that is scheduled by the game engine to execute some action.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GameEvent {
    /// Event of a line being cleared from the board.
    LineClear,
    /// Event of a new [`ActivePiece`] coming into play.
    Spawn,
    /// Event of the current [`ActivePiece`] being fixed on the board, allowing no further updates
    /// to its state.
    Lock,
    /// Event of trying to hold / swap out the current piece.
    Hold,
    /// Event of the active piece being dropped down and a fast [`GameEvent::LockTimer`] being initiated.
    HardDrop,
    /// Event of the active piece being dropped down (without any further action or locking).
    SonicDrop,
    /// Event of the active piece immediately dropping down by one.
    SoftDrop,
    /// Event of the active piece moving down due to ordinary game gravity.
    Fall,
    /// Event of the active piece moving sideways.
    ///
    /// Stores whether it was the initial move input in that direction.
    Move(bool),
    /// Event of the active piece rotating.
    ///
    /// Stores some number of right turns.
    Rotate(i8),
    /// Event of attempted piece lock down.
    LockTimer,
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
    Limit,
    /// Generic game over by player forfeit.
    Forfeit,
}

/// Struct storing internal game state that changes over the course of play.
#[derive(Eq, PartialEq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct State {
    /// Current in-game time.
    pub time: GameTime,
    /// Upcoming game events.
    pub events: HashMap<GameEvent, GameTime>,
    /// The current state of buttons being pressed in the game.
    pub buttons_pressed: ButtonsArray<Option<GameTime>>,
    /// The main playing grid storing empty (`None`) and filled, fixed tiles (`Some(nz_u32)`).
    pub board: Board,
    /// All relevant data of the current piece in play.
    pub active_piece_data: Option<(ActivePiece, LockingData)>,
    /// Data about the piece being held. `true` denotes that the held piece can be swapped back in.
    pub hold_piece: Option<(Tetromino, bool)>,
    /// Upcoming pieces to be played.
    pub next_pieces: VecDeque<Tetromino>,
    /// The method (and internal state) of tetromino generation used.
    pub piece_generator: TetrominoGenerator,
    /// Tallies of how many pieces of each type have been played so far.
    ///
    /// Accessibe through `impl Index<Tetromino> for [T; 7]`.
    pub pieces_locked: [u32; Tetromino::VARIANTS.len()],
    /// The total number of lines that have been cleared.
    pub lines_cleared: usize,
    /// The current gravity/speed level the game is letting the pieces fall sat.
    pub gravity: u32,
    /// The current total score the player has achieved in this round of play.
    pub score: u64,
    /// The number of consecutive pieces that have been played and caused a line clear.
    pub consecutive_line_clears: u32,
    /// The internal pseudo random number generator used.
    pub rng: GameRng,
    /// Whether the game has ended and how.
    pub result: Option<GameResult>,
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
    pub mod_function: Box<GameModFn>,
}

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
    pub start_generator: Option<TetrominoGenerator>,
    /// The value to seed the game's PRNG with.
    pub seed: Option<u64>,
}

/// Main game struct representing one round of play.
#[derive(Debug)]
pub struct Game {
    config: Configuration,
    init_vals: InitialValues,
    state: State,
    modifiers: Vec<Modifier>,
}

/// An error that can be thrown by [`Game::update`].
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
pub enum UpdateGameError {
    /// Error variant caused by an attempt to update the game with a requested `update_time` that lies in
    /// the game's past (` < game.state().time`).
    DurationPassed,
    /// Error variant caused by an attempt to update a game that has ended (`game.ended() == true`).
    GameEnded,
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
    PieceLocked(ActivePiece),
    /// A number of lines were cleared.
    ///
    /// The duration indicates the line clear delay the game was configured with at the time.
    LineClears(Vec<usize>, Duration),
    /// A piece was quickly dropped from its original position to a new one.
    HardDrop(ActivePiece, ActivePiece),
    /// The player cleared some lines with a number of other stats that might have increased their
    /// score bonus.
    Accolade {
        /// The final computed score bonus caused by the action.
        score_bonus: u32,
        /// How many lines were cleared by the piece simultaneously
        lines_cleared: u32,
        /// The number of consecutive pieces played that caused a lineclear.
        combo: u32,
        /// Whether the piece was spun into place.
        is_spin: bool,
        /// Whether the entire board was cleared empty by this action.
        is_perfect_clear: bool,
        /// The tetromino type that was locked.
        tetromino: Tetromino,
    },
    /// A message containing an exact in-engine [`GameEvent`] that was processed.
    EngineEvent(GameEvent),
    /// A message containing an exact in-engine [`PressedButtons`] (user input) that was processed.
    EngineInput(PressedButtons, PressedButtons),
    /// Generic text feedback message.
    ///
    /// This is currently unused in the base engine.
    Text(String),
}

/// The points at which a [`GameModFn`] will be applied.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
pub enum UpdatePoint {
    /// Passed at the beginning of any call to [`Game::update`].
    UpdateStart,
    /// Passed when the modifier is called immediately before an [`GameEvent`] is handled.
    BeforeEvent(GameEvent),
    /// Passed when the modifier is called immediately after an [`GameEvent`] has been handled.
    AfterEvent(GameEvent),
    /// Passed when the modifier is called immediately before new user input is handled.
    BeforeInput,
    /// Passed when the modifier is called immediately after new user input has been handled.
    AfterInput,
}

impl Orientation {
    /// Find a new direction by turning right some number of times.
    ///
    /// This accepts `i32` to allow for left rotation.
    pub const fn reorient_right(&self, right_turns: i8) -> Self {
        use Orientation::*;
        let base = match self {
            N => 0,
            E => 1,
            S => 2,
            W => 3,
        };
        match (base + right_turns).rem_euclid(4) {
            0 => N,
            1 => E,
            2 => S,
            3 => W,
            _ => unreachable!(),
        }
    }
}

impl Tetromino {
    /// The Tetromino variants.
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
                N | S => [(0, 0), (1, 0), (1, 1), (2, 1)], // ⠴⠂
                E | W => [(1, 0), (0, 1), (1, 1), (0, 2)], // ⠳
            },
            Tetromino::Z => match oriented {
                N | S => [(1, 0), (2, 0), (0, 1), (1, 1)], // ⠲⠄
                E | W => [(0, 0), (0, 1), (1, 1), (1, 2)], // ⠞
            },
            Tetromino::T => match oriented {
                N => [(0, 0), (1, 0), (2, 0), (1, 1)], // ⠴⠄
                E => [(0, 0), (0, 1), (1, 1), (0, 2)], // ⠗
                S => [(1, 0), (0, 1), (1, 1), (2, 1)], // ⠲⠂
                W => [(1, 0), (0, 1), (1, 1), (1, 2)], // ⠺
            },
            Tetromino::L => match oriented {
                N => [(0, 0), (1, 0), (2, 0), (2, 1)], // ⠤⠆
                E => [(0, 0), (1, 0), (0, 1), (0, 2)], // ⠧
                S => [(0, 0), (0, 1), (1, 1), (2, 1)], // ⠖⠂
                W => [(1, 0), (1, 1), (0, 2), (1, 2)], // ⠹
            },
            Tetromino::J => match oriented {
                N => [(0, 0), (1, 0), (2, 0), (0, 1)], // ⠦⠄
                E => [(0, 0), (0, 1), (0, 2), (1, 2)], // ⠏
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

impl<T> ops::Index<Tetromino> for [T; 7] {
    type Output = T;

    fn index(&self, idx: Tetromino) -> &Self::Output {
        match idx {
            Tetromino::O => &self[0],
            Tetromino::I => &self[1],
            Tetromino::S => &self[2],
            Tetromino::Z => &self[3],
            Tetromino::T => &self[4],
            Tetromino::L => &self[5],
            Tetromino::J => &self[6],
        }
    }
}

impl<T> ops::IndexMut<Tetromino> for [T; 7] {
    fn index_mut(&mut self, idx: Tetromino) -> &mut Self::Output {
        match idx {
            Tetromino::O => &mut self[0],
            Tetromino::I => &mut self[1],
            Tetromino::S => &mut self[2],
            Tetromino::Z => &mut self[3],
            Tetromino::T => &mut self[4],
            Tetromino::L => &mut self[5],
            Tetromino::J => &mut self[6],
        }
    }
}

impl ActivePiece {
    /// Returns the coordinates and tile types for he piece on the board.
    pub fn tiles(&self) -> [(Coord, TileTypeID); 4] {
        let Self {
            shape,
            orientation,
            position: (x, y),
        } = self;
        let tile_type_id = shape.tiletypeid();
        shape
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
    pub fn fits_at(&self, board: &Board, offset: Offset) -> Option<ActivePiece> {
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
    ) -> Option<ActivePiece> {
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
    ) -> Option<ActivePiece> {
        let mut new_piece = *self;
        new_piece.orientation = new_piece.orientation.reorient_right(right_turns);
        let old_pos = self.position;
        offsets.into_iter().find_map(|offset| {
            new_piece.position = add(old_pos, offset)?;
            new_piece.fits(board).then_some(new_piece)
        })
    }

    /// Returns the lowest position the piece can reached until it touches ground if dropped
    /// straight down.
    pub fn well_piece(&self, board: &Board) -> ActivePiece {
        let mut well_piece = *self;
        // Move piece all the way down.
        while let Some(piece_below) = well_piece.fits_at(board, (0, -1)) {
            well_piece = piece_below;
        }
        well_piece
    }
}

impl Button {
    /// All button variants.
    pub const VARIANTS: [Self; 9] = [
        Self::MoveLeft,
        Self::MoveRight,
        Self::RotateLeft,
        Self::RotateRight,
        Self::RotateAround,
        Self::DropSoft,
        Self::DropHard,
        Self::DropSonic,
        Self::HoldPiece,
    ];
}

impl<T> ops::Index<Button> for [T; 9] {
    type Output = T;

    fn index(&self, idx: Button) -> &Self::Output {
        match idx {
            Button::MoveLeft => &self[0],
            Button::MoveRight => &self[1],
            Button::RotateLeft => &self[2],
            Button::RotateRight => &self[3],
            Button::RotateAround => &self[4],
            Button::DropSoft => &self[5],
            Button::DropHard => &self[6],
            Button::DropSonic => &self[7],
            Button::HoldPiece => &self[8],
        }
    }
}

impl<T> ops::IndexMut<Button> for [T; 9] {
    fn index_mut(&mut self, idx: Button) -> &mut Self::Output {
        match idx {
            Button::MoveLeft => &mut self[0],
            Button::MoveRight => &mut self[1],
            Button::RotateLeft => &mut self[2],
            Button::RotateRight => &mut self[3],
            Button::RotateAround => &mut self[4],
            Button::DropSoft => &mut self[5],
            Button::DropHard => &mut self[6],
            Button::DropSonic => &mut self[7],
            Button::HoldPiece => &mut self[8],
        }
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            piece_preview_count: 4,
            allow_prespawn_actions: true,
            rotation_system: RotationSystem::Ocular,
            delayed_auto_shift: Duration::from_millis(167),
            auto_repeat_rate: Duration::from_millis(33),
            soft_drop_factor: 10.0,
            hard_drop_delay: Duration::from_micros(100),
            ground_time_max: Duration::from_millis(2000),
            line_clear_delay: Duration::from_millis(200),
            appearance_delay: Duration::from_millis(50),
            progressive_gravity: true,
            end_conditions: EndConditions::default(),
            feedback_verbosity: FeedbackVerbosity::default(),
        }
    }
}

impl fmt::Debug for Modifier {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("GameModifier")
            .field("identifier", &self.descriptor)
            .field(
                "mod_function",
                &std::any::type_name_of_val(&self.mod_function),
            )
            .finish()
    }
}

impl GameBuilder {
    /// Creates a blank new template representing a yet-to-be-started [`Game`] ready for configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a [`Game`] with the information specified by `self`.
    pub fn build(&self) -> Game {
        self.build_modified([])
    }

    /// Creates a [`Game`] with the information specified by `self` and some one-time `modifiers`.
    pub fn build_modified(&self, modifiers: impl IntoIterator<Item = Modifier>) -> Game {
        let init_vals = InitialValues {
            initial_gravity: self.initial_gravity.unwrap_or(1),
            start_generator: self.start_generator.clone().unwrap_or_default(),
            seed: self.seed.unwrap_or_else(|| rand::rng().next_u64()),
        };
        Game {
            config: self.config.clone(),
            state: State {
                time: Duration::ZERO,
                events: HashMap::from([(GameEvent::Spawn, Duration::ZERO)]),
                buttons_pressed: ButtonsArray::default(),
                board: Board::default(),
                active_piece_data: None,
                hold_piece: None,
                next_pieces: VecDeque::default(),
                piece_generator: init_vals.start_generator.clone(),
                pieces_locked: [0; 7],
                lines_cleared: 0,
                gravity: init_vals.initial_gravity,
                score: 0,
                consecutive_line_clears: 0,
                rng: GameRng::seed_from_u64(init_vals.seed),
                result: None,
            },
            init_vals,
            modifiers: modifiers.into_iter().collect(),
        }
    }

    /// Sets the [`InitialValues`] that will be used by [`Game`].
    pub fn init_vals(self, x: InitialValues) -> Self {
        self.seed(x.seed)
            .initial_gravity(x.initial_gravity)
            .start_generator(x.start_generator)
    }

    /// The value to seed the game's PRNG with.
    pub fn seed(mut self, x: u64) -> Self {
        self.seed = Some(x);
        self
    }

    /// The gravity at which a game should start.
    pub fn initial_gravity(mut self, x: u32) -> Self {
        self.initial_gravity = Some(x);
        self
    }

    /// The method (and internal state) of tetromino generation used.
    pub fn start_generator(mut self, x: TetrominoGenerator) -> Self {
        self.start_generator = Some(x);
        self
    }

    /// Sets the [`Configuration`] that will be used by [`Game`].
    pub fn config(mut self, x: Configuration) -> Self {
        self.config = x;
        self
    }

    /// How many pieces should be pre-generated and accessible/visible in the game state.
    pub fn piece_preview_count(mut self, x: usize) -> Self {
        self.config.piece_preview_count = x;
        self
    }
    /// Whether holding a rotation button lets a piece be smoothly spawned in a rotated state.
    pub fn allow_prespawn_actions(mut self, x: bool) -> Self {
        self.config.allow_prespawn_actions = x;
        self
    }
    /// The method of tetromino rotation used.
    pub fn rotation_system(mut self, x: RotationSystem) -> Self {
        self.config.rotation_system = x;
        self
    }
    /// How long it takes for the active piece to start automatically shifting more to the side
    /// after the initial time a 'move' button has been pressed.
    pub fn delayed_auto_shift(mut self, x: Duration) -> Self {
        self.config.delayed_auto_shift = x;
        self
    }
    /// How long it takes for automatic side movement to repeat once it has started.
    pub fn auto_repeat_rate(mut self, x: Duration) -> Self {
        self.config.auto_repeat_rate = x;
        self
    }
    /// How much faster than normal drop speed a piece should fall while 'soft drop' is being held.
    pub fn soft_drop_factor(mut self, x: f64) -> Self {
        self.config.soft_drop_factor = x;
        self
    }
    /// How long it takes a piece to attempt locking down after 'hard drop' has landed the piece on
    /// the ground.
    pub fn hard_drop_delay(mut self, x: Duration) -> Self {
        self.config.hard_drop_delay = x;
        self
    }
    /// How long each spawned active piece may touch the ground in total until it should lock down
    /// immediately.
    pub fn ground_time_max(mut self, x: Duration) -> Self {
        self.config.ground_time_max = x;
        self
    }
    /// How long the game should wait after clearing a line.
    pub fn line_clear_delay(mut self, x: Duration) -> Self {
        self.config.line_clear_delay = x;
        self
    }
    /// How long the game should wait *additionally* before spawning a new piece.
    pub fn appearance_delay(mut self, x: Duration) -> Self {
        self.config.appearance_delay = x;
        self
    }
    /// Whether the gravity should be automatically incremented while the game plays.
    pub fn progressive_gravity(mut self, x: bool) -> Self {
        self.config.progressive_gravity = x;
        self
    }
    /// Stores the ways in which a round of the game should be limited.
    ///
    /// Each limitation may be either of positive ('game completed') or negative ('game over'), as
    /// designated by the `bool` stored with it.
    ///
    /// No limitations may allow for endless games.
    pub fn end_conditions(mut self, x: Vec<(Stat, bool)>) -> Self {
        self.config.end_conditions = x;
        self
    }

    /// The amount of feedback information that is to be generated.
    pub fn feedback_verbosity(mut self, x: FeedbackVerbosity) -> Self {
        self.config.feedback_verbosity = x;
        self
    }
}

impl Game {
    /// The maximum height *any* piece tile could reach before [`GameOver::LockOut`] occurs.
    pub const HEIGHT: usize = Self::SKYLINE + 7;
    /// The game field width.
    pub const WIDTH: usize = 10;
    /// The maximal height of the (conventionally visible) playing grid that can be played in.
    pub const SKYLINE: usize = 20;
    /// This is the gravity level at which blocks instantly hit the floor ("20G").
    pub const INSTANT_GRAVITY: u32 = 20;

    /// Creates a blank new template representing a yet-to-be-started [`Game`] ready for configuration.
    pub fn builder() -> GameBuilder {
        GameBuilder::default()
    }

    /// Read accessor for the game's configuration.
    pub const fn config(&self) -> &Configuration {
        &self.config
    }

    /// Read accessor for the game's initial values.
    pub const fn init_vals(&self) -> &InitialValues {
        &self.init_vals
    }

    /// Read accessor for the game's list of modifiers.
    pub const fn modifiers(&self) -> &Vec<Modifier> {
        &self.modifiers
    }

    /// Read accessor for the current game state.
    pub const fn state(&self) -> &State {
        &self.state
    }

    /// Mutable accessor for the current game configurations.
    ///
    /// # Reproducibility
    /// Modifying a [`Game`]'s configuration after it was created might not make it easily
    /// reproducible anymore.
    pub const fn config_mut(&mut self) -> &mut Configuration {
        &mut self.config
    }

    /// Mutable accessor for the current game modifiers.
    pub const fn modifiers_mut(&mut self) -> &mut Vec<Modifier> {
        &mut self.modifiers
    }

    /// Check whether a certain stat value has been met or exceeded.
    pub fn check_stat_met(&self, stat: &Stat) -> bool {
        match stat {
            Stat::TimeElapsed(t) => *t <= self.state.time,
            Stat::PiecesLocked(p) => *p <= self.state.pieces_locked.iter().sum(),
            Stat::LinesCleared(l) => *l <= self.state.lines_cleared,
            Stat::GravityReached(g) => *g <= self.state.gravity,
            Stat::PointsScored(s) => *s <= self.state.score,
        }
    }

    /// Whether the game has ended, and whether it can continue to update.
    pub const fn ended(&self) -> bool {
        self.state.result.is_some()
    }

    /// Immediately end a game by forfeiting the current round.
    ///
    /// This can be used so `game.ended()` returns true and prevents future
    /// calls to `update` from continuing to advance the game.
    pub fn forfeit(&mut self) {
        self.state.result = Some(Err(GameOver::Forfeit))
    }

    /// Creates a blueprint [`GameBuilder`] and an iterator over current modifier identifiers ([`&str`]s) from which the exact game can potentially be rebuilt.
    pub fn blueprint(&self) -> (GameBuilder, impl Iterator<Item = &str>) {
        let builder = GameBuilder {
            config: self.config.clone(),
            initial_gravity: Some(self.init_vals.initial_gravity),
            start_generator: Some(self.init_vals.start_generator.clone()),
            seed: Some(self.init_vals.seed),
        };
        let mod_descriptors = self.modifiers.iter().map(|m| m.descriptor.as_str());
        (builder, mod_descriptors)
    }

    /// Tries to create an identical, independent copy of the current game.
    ///
    /// This function fails if the [`Game`] has any modifiers attached to it.
    pub fn try_clone(&self) -> Option<Self> {
        if self.modifiers.is_empty() {
            None
        } else {
            Some(Game {
                config: self.config.clone(),
                init_vals: self.init_vals.clone(),
                state: self.state.clone(),
                modifiers: Vec::new(),
            })
        }
    }
}

impl std::fmt::Display for UpdateGameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UpdateGameError::DurationPassed => {
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
