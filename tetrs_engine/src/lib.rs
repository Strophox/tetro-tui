/*!
# Tetrs Engine

`tetrs_engine` is an implementation of a tetromino game engine, able to handle numerous modern
mechanics.

# Examples

```
use tetrs_engine::*;

// Starting a game.
let mut game = Game::new(GameMode::marathon());

let mut button_state_1 = ButtonsPressed::default();
button_state_1[Button::MoveLeft] = true;

let update_time_1 = std::time::Duration::from_secs(3);

// Updating the game with 'left' pressed at second 3.
game.update(Some(button_state_1), update_time_1);

// ...

let update_time_2 = std::time::Duration::from_secs(4);

// Updating the game with *no* change in (left pressed) button state (since second 3).
game.update(None, update_time_2);

// View game state
let GameState { board, .. } = game.state();
// (Render the board, etc..)
```

TASK: Document all features (including IRS, etc. - cargo feature `serde`).
*/

#![warn(missing_docs)]

pub mod piece_generation;
pub mod piece_rotation;

use std::{
    collections::{HashMap, VecDeque},
    fmt,
    num::{NonZeroU32, NonZeroU8},
    ops,
    time::Duration,
};

pub use piece_generation::TetrominoSource;
pub use piece_rotation::RotationSystem;
use rand::rngs::ThreadRng;

/// Abstract identifier for which type of tile occupies a cell in the grid.
pub type TileTypeID = NonZeroU8;
/// The type of horizontal lines of the playing grid.
pub type Line = [Option<TileTypeID>; Game::WIDTH];
// NOTE: Would've liked to use `impl Game { type Board = ...` (https://github.com/rust-lang/rust/issues/8995)
/// The type of the entire two-dimensional playing grid.
pub type Board = Vec<Line>;
/// Coordinates conventionally used to index into the [`Board`], starting in the bottom left.
pub type Coord = (usize, usize);
/// Coordinates offsets that can be [`add`]ed to [`Coord`]inates.
pub type Offset = (isize, isize);
/// The type used to identify points in time in a game's internal timeline.
pub type GameTime = Duration;
/// A mapping for buttons, usable through `impl Index<Button>`.
type ButtonMap<T> = [T; 9];
/// A mapping for which buttons were pressed.
pub type ButtonsPressed = ButtonMap<bool>;
/// Convenient type alias to denote a collection of [`Feedback`]s associated with some [`GameTime`].
pub type FeedbackEvents = Vec<(GameTime, Feedback)>;
/// Type of functions that can be used to modify a game, c.f. [`Game::add_modifier`].
pub type FnGameMod = Box<
    dyn FnMut(&mut GameConfig, &mut GameMode, &mut GameState, &mut FeedbackEvents, &ModifierPoint),
>;
type EventMap = HashMap<GameEvent, GameTime>;

/// Represents an abstract game input.
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

/// Stores the ways in which a round of the game should be limited.
///
/// Each limitation may be either of positive ('game completed') or negative ('game over'), as
/// designated by the `bool` stored with it.
///
/// No limitations may allow for endless games.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Limits {
    /// The total time a round may be played.
    pub time: Option<(bool, Duration)>,
    /// The total number of pieces locked that may be played.
    pub pieces: Option<(bool, u32)>,
    /// The total number of full lines that may be cleared.
    pub lines: Option<(bool, usize)>,
    /// The gravity level to stop at.
    pub gravity: Option<(bool, u32)>,
    /// The number of game points to earn.
    pub score: Option<(bool, u64)>,
}

/// The playing configuration specific to the single, current round of play.
///
/// A 'game mode' usually mainly designates what kind of game is currently played,
/// and how it may end (un)successfully with regards to some goal.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameMode {
    /// Conventional name that may be given to an instance of this struct.
    pub name: String,
    /// The gravity at which a game should start.
    pub initial_gravity: u32,
    /// Whether the gravity should be automatically incremented while the game plays.
    pub increase_gravity: bool,
    /// The limitations under which a game may end (un)successfully.
    pub limits: Limits,
}

/// User-focused configuration options that mainly influence time-sensitive or cosmetic mechanics.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameConfig {
    /// The method of tetromino rotation used.
    pub rotation_system: RotationSystem,
    /// The method (and internal state) of tetromino generation used.
    pub tetromino_generator: TetrominoSource,
    /// How many pieces should be pre-generated and accessible/visible in the game state.
    pub preview_count: usize,
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
    /// Whether to disable a 'soft drop' button press to explicitly and immediately lock down a piece.
    pub no_soft_drop_lock: bool,
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
    Rotate(i32),
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
    ModeLimit,
    /// Generic game over by player forfeit.
    Forfeit,
}

// FIXME: Document all Invariants.
// * Until the game has finished there will always be more events: `finished.is_some() || !next_events.is_empty()`.
// * Unhandled events lie in the future: `for (event,event_time) in self.events { assert(self.time_updated < event_time); }`.
/// Struct storing internal game state that changes over the course of play.
#[derive(Eq, PartialEq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameState {
    /// Current in-game time.
    pub time: GameTime,
    /// Whether the game has ended and how.
    pub end: Option<Result<(), GameOver>>,
    /// Upcoming game events.
    pub events: EventMap,
    /// The current state of buttons being pressed in the game.
    pub buttons_pressed: ButtonMap<Option<GameTime>>,
    /// The main playing grid storing empty (`None`) and filled, fixed tiles (`Some(nz_u32)`).
    pub board: Board,
    /// All relevant data of the current piece in play.
    pub active_piece_data: Option<(ActivePiece, LockingData)>,
    /// Data about the piece being held. `true` denotes that the held piece can be swapped back in.
    pub hold_piece: Option<(Tetromino, bool)>,
    /// Upcoming pieces to be played.
    pub next_pieces: VecDeque<Tetromino>,
    /// Tallies of how many pieces of each type have been played so far.
    ///
    /// Accessibe through `impl Index<Tetromino> for [T; 7]`.
    pub pieces_played: [u32; 7],
    /// The total number of lines that have been cleared.
    pub lines_cleared: usize,
    /// The current gravity/speed level the game is letting the pieces fall sat.
    pub gravity: u32,
    /// The current total score the player has achieved in this round of play.
    pub score: u64,
    /// The number of consecutive pieces that have been played and caused a line clear.
    pub consecutive_line_clears: u32,
    /// The number of line clears that were either a quadruple, spin or perfect clear.
    pub back_to_back_special_clears: u32,
}

