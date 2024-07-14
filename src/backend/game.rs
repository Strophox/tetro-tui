// TODO: Too many (unnecessary) derives for all the structs?
use std::{
    collections::{HashMap, VecDeque},
    num::NonZeroU64,
    time::{Duration, Instant},
};

use crate::backend::{rotation_systems, tetromino_generators};

pub type ButtonsPressed = ButtonMap<bool>;
// NOTE: Would've liked to use `impl Game { type Board = ...` (https://github.com/rust-lang/rust/issues/8995)
pub type TileTypeID = u32;
pub type Line = [Option<TileTypeID>; Game::WIDTH];
pub type Board = [Line; Game::HEIGHT];
pub type Coord = (usize, usize);
pub type Offset = (isize, isize);
type EventMap<T> = HashMap<Event, T>;

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub enum Orientation {
    N,
    E,
    S,
    W,
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub enum Tetromino {
    O,
    I,
    S,
    Z,
    T,
    L,
    J,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub(crate) struct ActivePiece {
    pub shape: Tetromino,
    pub orientation: Orientation,
    pub pos: Coord,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
pub enum MeasureStat {
    Lines(usize),
    Level(u64),
    Score(u64),
    Pieces(u64),
    Time(Duration),
}

// TODO: Manually `impl Eq, PartialEq for Gamemode`?
#[derive(Eq, PartialEq, Clone, Hash, Debug)]
pub struct Gamemode {
    name: String,
    start_level: u64,
    increase_level: bool,
    limit: Option<MeasureStat>,
    optimize: MeasureStat,
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub enum Button {
    MoveLeft,
    MoveRight,
    RotateLeft,
    RotateRight,
    RotateAround,
    DropSoft,
    DropHard,
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Default, Debug)]
pub struct ButtonMap<T>(T, T, T, T, T, T, T);

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
struct LockingData {
    touches_ground: bool,
    last_touchdown: Instant,
    last_liftoff: Instant,
    ground_time_left: Duration,
    lowest_y: usize,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
enum Event {
    LineClearDone,
    Spawn,
    //GroundTimer, // TODO:
    Lock,
    LockTimer,
    HardDrop,
    SoftDrop,
    MoveInitial,
    MoveRepeat,
    Rotate,
    Fall, // TODO: Fall timer gets reset upon manual drop.
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
enum GameOverError {
    LockOut,
    BlockOut,
}

/// TODO: Documentation. "Does not query time anywhere, it's the user's responsibility to handle time correctly."
// TODO: `#[derive(Debug)]`.
pub struct Game {
    // Game "state" fields.
    finish_status: Option<bool>,
    /// Invariants:
    /// * Until the game has finished there will always be more events: `finish_status.is_some() || !next_events.is_empty()`.
    /// * Unhandled events lie in the future: `for (event,event_time) in self.events { assert(self.time_updated < event_time); }`.
    events: EventMap<Instant>,
    buttons_pressed: ButtonsPressed,
    board: Board,
    active_piece_data: Option<(ActivePiece, LockingData)>,
    /// Invariants:
    /// * The Preview size stays constant: `self.next_pieces().size() == old(self.next_pieces().size())`.
    next_pieces: VecDeque<Tetromino>,
    /// Invariants:
    /// * The Preview size stays constant: `self.next_pieces().size() == old(self.next_pieces().size())`.
    time_started: Instant,
    time_updated: Instant,
    pieces_played: [u64; 7],
    lines_cleared: Vec<Line>,
    level: u64, // TODO: Make this into NonZeroU64 or explicitly allow level 0.
    score: u64,

    // Game "settings" fields.
    gamemode: Gamemode,
    tetromino_generator: Box<dyn Iterator<Item = Tetromino>>,
    rotate_fn: rotation_systems::RotateFn,
    appearance_delay: Duration,
    delayed_auto_shift: Duration,
    auto_repeat_rate: Duration,
    soft_drop_factor: f64,
    hard_drop_delay: Duration,
    ground_time_cap: Duration,
    line_clear_delay: Duration,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct GameInfo<'a> {
    gamemode: &'a Gamemode,
    lines_cleared: &'a Vec<Line>,
    level: u64,
    score: u64,
    time_started: Instant,
    time_updated: Instant,
    board: &'a Board,
    active_piece: Option<ActivePiece>,
    next_pieces: &'a VecDeque<Tetromino>,
}

impl Orientation {
    pub fn rotate_r(&self, right_turns: i32) -> Self {
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
    fn minos(&self, oriented: Orientation) -> [Coord; 4] {
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
}

impl TryFrom<usize> for Tetromino {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        use Tetromino::*;
        Ok(match value {
            0 => O,
            1 => I,
            2 => S,
            3 => Z,
            4 => T,
            5 => L,
            6 => J,
            _ => Err(())?,
        })
    }
}

impl From<Tetromino> for usize {
    fn from(value: Tetromino) -> Self {
        use Tetromino::*;
        match value {
            O => 0,
            I => 1,
            S => 2,
            Z => 3,
            T => 4,
            L => 5,
            J => 6,
        }
    }
}

impl ActivePiece {
    pub fn tiles(&self) -> [Coord; 4] {
        let Self {
            shape,
            orientation,
            pos: (x, y),
        } = self;
        shape.minos(*orientation).map(|(dx, dy)| (x + dx, y + dy))
    }

    pub(crate) fn fits(&self, board: Board) -> bool {
        self.tiles()
            .iter()
            .all(|&(x, y)| x < Game::WIDTH && y < Game::HEIGHT && board[y][x].is_none())
    }

    pub fn fits_at(&self, board: Board, offset: Offset) -> Option<ActivePiece> {
        let mut new_piece = *self;
        new_piece.pos = add(self.pos, offset)?;
        new_piece.fits(board).then_some(new_piece)
    }

    pub(crate) fn first_fit(
        &self,
        board: Board,
        offsets: impl IntoIterator<Item = Offset>,
    ) -> Option<ActivePiece> {
        let mut new_piece = *self;
        let old_pos = self.pos;
        offsets.into_iter().find_map(|offset| {
            new_piece.pos = add(old_pos, offset)?;
            new_piece.fits(board).then_some(new_piece)
        })
    }

    fn well_piece(&self, board: Board) -> ActivePiece {
        let mut well_piece = *self;
        // Move piece all the way down.
        while let Some(piece_below) = well_piece.fits_at(board, (0, -1)) {
            well_piece = piece_below;
        }
        well_piece
    }
}

impl Gamemode {
    #[allow(dead_code)]
    pub const fn custom(
        name: String,
        start_level: NonZeroU64,
        increase_level: bool,
        mode_limit: Option<MeasureStat>,
        optimization_goal: MeasureStat,
    ) -> Self {
        let start_level = start_level.get();
        Self {
            name,
            start_level,
            increase_level,
            limit: mode_limit,
            optimize: optimization_goal,
        }
    }

    #[allow(dead_code)]
    pub fn sprint(start_level: NonZeroU64) -> Self {
        let start_level = start_level.get();
        Self {
            name: String::from("Sprint"),
            start_level,
            increase_level: false,
            limit: Some(MeasureStat::Lines(40)),
            optimize: MeasureStat::Time(Duration::ZERO),
        }
    }

    #[allow(dead_code)]
    pub fn ultra(start_level: NonZeroU64) -> Self {
        let start_level = start_level.get();
        Self {
            name: String::from("Ultra"),
            start_level,
            increase_level: false,
            limit: Some(MeasureStat::Time(Duration::from_secs(3 * 60))),
            optimize: MeasureStat::Lines(0),
        }
    }

    #[allow(dead_code)]
    pub fn marathon() -> Self {
        Self {
            name: String::from("Marathon"),
            start_level: 1,
            increase_level: true,
            limit: Some(MeasureStat::Level(30)), // TODO: This depends on the highest level available.
            optimize: MeasureStat::Score(0),
        }
    }

    #[allow(dead_code)]
    pub fn endless() -> Self {
        Self {
            name: String::from("Endless"),
            start_level: 1,
            increase_level: true,
            limit: None,
            optimize: MeasureStat::Pieces(0),
        }
    }
    // TODO: Gamemode pub fn master() -> Self : 20G gravity mode...
    // TODO: Gamemode pub fn increment() -> Self : regain time to keep playing...
    // TODO: Gamemode pub fn finesse() -> Self : minimize Finesse(u64) for certain linecount...
}

impl<T> std::ops::Index<Button> for ButtonMap<T> {
    type Output = T;

    fn index(&self, idx: Button) -> &Self::Output {
        match idx {
            Button::MoveLeft => &self.0,
            Button::MoveRight => &self.1,
            Button::RotateLeft => &self.2,
            Button::RotateRight => &self.3,
            Button::RotateAround => &self.4,
            Button::DropSoft => &self.5,
            Button::DropHard => &self.6,
        }
    }
}

impl<T> std::ops::IndexMut<Button> for ButtonMap<T> {
    fn index_mut(&mut self, idx: Button) -> &mut Self::Output {
        match idx {
            Button::MoveLeft => &mut self.0,
            Button::MoveRight => &mut self.1,
            Button::RotateLeft => &mut self.2,
            Button::RotateRight => &mut self.3,
            Button::RotateAround => &mut self.4,
            Button::DropSoft => &mut self.5,
            Button::DropHard => &mut self.6,
        }
    }
}

impl Game {
    pub const HEIGHT: usize = 27;
    pub const WIDTH: usize = 10;

    pub fn new(mode: Gamemode, time_started: Instant) -> Self {
        let mut generator = tetromino_generators::RecencyProbGen::new();
        let preview_size = 1;
        let next_pieces = generator.by_ref().take(preview_size).collect();
        Game {
            finish_status: None,
            events: HashMap::from([(Event::Spawn, time_started)]),
            buttons_pressed: Default::default(),
            board: Default::default(),
            active_piece_data: None,
            next_pieces,
            time_started, // TODO: Refactor internal timeline to be Duration-based, shifting responsibility higher up.
            time_updated: time_started,
            pieces_played: [0; 7],
            lines_cleared: Vec::new(),
            level: mode.start_level,
            score: 0,
            gamemode: mode,
            tetromino_generator: Box::new(generator),
            rotate_fn: rotation_systems::rotate_classic,
            appearance_delay: Duration::from_millis(100),
            delayed_auto_shift: Duration::from_millis(300),
            auto_repeat_rate: Duration::from_millis(100),
            soft_drop_factor: 20.0,
            hard_drop_delay: Duration::from_micros(100),
            ground_time_cap: Duration::from_millis(2250),
            line_clear_delay: Duration::from_millis(200),
        }
    }

    pub fn finish_status(&self) -> Option<bool> {
        self.finish_status
    }

    pub fn info(&self) -> GameInfo {
        GameInfo {
            // TODO: Return current GameState, timeinterval (so we can render e.g. lineclears with intermediate states).
            board: &self.board,
            active_piece: self.active_piece_data.map(|apd| apd.0),
            next_pieces: &self.next_pieces,
            gamemode: &self.gamemode,
            lines_cleared: &self.lines_cleared,
            level: self.level,
            score: self.score,
            time_started: self.time_started,
            time_updated: self.time_updated,
        }
    }

    pub fn update(&mut self, mut new_button_state: Option<ButtonsPressed>, update_time: Instant) {
        // Handle game over: return immediately.
        if self.finish_status.is_some() {
            return;
        }
        // We linearly process all events until we reach the update time.
        'work_through_events: loop {
            // SAFETY: `Game` invariants guarantee there's some event.
            let (&event, _) = self
                .events
                .iter()
                .min_by_key(|(&event, &event_time)| (event_time, event))
                .unwrap();
            // SAFETY: `event` key was given to use by the `.min` function.
            let (event, event_time) = self.events.remove_entry(&event).unwrap();
            debug_assert!(
                self.time_updated <= event_time,
                "handling event lying in the past"
            );
            // Next event within requested update time, handle event first.
            if event_time <= update_time {
                // Handle next in-game event.
                let result = self.handle_event(event, event_time);
                self.time_updated = event_time;
                match result {
                    Ok(()) => {
                        // Check if game completed.
                        if let Some(limit) = self.gamemode.limit {
                            let goal_achieved = match limit {
                                MeasureStat::Lines(lines) => lines <= self.lines_cleared.len(),
                                MeasureStat::Level(level) => level <= self.level,
                                MeasureStat::Score(score) => score <= self.score,
                                MeasureStat::Pieces(pieces) => pieces <= self.pieces_played.iter().sum(),
                                MeasureStat::Time(timer) => timer <= self.time_updated - self.time_started,
                            };
                            if goal_achieved {
                                self.finish_status = Some(true);
                                break 'work_through_events;
                            }
                        }
                    }
                    Err(GameOverError::BlockOut | GameOverError::LockOut) => {
                        // Game Over.
                        self.finish_status = Some(false);
                        break 'work_through_events;
                    }
                }
            // Possibly process user input events now or break out.
            } else {
                // NOTE: We should be able to update the time here because `self.process_input(...)` does not access it.
                self.time_updated = update_time;
                // Update button inputs.
                if let Some(buttons_pressed) = new_button_state.take() {
                    self.process_input(buttons_pressed, update_time);
                } else {
                    break 'work_through_events;
                }
            }
        }
    }

    fn process_input(&mut self, new_buttons_pressed: ButtonsPressed, update_time: Instant) {
        #[allow(non_snake_case)]
        let ButtonMap(mL0, mR0, rL0, rR0, rA0, dS0, dH0) = self.buttons_pressed;
        #[allow(non_snake_case)]
        let ButtonMap(mL1, mR1, rL1, rR1, rA1, dS1, dH1) = new_buttons_pressed;
        /*
        Table:                                 Karnaugh map:
        | mL0 mR0 mL1 mR1                      |           !mL1 !mL1  mL1  mL1
        |  0   0   0   0  :  -                 |           !mR1  mR1  mR1 !mR1
        |  0   0   0   1  :  move, move (DAS)  | !mL0 !mR0   -   DAS   -   DAS
        |  0   0   1   0  :  move, move (DAS)  | !mL0  mR0  rem   -   rem  ARR
        |  0   0   1   1  :  -                 |  mL0  mR0   -   ARR   -   ARR
        |  0   1   0   0  :  remove            |  mL0 !mR0  rem  ARR  rem   -
        |  0   1   0   1  :  -
        |  0   1   1   0  :  move, move (ARR)
        |  0   1   1   1  :  remove
        |  1   0   0   0  :  remove
        |  1   0   0   1  :  move, move (ARR)
        |  1   0   1   0  :  -
        |  1   0   1   1  :  remove
        |  1   1   0   0  :  -
        |  1   1   0   1  :  move, move (ARR)
        |  1   1   1   0  :  move, move (ARR)
        |  1   1   1   1  :  -
        */
        // No buttons pressed -> one button pressed, add initial move.
        if (!mL0 && !mR0) && (mL1 != mR1) {
            self.events.insert(Event::MoveInitial, update_time);
        // One/Two buttons pressed -> different/one button pressed, (re-)add fast repeat move.
        } else if (mL0 && (!mL1 && mR1)) || (mR0 && (mL1 && !mR1)) {
            self.events.remove(&Event::MoveRepeat);
            self.events.insert(Event::MoveRepeat, update_time);
        // Single button pressed -> both (un)pressed, remove future moves.
        } else if (mL0 != mR0) && (mL1 == mR1) {
            self.events.remove(&Event::MoveRepeat);
        }
        /*
        Table:                       Karnaugh map:
        | rL0 rR0 rL1 rR1            |           !rR1  rR1  rR1 !rR1
        |  0   0   0   0  :  -       |           !rL1 !rL1  rL1  rL1
        |  0   0   0   1  :  rotate  | !rL0 !rR0   -   rot   -   rot
        |  0   0   1   0  :  rotate  | !rL0  rR0   -    -   rot  rot
        |  0   0   1   1  :  -       |  rL0  rR0   -    -    -    -
        |  0   1   0   0  :  -       |  rL0 !rR0   -   rot  rot   -
        |  0   1   0   1  :  -
        |  0   1   1   0  :  rotate
        |  0   1   1   1  :  rotate
        |  1   0   0   0  :  -
        |  1   0   0   1  :  rotate
        |  1   0   1   0  :  -
        |  1   0   1   1  :  rotate
        |  1   1   0   0  :  -
        |  1   1   0   1  :  -
        |  1   1   1   0  :  -
        |  1   1   1   1  :  -
        We rotate around (rA) if (!rA0 && rA1).
        This always causes a rotation event (with no cancellation possible with rL,rR).
        */
        // Either a 180 rotation, or a single L/R rotation button was pressed.
        if (!rA0 && rA1) || (((!rR0 && rR1) || (!rL0 && rL1)) && (rL0 || rR0 || !rR1 || !rL1)) {
            self.events.insert(Event::Rotate, update_time);
        }
        // Soft drop button pressed.
        if !dS0 && dS1 {
            self.events.insert(Event::SoftDrop, update_time);
        // Soft drop button released, reset drop delay immediately.
        } else if dS0 && !dS1 {
            self.events
                .insert(Event::Fall, update_time + self.drop_delay());
        }
        // Hard drop button pressed.
        if !dH0 && dH1 {
            self.events.insert(Event::HardDrop, update_time);
        }
        self.buttons_pressed = new_buttons_pressed;
    }

    fn handle_event(&mut self, event: Event, event_time: Instant) -> Result<(), GameOverError> {
        // Active piece touches the ground before update (or doesn't exist, counts as not touching).
        match event {
            Event::LineClearDone => {
                self.events
                    .insert(Event::Spawn, event_time + self.appearance_delay);
            }
            Event::Spawn => {
                // We generate a new piece above the skyline, and immediately queue a fall event for it.
                let gen_tetromino = self
                    .tetromino_generator
                    .next()
                    .expect("random piece generator ran out of values before end of game");
                let new_tetromino = if let Some(pregen_tetromino) = self.next_pieces.pop_front() {
                    self.next_pieces.push_back(gen_tetromino);
                    pregen_tetromino
                } else {
                    gen_tetromino
                };
                let start_pos = match new_tetromino {
                    Tetromino::O => (4, 21),
                    Tetromino::I => (3, 21),
                    _ => (3, 21),
                };
                debug_assert!(
                    self.active_piece_data.is_none(),
                    "spawning new piece while an active piece is still in play"
                );
                let new_piece = ActivePiece {
                    shape: new_tetromino,
                    orientation: Orientation::N,
                    pos: start_pos,
                };
                // Newly spawned piece conflicts with board - Game over!
                if !new_piece.fits(self.board) {
                    return Err(GameOverError::BlockOut);
                }
                let locking_data = LockingData {
                    touches_ground: new_piece.fits_at(self.board, (0, -1)).is_none(),
                    last_touchdown: event_time,
                    last_liftoff: event_time,
                    ground_time_left: self.ground_time_cap,
                    lowest_y: start_pos.1,
                };
                self.active_piece_data = Some((new_piece, locking_data));
                self.pieces_played[<usize>::from(new_tetromino)] += 1;
                self.events.insert(Event::Fall, event_time);
            }
            Event::Lock => {
                // TODO: Handle GameOverError.
                // TODO: Lineclear.
                // TODO: `self.lines_cleared`.
                // TODO: `self.score`.
                // TODO: `self.level`.
                todo!();
                // Clear all events and only put in line clear delay.
                self.events.clear();
                self.events
                    .insert(Event::LineClearDone, event_time + self.line_clear_delay);
            }
            Event::LockTimer => {
                self.events.insert(Event::Lock, event_time);
            }
            Event::HardDrop => {
                let Some((active_piece, _)) = self.active_piece_data.as_mut() else {
                    unreachable!("hard-dropping none active piece")
                };
                // Move piece all the way down.
                *active_piece = active_piece.well_piece(self.board);
                self.events
                    .insert(Event::LockTimer, event_time + self.hard_drop_delay);
            }
            Event::SoftDrop | Event::Fall => {
                let drop_delay = Duration::from_secs_f64(
                    self.drop_delay().as_secs_f64() / self.soft_drop_factor,
                );
                let Some((active_piece, _)) = self.active_piece_data.as_mut() else {
                    unreachable!("dropping none active piece")
                };
                // Try to move active piece down.
                if let Some(piece_below) = active_piece.fits_at(self.board, (0, -1)) {
                    *active_piece = piece_below;
                    self.events.insert(Event::Fall, event_time + drop_delay);
                // Piece hit ground but SoftDrop was pressed.
                } else if event == Event::SoftDrop {
                    self.events.insert(Event::Lock, event_time);
                // Piece hit ground and tried to drop naturally: don't do anything but try falling again later.
                } else if event == Event::Fall {
                    // TODO: Is this enough? Does this lead to inconsistent gameplay and should `Fall` be inserted outside together with locking?
                    self.events.insert(Event::Fall, event_time + drop_delay);
                }
            }
            Event::MoveInitial | Event::MoveRepeat => {
                // Handle move attempt and auto repeat move.
                let Some((active_piece, _)) = self.active_piece_data.as_mut() else {
                    unreachable!("moving none active piece")
                };
                let dx = if self.buttons_pressed[Button::MoveLeft] {
                    -1
                } else {
                    1
                };
                if let Some(moved_piece) = active_piece.fits_at(self.board, (0, dx)) {
                    *active_piece = moved_piece;
                }
                let move_delay = if event == Event::MoveInitial {
                    self.delayed_auto_shift
                } else {
                    self.auto_repeat_rate
                };
                self.events
                    .insert(Event::MoveRepeat, event_time + move_delay);
            }
            Event::Rotate => {
                let Some((active_piece, _)) = self.active_piece_data.as_mut() else {
                    unreachable!("rotating none active piece")
                };
                let mut rotation = 0;
                if self.buttons_pressed[Button::RotateLeft] {
                    rotation -= 1;
                }
                if self.buttons_pressed[Button::RotateRight] {
                    rotation += 1;
                }
                if self.buttons_pressed[Button::RotateAround] {
                    rotation += 2;
                }
                if let Some(rotated_piece) = (self.rotate_fn)(*active_piece, self.board, rotation) {
                    *active_piece = rotated_piece;
                }
            }
        }
        /*
        Event interactions and locking:
        LineClearDone: ε -> ε +Spawn dly  (no piece)
        Spawn        : ε -> ε +Fall imm  (update Locking)
        Lock         : . -> ε +dly LineClearDone  (no piece)
        HardDrop     : . -> . +Fall dly  (update Locking)
        SoftDrop     : . -> . +Fall dly  (update Locking)
        MoveInitial  : . -> . +MoveRepeat dly(DAS)  (update Locking)
        MoveRepeat   : . -> . +MoveRepeat dly(ARR)  (update Locking)
        Rotate       : . -> .  (update Locking)
        Fall         : . -> dly Fall  (update Locking)

        Table (touches_ground):
        | !t0 !t1  :  -
        | !t0  t1  :  evaluate touchdown etc., add LockTimer
        |  t0 !t1  :  remember liftoff   etc., rem LockTimer
        |  t0  t1  :  (maybe re-add LockTimer)
         */
        // TODO: Probably check this code again, because it's kind of distasteful and thus error-prone.
        // Update locking state of active piece.
        if let Some(touches_ground_after) = self
            .active_piece_data
            .as_ref()
            .map(|(active_piece, _)| active_piece.fits_at(self.board, (0, -1)).is_none())
        {
            let drop_delay = self.drop_delay();
            let lock_delay = self.lock_delay();
            if let Some((
                active_piece,
                LockingData {
                    touches_ground,
                    last_touchdown,
                    last_liftoff,
                    ground_time_left,
                    lowest_y,
                },
            )) = self.active_piece_data.as_mut()
            {
                // Piece was afloat and now landed.
                if !*touches_ground && touches_ground_after {
                    // New lowest ever reached, reset ground time completely.
                    if active_piece.pos.1 < *lowest_y {
                        *lowest_y = active_piece.pos.1;
                        *ground_time_left = self.ground_time_cap;
                        *last_touchdown = event_time;
                    // Not connected to last ground touch, update ground time and set last_touchdown.
                    } else if event_time.saturating_duration_since(*last_liftoff) > drop_delay {
                        let last_ground_time =
                            last_liftoff.saturating_duration_since(*last_touchdown);
                        *ground_time_left -= ground_time_left.saturating_sub(last_ground_time);
                        *last_touchdown = event_time;
                        // Otherwise: Connected to last ground touch, leave last_touchdown intact.
                    }
                // Piece was on ground and is now in the air.
                } else if *touches_ground && !touches_ground_after {
                    *last_liftoff = event_time;
                    self.events.remove(&Event::LockTimer);
                }
                // (Re)schedule lock timer if just landed or upon move/rotate.
                if touches_ground_after
                    && (!*touches_ground
                        || event == Event::MoveInitial
                        || event == Event::MoveRepeat
                        || event == Event::Rotate)
                {
                    let ground_time_left =
                        *ground_time_left - event_time.saturating_duration_since(*last_touchdown);
                    let delay = std::cmp::min(lock_delay, ground_time_left);
                    self.events.insert(Event::LockTimer, event_time + delay);
                }
                *touches_ground = touches_ground_after;
            }
        }
        Ok(())
    }

    #[rustfmt::skip]
    const fn drop_delay(&self) -> Duration {
        Duration::from_nanos(match self.level {
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
             _ =>       823_907, // TODO: Tweak curve so this matches `833_333`?
        })
    }

    #[rustfmt::skip]
    const fn lock_delay(&self) -> Duration {
        Duration::from_millis(match self.level {
            1..=19 => 500,
                20 => 450,
                21 => 400,
                22 => 350,
                23 => 300,
                24 => 250,
                25 => 200,
                26 => 195,
                27 => 184,
                28 => 167,
                29 => 151,
                 _ => 150, // TODO: Tweak curve?
        })
    }
}

pub(crate) fn add((x0, y0): Coord, (x1, y1): Offset) -> Option<Coord> {
    Some((x0.checked_add_signed(x1)?, y0.checked_add_signed(y1)?))
}

/* TODO: Testing?
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let res = add((1,2),(3,4));
        assert_eq!(res, (4,6));
    }
}
*/