/// An error that can be thrown by [`Game::update`].
pub enum GameUpdateError {
    /// Error variant caused by an attempt to update the game with a requested `update_time` that lies in
    /// the game's past (` < game.state().time`).
    DurationPassed,
    /// Error variant caused by an attempt to update a game that has ended (`game.ended() == true`).
    GameEnded,
}

/// Main game struct representing one round of play.
pub struct Game {
    config: GameConfig,
    mode: GameMode,
    state: GameState,
    rng: ThreadRng,
    modifiers: Vec<FnGameMod>,
}

/// A number of feedback events that can be returned by the game.
///
/// These can be used to more easily render visual feedback to the player.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Feedback {
    /// A new piece was spawned.
    PieceSpawned(ActivePiece),
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
        /// The shape that was locked.
        shape: Tetromino,
        /// Whether the piece was spun into place.
        spin: bool,
        /// How many lines were cleared by the piece simultaneously
        lineclears: u32,
        /// Whether the entire board was cleared empty by this action.
        perfect_clear: bool,
        /// The number of consecutive pieces played that caused a lineclear.
        combo: u32,
        /// The number of consecutive lineclears where a spin, quadruple or perfect clear occurred.
        back_to_back: u32,
    },
    /// Generic text feedback message.
    ///
    /// This is currently unused in base game modes.
    Message(String),
}

/// The points at which a [`FnGameMod`] will be applied.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
pub enum ModifierPoint {
    /// Passed at the beginning of any call to [`Game::update`].
    UpdateStart,
    /// Passed when the modifier is called immediately before an [`GameEvent`] is handled.
    BeforeEvent(GameEvent),
    /// Passed when the modifier is called immediately after an [`GameEvent`] has been handled.
    AfterEvent(GameEvent),
    /// Passed when the modifier is called immediately before new user input is handled.
    BeforeButtonChange,
    /// Passed when the modifier is called immediately after new user input has been handled.
    AfterButtonChange,
}

impl Orientation {
    /// Find a new direction by turning right some number of times.
    ///
    /// This accepts `i32` to allow for left rotation.
    pub fn rotate_right(&self, right_turns: i32) -> Self {
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
    pub const SHAPES: [Self; 7] = {
        use Tetromino::*;
        [O, I, S, Z, T, L, J]
    };

    /// Returns the mino offsets of a tetromino shape, given an orientation.
    pub fn minos(&self, oriented: Orientation) -> [Coord; 4] {
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
    pub fn fits_at_rotated(
        &self,
        board: &Board,
        offset: Offset,
        right_turns: i32,
    ) -> Option<ActivePiece> {
        let mut new_piece = *self;
        new_piece.orientation = new_piece.orientation.rotate_right(right_turns);
        new_piece.position = add(self.position, offset)?;
        new_piece.fits(board).then_some(new_piece)
    }

    /// Given an iterator over some offsets, checks whether the rotated piece fits at any offset
    /// location onto the board.
    pub fn first_fit(
        &self,
        board: &Board,
        offsets: impl IntoIterator<Item = Offset>,
        right_turns: i32,
    ) -> Option<ActivePiece> {
        let mut new_piece = *self;
        new_piece.orientation = new_piece.orientation.rotate_right(right_turns);
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

impl GameMode {
    /// Produce a game mode template for "Marathon" mode.
    ///
    /// Settings:
    /// - Name: "Marathon".
    /// - Start level: 1.
    /// - Level increment: Yes.
    /// - Limits: Level 16.
    pub fn marathon() -> Self {
        Self {
            name: String::from("Marathon"),
            initial_gravity: 1,
            increase_gravity: true,
            limits: Limits {
                gravity: Some((true, 15)),
                ..Default::default()
            },
        }
    }

    /// Produce a game mode template for "40-Lines" mode.
    ///
    /// Settings:
    /// - Name: "40-Lines".
    /// - Start level: (variable).
    /// - Level increment: No.
    /// - Limits: 40 line clears.
    pub fn sprint(initial_gravity: u32) -> Self {
        Self {
            name: String::from("40-Lines"),
            initial_gravity,
            increase_gravity: false,
            limits: Limits {
                lines: Some((true, 40)),
                ..Default::default()
            },
        }
    }

    /// Produce a game mode template for "Time Trial" mode.
    ///
    /// Settings:
    /// - Name: "Time Trial".
    /// - Start level: (variable).
    /// - Level increment: No.
    /// - Limits: 180 seconds.
    pub fn ultra(initial_gravity: u32) -> Self {
        Self {
            name: String::from("Time Trial"),
            initial_gravity,
            increase_gravity: false,
            limits: Limits {
                time: Some((true, Duration::from_secs(3 * 60))),
                ..Default::default()
            },
        }
    }

    /// Produce a game mode template for "Master" mode.
    ///
    /// Settings:
    /// - Name: "Master".
    /// - Start level: 19.
    /// - Level increment: Yes.
    /// - Limits: 100 Lines.
    pub fn master() -> Self {
        Self {
            name: String::from("Master"),
            initial_gravity: Game::INSTANT_GRAVITY,
            increase_gravity: true,
            limits: Limits {
                gravity: Some((true, 35)),
                ..Default::default()
            },
        }
    }

    /// Produce a game mode template for "Endless" mode.
    ///
    /// Settings:
    /// - Name: "Endless".
    /// - Start level: 1.
    /// - Level increment: No.
    /// - Limits: None.
    pub fn zen(gravity: u32) -> Self {
        Self {
            name: String::from("Endless"),
            initial_gravity: gravity,
            increase_gravity: false,
            limits: Default::default(),
        }
    }
}

impl Button {
    // FIXME: Needs to be MANUALLY updated in sync with `Button`...
    const VARIANTS: [Self; 9] = [
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

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            rotation_system: RotationSystem::Ocular,
            tetromino_generator: TetrominoSource::recency(),
            preview_count: 4,
            delayed_auto_shift: Duration::from_millis(167),
            auto_repeat_rate: Duration::from_millis(33),
            soft_drop_factor: 15.0,
            hard_drop_delay: Duration::from_micros(100),
            ground_time_max: Duration::from_millis(2000),
            line_clear_delay: Duration::from_millis(200),
            appearance_delay: Duration::from_millis(50),
            no_soft_drop_lock: false,
        }
    }
}

impl fmt::Debug for Game {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Game")
            .field("config", &self.config)
            .field("state", &self.state)
            .field("rng", &std::any::type_name_of_val(&self.rng))
            .field("modifiers", &std::any::type_name_of_val(&self.modifiers))
            .finish()
    }
}

impl Game {
    /// The maximum height *any* piece tile could reach before [`GameOver::LockOut`] occurs.
    pub const HEIGHT: usize = Self::SKYLINE + 7;
    /// The game field width.
    pub const WIDTH: usize = 10;
    /// The maximal height of the (conventionally visible) playing grid that can be played in.
    pub const SKYLINE: usize = 20;
    // This is the level at which blocks start falling with 20G / instantly hit the floor.
    const INSTANT_GRAVITY: u32 = 20;

    /// Start a new game given some game mode.
    pub fn new(game_mode: GameMode) -> Self {
        Self::with_config(game_mode, GameConfig::default())
    }

    /// Start a new game given a gamemode and some advanced configuration options.
    pub fn with_config(game_mode: GameMode, config: GameConfig) -> Self {
        let state = GameState {
            time: Duration::ZERO,
            end: None,
            events: HashMap::from([(GameEvent::Spawn, Duration::ZERO)]),
            buttons_pressed: Default::default(),
            board: std::iter::repeat(Line::default())
                .take(Self::HEIGHT)
                .collect(),
            active_piece_data: None,
            hold_piece: None,
            next_pieces: VecDeque::new(),
            pieces_played: [0; 7],
            lines_cleared: 0,
            gravity: game_mode.initial_gravity,
            score: 0,
            consecutive_line_clears: 0,
            back_to_back_special_clears: 0,
        };
        Game {
            config,
            mode: game_mode,
            state,
            rng: rand::thread_rng(),
            modifiers: Vec::new(),
        }
    }

    /// Immediately end a game by forfeiting the current round.
    ///
    /// This can be used so `game.ended()` returns true and prevents future
    /// calls to `update` from continuing to advance the game.
    pub fn forfeit(&mut self) {
        self.state.end = Some(Err(GameOver::Forfeit))
    }

    /// Whether the game has ended, or whether it can continue to update.
    pub fn ended(&self) -> bool {
        self.state.end.is_some()
    }

    /// Immutable accessor for the current game configurations.
    pub fn config(&self) -> &GameConfig {
        &self.config
    }

    /// Mutable accessor for the current game configurations.
    pub fn config_mut(&mut self) -> &mut GameConfig {
        &mut self.config
    }

    /// Immutable accessor for the current game mode.
    pub fn mode(&self) -> &GameMode {
        &self.mode
    }

    /// Mutable accessor for the current game mode.
    ///
    // FIXME: Document Safety.
    /// This directly allows raw, mutable access to the game's [`GameMode`] field.
    /// This should not cause undefined behaviour per se, but may lead to spurious `panic!`s or
    /// other unexpected gameplay behaviour due to violating internal invarints.
    pub fn mode_mut(&mut self) -> &mut GameMode {
        &mut self.mode
    }

    /// Immutable accessor for the current game state.
    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// Mutable accessor for the current game mode.
    ///
    // FIXME: Document Safety.
    /// This directly allows raw, mutable access to the game's [`GameState`] field.
    /// This should not cause undefined behaviour per se, but may lead to spurious `panic!`s or
    /// other unexpected gameplay behaviour due to violating internal invarints.
    pub fn state_mut(&mut self) -> &mut GameState {
        &mut self.state
    }

    /// Adds a 'game mod' that will get executed regularly before and after each [`GameEvent`].
    ///
    /// This indirectly allows raw, mutable access to the game's [`GameMode`]
    /// and [`GameState`] fields.
    /// This should not cause undefined behaviour per se, but may lead to spurious `panic!`s or
    /// other unexpected gameplay behaviour due to violating internal invarints.
    pub fn add_modifier(&mut self, game_mod: FnGameMod) {
        self.modifiers.push(game_mod)
    }

    /// Updates the internal `self.state.end` state, checking whether any [`Limits`] have been reached.
    fn update_game_end(&mut self) {
        self.state.end = self.state.end.or_else(|| {
            [
                self.mode
                    .limits
                    .time
                    .and_then(|(win, dur)| (dur <= self.state.time).then_some(win)),
                self.mode.limits.pieces.and_then(|(win, pcs)| {
                    (pcs <= self.state.pieces_played.iter().sum()).then_some(win)
                }),
                self.mode
                    .limits
                    .lines
                    .and_then(|(win, lns)| (lns <= self.state.lines_cleared).then_some(win)),
                self.mode
                    .limits
                    .gravity
                    .and_then(|(win, lvl)| (lvl <= self.state.gravity).then_some(win)),
                self.mode
                    .limits
                    .score
                    .and_then(|(win, pts)| (pts <= self.state.score).then_some(win)),
            ]
            .into_iter()
            .find_map(|limit_reached| {
                limit_reached.map(|win| {
                    if win {
                        Ok(())
                    } else {
                        Err(GameOver::ModeLimit)
                    }
                })
            })
        });
    }

    /// Goes through all internal 'game mods' and applies them sequentially at the given [`ModifierPoint`].
    fn apply_modifiers(
        &mut self,
        feedback_events: &mut Vec<(GameTime, Feedback)>,
        modifier_point: &ModifierPoint,
    ) {
        for modify in &mut self.modifiers {
            modify(
                &mut self.config,
                &mut self.mode,
                &mut self.state,
                feedback_events,
                modifier_point,
            );
        }
    }

    /// The main function used to advance the game state.
    ///
    /// This will cause an internal update of all [`GameEvent`]s up to and including the given
    /// `update_time` requested.
    /// If `new_button_state.is_some()` then the same thing happens, except that the very last
    /// 'event' will be the change of [`ButtonsPressed`] at `update_time` (which might cause some
    /// further events that are handled at `update_time` before finally returning).
    ///
    /// Unless an error occurs, this function will return all [`FeedbackEvents`] caused between the
    /// previous and the current `update` call, in chronological order.
    ///
    /// # Errors
    ///
    /// This function may error with:
    /// - [`GameUpdateError::GameEnded`] if `game.ended()` is `true`, indicating that no more updates
    ///   can change the game state, or
    /// - [`GameUpdateError::DurationPassed`] if `update_time < game.state().time`, indicating that
    ///   the requested update lies in the past.
    pub fn update(
        &mut self,
        mut new_button_state: Option<ButtonsPressed>,
        update_time: GameTime,
    ) -> Result<FeedbackEvents, GameUpdateError> {
        /*
        Order:
        - if game already ended, return immediately
        * find next event
        - event less-or-equal update point:
            - allow modifiers
            - handle event
            - allow modifiers
            - update game end state, possibly return immediately
            - goto *
        - update point reached:
            - try adding input events, goto *
            - else return immediately
         */
        // Invalid call: return immediately.
        if update_time < self.state.time {
            return Err(GameUpdateError::DurationPassed);
        }
        if self.ended() {
            return Err(GameUpdateError::GameEnded);
        };
        // NOTE: Returning an empty Vec is efficient because it won't even allocate (as by Rust API).
        let mut feedback_events = Vec::new();
        self.apply_modifiers(&mut feedback_events, &ModifierPoint::UpdateStart);
        // We linearly process all events until we reach the update time.
        'event_simulation: loop {
            // Peek the next closest event.
            // SAFETY: `Game` invariants guarantee there's some event.
            let next_event = self
                .state
                .events
                .iter()
                .min_by_key(|(&event, &event_time)| (event_time, event));
            match next_event {
                // Next event within requested update time, handle event first.
                Some((&event, &event_time)) if event_time <= update_time => {
                    self.apply_modifiers(&mut feedback_events, &ModifierPoint::BeforeEvent(event));
                    // Remove next event and handle it.
                    self.state.events.remove_entry(&event);
                    let new_feedback_events = self.handle_event(event, event_time);
                    self.state.time = event_time;
                    feedback_events.extend(new_feedback_events);
                    self.apply_modifiers(&mut feedback_events, &ModifierPoint::AfterEvent(event));
                    // Stop simulation early if event or modifier ended game.
                    self.update_game_end();
                    if self.ended() {
                        break 'event_simulation;
                    }
                }
                _ => {
                    // Possibly process user input events now or break out.
                    // NOTE: We should be able to update the time here because `self.process_input(...)` does not access it.
                    self.state.time = update_time;
                    // Update button inputs.
                    if let Some(buttons_pressed) = new_button_state.take() {
                        self.apply_modifiers(
                            &mut feedback_events,
                            &ModifierPoint::BeforeButtonChange,
                        );
                        self.input_update(buttons_pressed, update_time);
                        self.apply_modifiers(
                            &mut feedback_events,
                            &ModifierPoint::AfterButtonChange,
                        );
                    } else {
                        self.update_game_end();
                        break 'event_simulation;
                    }
                }
            };
        }
        Ok(feedback_events)
    }

    /// Computes and adds to the internal event queue any relevant [`GameEvent`]s caused by the
    /// player in form of a change of button states.
    #[allow(clippy::bool_comparison, clippy::comparison_chain)]
    fn input_update(&mut self, next_buttons_pressed: ButtonsPressed, update_time: GameTime) {
        let prev_buttons_pressed = self.state.buttons_pressed.map(|x| x.is_some());
        if self.state.active_piece_data.is_some() {
            let [ml0, mr0, rl0, rr0, ra0, ds0, dh0, dc0, h0] = prev_buttons_pressed;
            let [ml1, mr1, rl1, rr1, ra1, ds1, dh1, dc1, h1] = next_buttons_pressed;

            // Single new button has been pressed, remove repeat moves and add initial move.
            /*
            Karnaugh map:
            |           !mR0 !mR0  mR0  mR0
            |           !mR1  mR1  mR1 !mR1
            | !mL0 !mL1   -    r    -    0
            | !mL0  mL1   l    -    l    l
            |  mL0  mL1   -    r    -    l?
            |  mL0 !mL1   0    r    r?   0
            */
            let one_m_pressed = (ml0 < ml1) != (mr0 < mr1);
            let revert_m_left = (ml0 && ml1)
                && (mr0 > mr1)
                && self.state.buttons_pressed[Button::MoveLeft]
                    < self.state.buttons_pressed[Button::MoveRight];
            let revert_m_right = (mr0 && mr1)
                && (ml0 > ml1)
                && self.state.buttons_pressed[Button::MoveRight]
                    < self.state.buttons_pressed[Button::MoveLeft];
            let any_m_unpressed = (ml0 || mr0) && (!ml1 && !mr1);
            if one_m_pressed || revert_m_left || revert_m_right {
                self.state.events.remove(&GameEvent::Move(false));
                self.state.events.insert(GameEvent::Move(true), update_time);
            // Both move buttons unpressed, remove repeat moves.
            } else if any_m_unpressed {
                self.state.events.remove(&GameEvent::Move(false));
            }
            // Count number of turns using newly pressed rotation buttons.
            let mut turns = 0;
            if rl0 < rl1 {
                turns -= 1;
            }
            if rr0 < rr1 {
                turns += 1;
            }
            if ra0 < ra1 {
                turns += 2;
            }
            if turns != 0 {
                self.state
                    .events
                    .insert(GameEvent::Rotate(turns), update_time);
            }
            // Soft drop button pressed, add event.
            if ds0 < ds1 {
                self.state.events.insert(GameEvent::SoftDrop, update_time);
            // Soft drop button released: Reset fall timer.
            } else if ds0 > ds1 {
                if let Ok(level) = NonZeroU32::try_from(self.state.gravity) {
                    self.state
                        .events
                        .insert(GameEvent::Fall, update_time + Self::drop_delay(level, None));
                }
            }
            // Hard drop button pressed.
            if dh0 < dh1 {
                self.state.events.insert(GameEvent::HardDrop, update_time);
            }
            // Sonic drop button pressed
            if dc0 < dc1 {
                self.state.events.insert(GameEvent::SonicDrop, update_time);
            }
            // Hold button pressed
            if h0 < h1 {
                self.state.events.insert(GameEvent::Hold, update_time);
            }
        }
        // Update internal button state.
        #[allow(clippy::bool_comparison)]
        for ((button, prev), next) in Button::VARIANTS
            .iter()
            .zip(prev_buttons_pressed)
            .zip(next_buttons_pressed)
        {
            if prev < next {
                self.state.buttons_pressed[*button] = Some(update_time);
            } else if prev > next {
                self.state.buttons_pressed[*button] = None;
            }
        }
    }

    /// Given a tetromino variant to be spawned onto the board, returns the correct initial state of
    /// [`ActivePiece`].
    fn spawn_piece(shape: Tetromino) -> ActivePiece {
        let pos = match shape {
            Tetromino::O => (4, 20),
            _ => (3, 20),
        };
        let orientation = Orientation::N;
        /* NOTE: Unused spawn positions/orientations. While nice and symmetrical :): also unusual.
        let (orientation, pos) = match shape {
            Tetromino::O => (Orientation::N, (4, 20)),
            Tetromino::I => (Orientation::N, (3, 20)),
            Tetromino::S => (Orientation::E, (4, 20)),
            Tetromino::Z => (Orientation::W, (4, 20)),
            Tetromino::T => (Orientation::N, (3, 20)),
            Tetromino::L => (Orientation::E, (4, 20)),
            Tetromino::J => (Orientation::W, (4, 20)),
        };*/
        ActivePiece {
            shape,
            orientation,
            position: pos,
        }
    }

    /// Try holding a tetromino in the game state and report success.
    fn attempt_hold(&mut self, tetromino: Tetromino, event_time: GameTime) -> bool {
        match self.state.hold_piece {
            None | Some((_, true)) => {
                if let Some((held_piece, _)) = self.state.hold_piece {
                    self.state.next_pieces.push_front(held_piece);
                } else {
                    self.state.next_pieces.extend(
                        self.config
                            .tetromino_generator
                            .with_rng(&mut self.rng)
                            .take(1),
                    );
                }
                self.state.hold_piece = Some((tetromino, false));
                self.state.events.clear();
                self.state.events.insert(GameEvent::Spawn, event_time);
                true
            }
            _ => false,
        }
    }

    /// Given an event, update the internal game state, possibly adding new future events.
    ///
    /// This function is likely the most important part of a game update as it handles the logic of
    /// spawning, dropping, moving, locking the active piece, etc.
    /// It also returns some feedback events caused by clearing lines, locking the piece, etc.
    fn handle_event(&mut self, event: GameEvent, event_time: GameTime) -> FeedbackEvents {
        // Active piece touches the ground before update (or doesn't exist, counts as not touching).
        let mut feedback_events = Vec::new();
        let prev_piece_data = self.state.active_piece_data;
        let prev_piece = prev_piece_data.unzip().0;
        let next_piece = match event {
            // We generate a new piece above the skyline, and immediately queue a fall event for it.
            GameEvent::Spawn => {
                debug_assert!(
                    prev_piece.is_none(),
                    "spawning event but an active piece is still in play"
                );
                let tetromino = self.state.next_pieces.pop_front().unwrap_or_else(|| {
                    self.config
                        .tetromino_generator
                        .with_rng(&mut self.rng)
                        .next()
                        .expect("piece generator ran out before game finished")
                });
                self.state.next_pieces.extend(
                    self.config
                        .tetromino_generator
                        .with_rng(&mut self.rng)
                        .take(
                            self.config
                                .preview_count
                                .saturating_sub(self.state.next_pieces.len()),
                        ),
                );
                // Initial Hold System.
                if self.state.buttons_pressed[Button::HoldPiece].is_some()
                    && self.attempt_hold(tetromino, event_time)
                {
                    None
                } else {
                    let raw_piece = Self::spawn_piece(tetromino);
                    let mut turns = 0;
                    if self.state.buttons_pressed[Button::RotateRight].is_some() {
                        turns += 1;
                    }
                    if self.state.buttons_pressed[Button::RotateAround].is_some() {
                        turns += 2;
                    }
                    if self.state.buttons_pressed[Button::RotateLeft].is_some() {
                        turns -= 1;
                    }
                    // Initial Rotation system.
                    let next_piece = self
                        .config
                        .rotation_system
                        .rotate(&raw_piece, &self.state.board, turns)
                        .unwrap_or(raw_piece);
                    feedback_events.push((event_time, Feedback::PieceSpawned(next_piece)));
                    // Newly spawned piece conflicts with board - Game over.
                    if !next_piece.fits(&self.state.board) {
                        self.state.end = Some(Err(GameOver::BlockOut));
                        return feedback_events;
                    }
                    self.state.events.insert(GameEvent::Fall, event_time);
                    Some(next_piece)
                }
            }
            GameEvent::Hold => {
                let prev_piece = prev_piece.expect("hold piece event but no active piece");
                if self.attempt_hold(prev_piece.shape, event_time) {
                    None
                } else {
                    Some(prev_piece)
                }
            }
            GameEvent::Rotate(turns) => {
                let prev_piece = prev_piece.expect("rotate event but no active piece");
                self.config
                    .rotation_system
                    .rotate(&prev_piece, &self.state.board, turns)
                    .or(Some(prev_piece))
            }
            GameEvent::Move(is_initial) => {
                // Handle move attempt and auto repeat move.
                let prev_piece = prev_piece.expect("move event but no active piece");
                let dx = match (
                    self.state.buttons_pressed[Button::MoveLeft],
                    self.state.buttons_pressed[Button::MoveRight],
                ) {
                    (Some(t_left), Some(t_right)) => {
                        if t_left < t_right {
                            1
                        } else {
                            -1
                        }
                    }
                    (left, right) => {
                        if left < right {
                            1
                        } else {
                            -1
                        }
                    }
                };
                Some(
                    if let Some(next_piece) = prev_piece.fits_at(&self.state.board, (dx, 0)) {
                        let mut move_delay = if is_initial {
                            self.config.delayed_auto_shift
                        } else {
                            self.config.auto_repeat_rate
                        };
                        if let Ok(level) = NonZeroU32::try_from(self.state.gravity) {
                            move_delay = move_delay.min(
                                Self::lock_delay(level).saturating_sub(Duration::from_millis(1)),
                            );
                        }
                        self.state
                            .events
                            .insert(GameEvent::Move(false), event_time + move_delay);
                        next_piece
                    } else {
                        prev_piece
                    },
                )
            }
            GameEvent::Fall => {
                let prev_piece = prev_piece.expect("falling event but no active piece");
                // Try to drop active piece down by one, and queue next fall event.
                Some(
                    if let Some(dropped_piece) = prev_piece.fits_at(&self.state.board, (0, -1)) {
                        // Drop delay is possibly faster due to soft drop button pressed.
                        let soft_drop = self.state.buttons_pressed[Button::DropSoft]
                            .map(|_| self.config.soft_drop_factor);
                        if let Ok(level) = NonZeroU32::try_from(self.state.gravity) {
                            let drop_delay = Self::drop_delay(level, soft_drop);
                            self.state
                                .events
                                .insert(GameEvent::Fall, event_time + drop_delay);
                        }
                        dropped_piece
                    } else {
                        // Otherwise piece could not move down.
                        prev_piece
                    },
                )
            }
            GameEvent::SoftDrop => {
                let prev_piece = prev_piece.expect("softdrop event but no active piece");
                // Try to drop active piece down by one, and queue next fall event.
                Some(
                    if let Some(dropped_piece) = prev_piece.fits_at(&self.state.board, (0, -1)) {
                        let soft_drop = self.state.buttons_pressed[Button::DropSoft]
                            .map(|_| self.config.soft_drop_factor);
                        if let Ok(level) = NonZeroU32::try_from(self.state.gravity) {
                            let drop_delay = Self::drop_delay(level, soft_drop);
                            self.state
                                .events
                                .insert(GameEvent::Fall, event_time + drop_delay);
                        }
                        dropped_piece
                    } else {
                        // Otherwise ciece could not move down.
                        // Immediately queue lock (unless option for it is disabled).
                        if !self.config.no_soft_drop_lock {
                            self.state.events.insert(GameEvent::LockTimer, event_time);
                        }
                        prev_piece
                    },
                )
            }
            GameEvent::SonicDrop => {
                let prev_piece = prev_piece.expect("sonicdrop event but no active piece");
                // Move piece all the way down and nothing more.
                Some(prev_piece.well_piece(&self.state.board))
            }
            GameEvent::HardDrop => {
                let prev_piece = prev_piece.expect("harddrop event but no active piece");
                // Move piece all the way down.
                let dropped_piece = prev_piece.well_piece(&self.state.board);
                feedback_events.push((event_time, Feedback::HardDrop(prev_piece, dropped_piece)));
                self.state.events.insert(
                    GameEvent::LockTimer,
                    event_time + self.config.hard_drop_delay,
                );
                Some(dropped_piece)
            }
            GameEvent::LockTimer => {
                self.state.events.insert(GameEvent::Lock, event_time);
                prev_piece
            }
            GameEvent::Lock => {
                let prev_piece = prev_piece.expect("lock event but no active piece");
                feedback_events.push((event_time, Feedback::PieceLocked(prev_piece)));
                // Attempt to lock active piece fully above skyline - Game over.
                if prev_piece
                    .tiles()
                    .iter()
                    .all(|((_, y), _)| *y >= Self::SKYLINE)
                {
                    self.state.end = Some(Err(GameOver::LockOut));
                    return feedback_events;
                }
                self.state.pieces_played[prev_piece.shape] += 1;
                // Pre-save whether piece was spun into lock position.
                let spin = prev_piece.fits_at(&self.state.board, (0, 1)).is_none();
                // Locking.
                for ((x, y), tile_type_id) in prev_piece.tiles() {
                    self.state.board[y][x] = Some(tile_type_id);
                }
                // Handle line clear counting for score (only do actual clearing in LineClear).
                let mut lines_cleared = Vec::<usize>::with_capacity(4);
                for y in (0..Self::HEIGHT).rev() {
                    if self.state.board[y].iter().all(|mino| mino.is_some()) {
                        lines_cleared.push(y);
                    }
                }
                let n_lines_cleared = u32::try_from(lines_cleared.len()).unwrap();
                if n_lines_cleared > 0 {
                    // Add score bonus.
                    let perfect_clear = self.state.board.iter().all(|line| {
                        line.iter().all(|tile| tile.is_none())
                            || line.iter().all(|tile| tile.is_some())
                    });
                    self.state.consecutive_line_clears += 1;
                    let special_clear = n_lines_cleared >= 4 || spin || perfect_clear;
                    if special_clear {
                        self.state.back_to_back_special_clears += 1;
                    } else {
                        self.state.back_to_back_special_clears = 0;
                    }
                    let score_bonus = u32::try_from(Game::WIDTH).unwrap()
                        * (n_lines_cleared + self.state.consecutive_line_clears - 1).pow(2)
                        * self.state.back_to_back_special_clears.max(1)
                        * if spin { 4 } else { 1 }
                        * if perfect_clear { 100 } else { 1 };
                    self.state.score += u64::from(score_bonus);
                    let yippie = Feedback::Accolade {
                        score_bonus,
                        shape: prev_piece.shape,
                        spin,
                        lineclears: n_lines_cleared,
                        perfect_clear,
                        combo: self.state.consecutive_line_clears,
                        back_to_back: self.state.back_to_back_special_clears,
                    };
                    feedback_events.push((event_time, yippie));
                    feedback_events.push((
                        event_time,
                        Feedback::LineClears(lines_cleared, self.config.line_clear_delay),
                    ));
                } else {
                    self.state.consecutive_line_clears = 0;
                }
                // Clear all events and only put in line clear / appearance delay.
                self.state.events.clear();
                if n_lines_cleared > 0 {
                    self.state.events.insert(
                        GameEvent::LineClear,
                        event_time + self.config.line_clear_delay,
                    );
                } else {
                    self.state
                        .events
                        .insert(GameEvent::Spawn, event_time + self.config.appearance_delay);
                }
                self.state.hold_piece = self
                    .state
                    .hold_piece
                    .map(|(held_piece, _swap_allowed)| (held_piece, true));
                None
            }
            GameEvent::LineClear => {
                for y in (0..Self::HEIGHT).rev() {
                    // Full line: move it to the cleared lines storage and push an empty line to the board.
                    if self.state.board[y].iter().all(|mino| mino.is_some()) {
                        self.state.board.remove(y);
                        self.state.lines_cleared += 1;
                        // Increment level if 10 lines cleared.
                        if self.mode.increase_gravity && self.state.lines_cleared % 10 == 0 {
                            self.state.gravity = self.state.gravity.saturating_add(1);
                        }
                    }
                }
                while self.state.board.len() < Self::HEIGHT {
                    self.state.board.push(Default::default());
                }
                self.state
                    .events
                    .insert(GameEvent::Spawn, event_time + self.config.appearance_delay);
                None
            }
        };
        // Piece is different to before.
        if next_piece.is_some() && prev_piece != next_piece {
            // User wants to move in a direction but no move event scheduled; add a move event.
            if (self.state.buttons_pressed[Button::MoveLeft]
                != self.state.buttons_pressed[Button::MoveRight])
                && !self.state.events.contains_key(&GameEvent::Move(false))
            {
                self.state.events.insert(GameEvent::Move(false), event_time);
            }
            // No fall event scheduled but piece might be able to; add fall event.
            #[allow(clippy::map_entry)]
            if !self.state.events.contains_key(&GameEvent::Fall) {
                let soft_drop = self.state.buttons_pressed[Button::DropSoft]
                    .map(|_| self.config.soft_drop_factor);
                if let Ok(level) = NonZeroU32::try_from(self.state.gravity) {
                    let drop_delay = Self::drop_delay(level, soft_drop);
                    self.state
                        .events
                        .insert(GameEvent::Fall, event_time + drop_delay);
                }
            }
        }
        self.state.active_piece_data = next_piece.map(|next_piece| {
            (
                next_piece,
                self.calculate_locking_data(
                    event,
                    event_time,
                    prev_piece_data,
                    next_piece,
                    next_piece.fits_at(&self.state.board, (0, -1)).is_none(),
                ),
            )
        });
        feedback_events
    }

    // FIXME: THIS is, by far, the ugliest part of this entire program. For the love of what's good, I hope this code can someday be surgically excised and drop-in replaced with elegant code.
    /// Calculates the newest locking details for the main active piece.
    fn calculate_locking_data(
        &mut self,
        event: GameEvent,
        event_time: GameTime,
        prev_piece_data: Option<(ActivePiece, LockingData)>,
        next_piece: ActivePiece,
        touches_ground: bool,
    ) -> LockingData {
        let Ok(level) = NonZeroU32::try_from(self.state.gravity) else {
            // FIXME: bruh, basically a placeholder.
            return LockingData {
                touches_ground,
                last_touchdown: None,
                last_liftoff: None,
                ground_time_left: self.config.ground_time_max,
                lowest_y: Game::HEIGHT,
            };
        };
        /*
        Table (touches_ground):
        | ∅t0 !t1  :  [1] init locking data
        | ∅t0  t1  :  [3.1] init locking data, track touchdown etc., add LockTimer
        | !t0 !t1  :  [4]  -
        | !t0  t1  :  [3.2] track touchdown etc., add LockTimer
        |  t0 !t1  :  [2] track liftoff etc., RMV LockTimer
        |  t0  t1  :  [3.3] upon move/rot. add LockTimer
        */
        match (prev_piece_data, touches_ground) {
            // [1] Newly spawned piece does not touch ground.
            (None, false) => LockingData {
                touches_ground: false,
                last_touchdown: None,
                last_liftoff: Some(event_time),
                ground_time_left: self.config.ground_time_max,
                lowest_y: next_piece.position.1,
            },
            // [2] Active piece lifted off the ground.
            (Some((_prev_piece, prev_locking_data)), false) if prev_locking_data.touches_ground => {
                self.state.events.remove(&GameEvent::LockTimer);
                LockingData {
                    touches_ground: false,
                    last_liftoff: Some(event_time),
                    ..prev_locking_data
                }
            }
            // [3] A piece is on the ground. Complex update to locking values.
            (prev_piece_data, true) => {
                let next_locking_data = match prev_piece_data {
                    // If previous piece exists and next piece hasn't reached newest low (i.e. not a reset situation).
                    Some((_prev_piece, prev_locking_data))
                        if next_piece.position.1 >= prev_locking_data.lowest_y =>
                    {
                        // Previously touched ground already, just continue previous data.
                        if prev_locking_data.touches_ground {
                            prev_locking_data
                        } else {
                            // SAFETY: We know we have an active piece that didn't touch ground before, so it MUST have its last_liftoff set.
                            let last_liftoff = prev_locking_data.last_liftoff.unwrap();
                            match prev_locking_data.last_touchdown {
                                /*
                                * `(prev_piece_data, Some((next_piece, true))) = (prev_piece_data, next_piece_dat)` [[NEXT ON GROUND]]
                                * `Some((_prev_piece, prev_locking_data)) if !(next_piece.pos.1 < prev_locking_data.lowest_y) = prev_piece_data` [[ACTIVE EXISTED, NO HEIGHT RESET]]
                                * `!prev_locking_data.touches_ground` [[PREV NOT ON GROUND]]

                                last_TD    notouch    CLOSE touchnow  :  TD = prev_locking_data.last_touchdown
                                -------    notouch    CLOSE touchnow  :  TD = Some(event_time)
                                last_TD    notouch      far touchnow  :  ground_time_left -= prev_stuff...,  TD = Some(event_time)
                                -------    notouch      far touchnow  :  TD = Some(event_time)
                                */
                                // Piece was a afloat before with valid last touchdown as well.
                                Some(last_touchdown) => {
                                    let (last_touchdown, ground_time_left) = if event_time
                                        .saturating_sub(last_liftoff)
                                        <= 2 * Self::drop_delay(level, None)
                                    {
                                        (
                                            prev_locking_data.last_touchdown,
                                            prev_locking_data.ground_time_left,
                                        )
                                    } else {
                                        let elapsed_ground_time =
                                            last_liftoff.saturating_sub(last_touchdown);
                                        (
                                            Some(event_time),
                                            prev_locking_data
                                                .ground_time_left
                                                .saturating_sub(elapsed_ground_time),
                                        )
                                    };
                                    LockingData {
                                        touches_ground: true,
                                        last_touchdown,
                                        last_liftoff: None,
                                        ground_time_left,
                                        lowest_y: prev_locking_data.lowest_y,
                                    }
                                }
                                // Piece existed, was not touching ground, is touching ground now, but does not have a last touchdown. Just set touchdown.
                                None => LockingData {
                                    touches_ground: true,
                                    last_touchdown: Some(event_time),
                                    ..prev_locking_data
                                },
                            }
                        }
                    }
                    // It's a newly generated piece directly spawned on the stack, or a piece that reached new lowest and needs completely reset locking data.
                    _ => LockingData {
                        touches_ground: true,
                        last_touchdown: Some(event_time),
                        last_liftoff: None,
                        ground_time_left: self.config.ground_time_max,
                        lowest_y: next_piece.position.1,
                    },
                };
                // Set lock timer if there isn't one, or refresh it if piece was moved.
                let repositioned = prev_piece_data
                    .map(|(prev_piece, _)| prev_piece != next_piece)
                    .unwrap_or(false);
                #[rustfmt::skip]
                let move_or_rotate = matches!(event, GameEvent::Rotate(_) | GameEvent::Move(_));
                if !self.state.events.contains_key(&GameEvent::LockTimer)
                    || (repositioned && move_or_rotate)
                {
                    // SAFETY: We know this must be `Some` in this case.
                    let current_ground_time =
                        event_time.saturating_sub(next_locking_data.last_touchdown.unwrap());
                    let remaining_ground_time = next_locking_data
                        .ground_time_left
                        .saturating_sub(current_ground_time);
                    let lock_timer = std::cmp::min(Self::lock_delay(level), remaining_ground_time);
                    self.state
                        .events
                        .insert(GameEvent::LockTimer, event_time + lock_timer);
                }
                next_locking_data
            }
            // [4] No change to state (afloat before and after).
            (Some((_prev_piece, prev_locking_data)), _next_piece_dat) => prev_locking_data,
        }
    }

    /// The amount of time left for a piece to fall naturally, purely dependent on level
    /// and an optional soft-drop-factor.
    #[rustfmt::skip]
    fn drop_delay(level: NonZeroU32, soft_drop: Option<f64>) -> Duration {
        let mut drop_delay = Duration::from_nanos(match level.get() {
             0 => unreachable!(),
             1 => 1_000_000_000,
             2 =>   793_000_000,
             3 =>   617_796_000,
             4 =>   472_729_139,
             5 =>   355_196_928,
             6 =>   262_003_550,
             7 =>   189_677_245,
             8 =>   134_734_731,
             9 =>    93_882_249,
            10 =>    64_151_585,
            11 =>    42_976_258,
            12 =>    28_217_678,
            13 =>    18_153_329,
            14 =>    11_439_342,
            15 =>     7_058_616,
            16 =>     4_263_557,
            17 =>     2_520_084,
            18 =>     1_457_139,
            19 =>       823_907, // NOTE: Close to 833'333ns = 1/120 s.
            20.. =>           0, // NOTE: We cap the formula here and call it INSTANT_GRAVITY.
        });
        if let Some(soft_drop_factor) = soft_drop {
            drop_delay = drop_delay.div_f64(0.00001f64.max(soft_drop_factor));
        }
        drop_delay
    }

    /// The amount of time left for an common ground lock timer, purely dependent on level.
    #[rustfmt::skip]
    const fn lock_delay(level: NonZeroU32) -> Duration {
        Duration::from_millis(match level.get() {
            1..=29 => 500,
                30 => 480,
                31 => 460,
                32 => 440,
                33 => 420,
                34 => 400,
                35 => 380,
                36 => 360,
                37 => 340,
                38 => 320,
                39 => 300,
                40 => 280,
                41 => 260,
                42 => 240,
                43 => 220,
                _  => 200,
        })
    }
}

/// Adds an offset to a board coordinate, failing if the result is out of bounds
/// (negative or positive overflow in either direction).
pub fn add((x0, y0): Coord, (x1, y1): Offset) -> Option<Coord> {
    Some((x0.checked_add_signed(x1)?, y0.checked_add_signed(y1)?))
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
