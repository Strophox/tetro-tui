mod rotation_systems;
mod tetromino_generators;

use std::{
    collections::{HashMap, VecDeque},
    fmt,
    num::NonZeroU32,
    ops,
    time::{Duration, Instant},
};

pub type ButtonsPressed = [bool; 7];
// NOTE: Would've liked to use `impl Game { type Board = ...` (https://github.com/rust-lang/rust/issues/8995)
pub type TileTypeID = NonZeroU32;
pub type Line = [Option<TileTypeID>; Game::WIDTH];
pub type Board = Vec<Line>;
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

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ActivePiece {
    pub shape: Tetromino,
    pub orientation: Orientation,
    pub pos: Coord,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
pub enum MeasureStat {
    Lines(usize),
    Level(NonZeroU32),
    Score(u32),
    Pieces(u32),
    Time(Duration),
}

#[derive(Eq, Clone, Hash, Debug)]
pub struct Gamemode {
    name: String,
    start_level: NonZeroU32,
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

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
struct LockingData {
    touches_ground: bool,
    last_touchdown: Option<Instant>,
    last_liftoff: Option<Instant>,
    ground_time_left: Duration,
    lowest_y: usize,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
enum Event {
    LineClear,
    Spawn,
    Lock,
    LockTimer,
    HardDrop,
    SoftDrop,
    MoveInitial,
    MoveRepeat,
    Rotate,
    Fall,
}

pub struct GameConfig {
    pub tetromino_generator: Box<dyn Iterator<Item = Tetromino>>,
    pub rotation_system: Box<dyn rotation_systems::RotationSystem>,
    pub preview_count: usize,
    pub appearance_delay: Duration,
    pub delayed_auto_shift: Duration,
    pub auto_repeat_rate: Duration,
    pub soft_drop_factor: f64,
    pub hard_drop_delay: Duration,
    pub ground_time_max: Duration,
    pub line_clear_delay: Duration,
}

#[derive(Debug)]
pub struct Game {
    // Game "constants" field.
    time_started: Instant, // TODO: Remove once internal timers are refactored to make game less dependent on real time.
    gamemode: Gamemode,

    // Game "settings" field.
    config: GameConfig,

    // Game "state" fields.
    finished: Option<Result<(), GameOver>>,
    time_updated: Instant,
    /// Invariants:
    /// * Until the game has finished there will always be more events: `finish_status.is_some() || !next_events.is_empty()`.
    /// * Unhandled events lie in the future: `for (event,event_time) in self.events { assert(self.time_updated < event_time); }`.
    events: EventMap<Instant>,
    buttons_pressed: ButtonsPressed,
    /// Invariants:
    /// * The Board height stays constant: `self.board.len() == old(self.board.len())`.
    board: Board,
    active_piece_data: Option<(ActivePiece, LockingData)>,
    /// Invariants:
    /// * The Preview size stays constant: `self.next_pieces().size() == old(self.next_pieces().size())`.
    next_pieces: VecDeque<Tetromino>,
    pieces_played: [u32; 7],
    lines_cleared: Vec<Line>,
    level: NonZeroU32,
    score: u32,
    consecutive_line_clears: u32,
    back_to_back_special_clears: u32,
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub enum GameOver {
    LockOut,
    BlockOut,
}

#[derive(Eq, PartialEq, Clone, Hash, Debug)]
pub struct GameStateView<'a> {
    pub time_started: Instant,
    pub gamemode: &'a Gamemode,

    pub lines_cleared: &'a Vec<Line>,
    pub level: NonZeroU32,
    pub score: u32,
    pub time_updated: Instant,
    pub board: &'a Board,
    pub active_piece: Option<ActivePiece>,
    pub next_pieces: &'a VecDeque<Tetromino>,
    pub pieces_played: &'a [u32; 7],
}

#[derive(Eq, PartialEq, Clone, Hash, Debug)]
pub enum FeedbackEvent {
    PieceLocked(ActivePiece),
    LineClears(Vec<usize>, Duration),
    HardDrop(ActivePiece, ActivePiece),
    Accolade {
        score_bonus: u32,
        shape: Tetromino,
        spin: bool,
        lineclears: u32,
        perfect_clear: bool,
        combo: u32,
        opportunity: u32,
    },
    Debug(String),
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

    const fn tiletypeid(&self) -> TileTypeID {
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
        unsafe { NonZeroU32::new_unchecked(u8) }
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
    pub fn tiles(&self) -> [(Coord, TileTypeID); 4] {
        let Self {
            shape,
            orientation,
            pos: (x, y),
        } = self;
        let tile_type_id = shape.tiletypeid();
        shape
            .minos(*orientation)
            .map(|(dx, dy)| ((x + dx, y + dy), tile_type_id))
    }

    pub(crate) fn fits(&self, board: &Board) -> bool {
        self.tiles()
            .iter()
            .all(|&((x, y), _)| x < Game::WIDTH && y < Game::HEIGHT && board[y][x].is_none())
    }

    pub fn fits_at(&self, board: &Board, offset: Offset) -> Option<ActivePiece> {
        let mut new_piece = *self;
        new_piece.pos = add(self.pos, offset)?;
        new_piece.fits(board).then_some(new_piece)
    }

    pub(crate) fn first_fit(
        &self,
        board: &Board,
        offsets: impl IntoIterator<Item = Offset>,
    ) -> Option<ActivePiece> {
        let mut new_piece = *self;
        let old_pos = self.pos;
        offsets.into_iter().find_map(|offset| {
            new_piece.pos = add(old_pos, offset)?;
            new_piece.fits(board).then_some(new_piece)
        })
    }

    fn well_piece(&self, board: &Board) -> ActivePiece {
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
        start_level: NonZeroU32,
        increase_level: bool,
        mode_limit: Option<MeasureStat>,
        optimization_goal: MeasureStat,
    ) -> Self {
        Self {
            name,
            start_level,
            increase_level,
            limit: mode_limit,
            optimize: optimization_goal,
        }
    }

    #[allow(dead_code)]
    pub fn sprint(start_level: NonZeroU32) -> Self {
        Self {
            name: String::from("sprint"),
            start_level,
            increase_level: false,
            limit: Some(MeasureStat::Lines(40)),
            optimize: MeasureStat::Time(Duration::ZERO),
        }
    }

    #[allow(dead_code)]
    pub fn ultra(start_level: NonZeroU32) -> Self {
        Self {
            name: String::from("ultra"),
            start_level,
            increase_level: false,
            limit: Some(MeasureStat::Time(Duration::from_secs(3 * 60))),
            optimize: MeasureStat::Lines(0),
        }
    }

    #[allow(dead_code)]
    pub fn marathon() -> Self {
        Self {
            name: String::from("marathon"),
            start_level: NonZeroU32::MIN,
            increase_level: true,
            limit: Some(MeasureStat::Level(Game::LEVEL_20G.saturating_add(1))),
            optimize: MeasureStat::Score(0),
        }
    }

    #[allow(dead_code)]
    pub fn endless() -> Self {
        Self {
            name: String::from("endless"),
            start_level: NonZeroU32::MIN,
            increase_level: true,
            limit: None,
            optimize: MeasureStat::Pieces(0),
        }
    }

    #[allow(dead_code)]
    pub fn master() -> Self {
        Self {
            name: String::from("master"),
            start_level: Game::LEVEL_20G,
            increase_level: true,
            limit: Some(MeasureStat::Lines(300)),
            optimize: MeasureStat::Score(0),
        }
    }
    // TODO: Gamemode pub fn increment() -> Self : regain time to keep playing...
    // TODO: Gamemode pub fn finesse() -> Self : minimize Finesse(u64) for certain linecount...
}

impl PartialEq for Gamemode {
    fn eq(&self, other: &Self) -> bool {
        self.start_level == other.start_level
            && self.increase_level == other.increase_level
            && self.limit == other.limit
            && self.optimize == other.optimize
    }
}

impl<T> ops::Index<Button> for [T; 7] {
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
        }
    }
}

impl<T> ops::IndexMut<Button> for [T; 7] {
    fn index_mut(&mut self, idx: Button) -> &mut Self::Output {
        match idx {
            Button::MoveLeft => &mut self[0],
            Button::MoveRight => &mut self[1],
            Button::RotateLeft => &mut self[2],
            Button::RotateRight => &mut self[3],
            Button::RotateAround => &mut self[4],
            Button::DropSoft => &mut self[5],
            Button::DropHard => &mut self[6],
        }
    }
}

impl fmt::Debug for GameConfig {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("GameConfig")
            .field(
                "tetromino_generator",
                &std::any::type_name_of_val(&self.tetromino_generator),
            )
            .field(
                "rotation_system",
                &std::any::type_name_of_val(&self.rotation_system),
            )
            .field("appearance_delay", &self.appearance_delay)
            .field("delayed_auto_shift", &self.delayed_auto_shift)
            .field("auto_repeat_rate", &self.auto_repeat_rate)
            .field("soft_drop_factor", &self.soft_drop_factor)
            .field("hard_drop_delay", &self.hard_drop_delay)
            .field("ground_time_cap", &self.ground_time_max)
            .field("line_clear_delay", &self.line_clear_delay)
            .finish()
    }
}

impl Game {
    pub const HEIGHT: usize = Self::SKYLINE + 7; // Max height *any* mino can reach before Lock out occurs.
    pub const WIDTH: usize = 10;
    pub const SKYLINE: usize = 20; // Typical maximal height of relevant (visible) playing grid.
                                   // SAFETY: 19 > 0, and this is the level at which blocks start falling with 20G.
    const LEVEL_20G: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(19) };

    pub fn with_gamemode(gamemode: Gamemode, time_started: Instant) -> Self {
        let default_config = GameConfig {
            tetromino_generator: Box::new(tetromino_generators::RecencyProbGen::new()),
            rotation_system: Box::new(rotation_systems::Classic),
            preview_count: 1,
            appearance_delay: Duration::from_millis(100),
            delayed_auto_shift: Duration::from_millis(250),
            auto_repeat_rate: Duration::from_millis(50),
            soft_drop_factor: 20.0,
            hard_drop_delay: Duration::from_micros(100),
            ground_time_max: Duration::from_millis(2250),
            line_clear_delay: Duration::from_millis(200),
        };
        Self::with_config(gamemode, time_started, default_config)
    }

    pub fn with_config(gamemode: Gamemode, time_started: Instant, mut config: GameConfig) -> Self {
        Game {
            finished: None,
            events: HashMap::from([(Event::Spawn, time_started)]),
            buttons_pressed: Default::default(),
            board: std::iter::repeat(Line::default())
                .take(Self::HEIGHT)
                .collect(),
            active_piece_data: None,
            next_pieces: config
                .tetromino_generator
                .by_ref()
                .take(config.preview_count)
                .collect(),
            time_updated: time_started,
            pieces_played: [0; 7],
            lines_cleared: Vec::new(),
            level: gamemode.start_level,
            score: 0,
            consecutive_line_clears: 0,
            back_to_back_special_clears: 0,

            config,

            time_started,
            gamemode,
        }
    }

    pub fn finished(&self) -> Option<Result<(), GameOver>> {
        self.finished
    }

    pub fn state(&self) -> GameStateView {
        GameStateView {
            board: &self.board,
            active_piece: self.active_piece_data.map(|apd| apd.0),
            next_pieces: &self.next_pieces,
            pieces_played: &self.pieces_played,
            lines_cleared: &self.lines_cleared,
            level: self.level,
            score: self.score,
            time_updated: self.time_updated,
            time_started: self.time_started,
            gamemode: &self.gamemode,
        }
    }

    pub fn config(&mut self) -> &mut GameConfig {
        &mut self.config
    }

    pub fn update(
        &mut self,
        mut new_button_state: Option<ButtonsPressed>,
        update_time: Instant,
    ) -> Result<Vec<(Instant, FeedbackEvent)>, bool> {
        // NOTE: Returning an empty Vec is efficient because it won't even allocate (as by Rust API).
        let mut feedback_events = Vec::new();
        // Handle game over: return immediately.
        if self.finished.is_some() {
            return Err(true);
        } else if !(self.time_updated <= update_time) {
            return Err(false);
        }
        // We linearly process all events until we reach the update time.
        'work_through_events: loop {
            // Peek the next closest event.
            // SAFETY: `Game` invariants guarantee there's some event.
            let (&event, &event_time) = self
                .events
                .iter()
                .min_by_key(|(&event, &event_time)| (event_time, event))
                .unwrap();
            // Next event within requested update time, handle event first.
            if event_time <= update_time {
                // Extract (remove) event and handle it.
                // SAFETY: `event` key was given to use by the `.min` function.
                self.events.remove_entry(&event);
                // Handle next in-game event.
                let result = self.handle_event(event, event_time);
                self.time_updated = event_time;
                match result {
                    Ok(new_feedback_events) => {
                        feedback_events.extend(new_feedback_events);
                        // Check if game has to end.
                        if let Some(limit) = self.gamemode.limit {
                            let goal_achieved = match limit {
                                MeasureStat::Lines(lines) => lines <= self.lines_cleared.len(),
                                MeasureStat::Level(level) => level <= self.level,
                                MeasureStat::Score(score) => score <= self.score,
                                MeasureStat::Pieces(pieces) => {
                                    pieces <= self.pieces_played.iter().sum()
                                }
                                MeasureStat::Time(timer) => {
                                    timer <= self.time_updated - self.time_started
                                }
                            };
                            if goal_achieved {
                                // Game Completed.
                                self.finished = Some(Ok(()));
                                break 'work_through_events;
                            }
                        }
                    }
                    Err(gameover) => {
                        // Game Over.
                        self.finished = Some(Err(gameover));
                        break 'work_through_events;
                    }
                }
            // Possibly process user input events now or break out.
            } else {
                // NOTE: We should be able to update the time here because `self.process_input(...)` does not access it.
                self.time_updated = update_time;
                // Update button inputs.
                if let Some(buttons_pressed) = new_button_state.take() {
                    if self.active_piece_data.is_some() {
                        self.process_input(buttons_pressed, update_time);
                    }
                    self.buttons_pressed = buttons_pressed;
                } else {
                    break 'work_through_events;
                }
            }
        }
        Ok(feedback_events)
    }

    fn process_input(&mut self, new_buttons_pressed: ButtonsPressed, update_time: Instant) {
        #[allow(non_snake_case)]
        let [mL0, mR0, rL0, rR0, rA0, dS0, dH0] = self.buttons_pressed;
        #[allow(non_snake_case)]
        let [mL1, mR1, rL1, rR1, rA1, dS1, dH1] = new_buttons_pressed;
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
        }
        // Hard drop button pressed.
        if !dH0 && dH1 {
            self.events.insert(Event::HardDrop, update_time);
        }
    }

    fn handle_event(
        &mut self,
        event: Event,
        event_time: Instant,
    ) -> Result<Vec<(Instant, FeedbackEvent)>, GameOver> {
        // Active piece touches the ground before update (or doesn't exist, counts as not touching).
        let mut feedback_events = Vec::new();
        // TODO: Remove debug.
        // feedback_events.push((
        //     event_time,
        //     FeedbackEvent::Debug(format!("{event:?} at {event_time:?}")),
        // ));
        let prev_piece_data = self.active_piece_data;
        let prev_piece = prev_piece_data.unzip().0;
        let next_piece = match event {
            // We generate a new piece above the skyline, and immediately queue a fall event for it.
            Event::Spawn => {
                debug_assert!(
                    prev_piece.is_none(),
                    "spawning new piece while an active piece is still in play"
                );
                let n_required_pieces = 1 + self
                    .config
                    .preview_count
                    .saturating_sub(self.next_pieces.len());
                self.next_pieces.extend(
                    self.config
                        .tetromino_generator
                        .by_ref()
                        .take(n_required_pieces),
                );
                let tetromino = self
                    .next_pieces
                    .pop_front()
                    .expect("piece generator ran out before game finished");
                let next_piece = self.config.rotation_system.place_initial(tetromino);
                // Newly spawned piece conflicts with board - Game over.
                if !next_piece.fits(&self.board) {
                    return Err(GameOver::BlockOut);
                }
                self.pieces_played[<usize>::from(tetromino)] += 1;
                self.events.insert(Event::Fall, event_time);
                Some(next_piece)
            }
            Event::Rotate => {
                let prev_piece = prev_piece.expect("rotating none active piece");
                // Special 20G fall immediately after.
                if self.level >= Self::LEVEL_20G {
                    self.events.insert(Event::Fall, event_time);
                }
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
                self.config
                    .rotation_system
                    .rotate(&prev_piece, &self.board, rotation)
                    .or(Some(prev_piece))
            }
            Event::MoveInitial | Event::MoveRepeat => {
                // Handle move attempt and auto repeat move.
                let prev_piece = prev_piece.expect("moving none active piece");
                // Special 20G fall immediately after.
                if self.level >= Self::LEVEL_20G {
                    self.events.insert(Event::Fall, event_time);
                }
                let move_delay = if event == Event::MoveInitial {
                    self.config.delayed_auto_shift
                } else {
                    self.config.auto_repeat_rate
                };
                self.events
                    .insert(Event::MoveRepeat, event_time + move_delay);
                #[rustfmt::skip]
                let dx = if self.buttons_pressed[Button::MoveLeft] { -1 } else { 1 };
                prev_piece
                    .fits_at(&self.board, (dx, 0))
                    .or(Some(prev_piece))
            }
            Event::Fall | Event::SoftDrop => {
                let prev_piece = prev_piece.expect("falling/softdropping none active piece");
                if self.level >= Self::LEVEL_20G {
                    Some(prev_piece.well_piece(&self.board))
                } else {
                    let drop_delay = if self.buttons_pressed[Button::DropSoft] {
                        Duration::from_secs_f64(
                            self.drop_delay().as_secs_f64() / self.config.soft_drop_factor,
                        )
                    } else {
                        self.drop_delay()
                    };
                    // Try to move active piece down.
                    if let Some(dropped_piece) = prev_piece.fits_at(&self.board, (0, -1)) {
                        self.events.insert(Event::Fall, event_time + drop_delay);
                        Some(dropped_piece)
                    // Piece hit ground but SoftDrop was pressed.
                    } else if event == Event::SoftDrop {
                        self.events.insert(Event::Lock, event_time);
                        Some(prev_piece)
                    // Piece hit ground and tried to drop naturally: don't do anything but try falling again later.
                    } else {
                        // NOTE: This could be changed if a reason for it appears.
                        self.events.insert(Event::Fall, event_time + drop_delay);
                        Some(prev_piece)
                    }
                }
            }
            Event::HardDrop => {
                let prev_piece = prev_piece.expect("harddropping none active piece");
                // Move piece all the way down.
                let dropped_piece = prev_piece.well_piece(&self.board);
                feedback_events.push((
                    event_time,
                    FeedbackEvent::HardDrop(prev_piece, dropped_piece),
                ));
                self.events
                    .insert(Event::LockTimer, event_time + self.config.hard_drop_delay);
                Some(dropped_piece)
            }
            Event::LockTimer => {
                self.events.insert(Event::Lock, event_time);
                prev_piece
            }
            Event::Lock => {
                let prev_piece = prev_piece.expect("locking none active piece");
                // Attempt to lock active piece fully above skyline - Game over.
                if prev_piece
                    .tiles()
                    .iter()
                    .any(|((_, y), _)| *y >= Self::SKYLINE)
                {
                    return Err(GameOver::LockOut);
                }
                feedback_events.push((event_time, FeedbackEvent::PieceLocked(prev_piece)));
                // Pre-save whether piece was spun into lock position.
                let spin = prev_piece.fits_at(&self.board, (0, 1)).is_none();
                // Locking.
                for ((x, y), tile_type_id) in prev_piece.tiles() {
                    self.board[y][x] = Some(tile_type_id);
                }
                // Handle line clear counting for score (only do actual clearing in LineClear).
                let mut lines_cleared = Vec::<usize>::with_capacity(4);
                for y in (0..Self::HEIGHT).rev() {
                    if self.board[y].iter().all(|mino| mino.is_some()) {
                        lines_cleared.push(y);
                    }
                }
                let n_lines_cleared = u32::try_from(lines_cleared.len()).unwrap();
                if n_lines_cleared > 0 {
                    let n_tiles_used = u32::try_from(
                        prev_piece
                            .tiles()
                            .iter()
                            .filter(|((_, y), _)| lines_cleared.contains(y))
                            .count(),
                    )
                    .unwrap();
                    // Add score bonus.
                    let perfect_clear = self
                        .board
                        .iter()
                        .all(|line| line.iter().all(|tile| tile.is_none()));
                    self.consecutive_line_clears += 1;
                    let special_clear = n_lines_cleared >= 4 || spin || perfect_clear;
                    if special_clear {
                        self.back_to_back_special_clears += 1;
                    } else {
                        self.back_to_back_special_clears = 0;
                    }
                    let score_bonus = (10 + self.level.get() - 1)
                        * n_lines_cleared
                        * n_tiles_used
                        * if spin { 2 } else { 1 }
                        * if perfect_clear { 10 } else { 1 }
                        * self.consecutive_line_clears;
                    self.score += score_bonus;
                    let yippie = FeedbackEvent::Accolade {
                        score_bonus,
                        shape: prev_piece.shape,
                        spin,
                        lineclears: n_lines_cleared,
                        perfect_clear,
                        combo: self.consecutive_line_clears,
                        opportunity: n_tiles_used,
                    };
                    feedback_events.push((event_time, yippie));
                    feedback_events.push((
                        event_time,
                        FeedbackEvent::LineClears(lines_cleared, self.config.line_clear_delay),
                    ));
                } else {
                    self.consecutive_line_clears = 0;
                }
                // Clear all events and only put in line clear / appearance delay.
                self.events.clear();
                if n_lines_cleared > 0 {
                    self.events
                        .insert(Event::LineClear, event_time + self.config.line_clear_delay);
                } else {
                    self.events
                        .insert(Event::Spawn, event_time + self.config.appearance_delay);
                }
                None
            }
            Event::LineClear => {
                for y in (0..Self::HEIGHT).rev() {
                    // Full line: move it to the cleared lines storage and push an empty line to the board.
                    if self.board[y].iter().all(|mino| mino.is_some()) {
                        let line = self.board.remove(y);
                        self.board.push(Default::default());
                        self.lines_cleared.push(line);
                    }
                }
                // Increment level if 10 lines cleared.
                if self.lines_cleared.len() % 10 == 0 {
                    self.level = self.level.saturating_add(1);
                }
                self.events
                    .insert(Event::Spawn, event_time + self.config.appearance_delay);
                None
            }
        };
        let next_piece_dat =
            next_piece.map(|piece| (piece, piece.fits_at(&self.board, (0, -1)).is_none()));
        let next_locking_data =
            self.calculate_locking_data(event, event_time, prev_piece_data, next_piece_dat);
        self.active_piece_data = next_piece.zip(next_locking_data);
        Ok(feedback_events)
    }

    // THIS is, by far, the ugliest part of this entire game. For the love of what's good, I hope this code can someday be surgically removed and replaced with an elegant alternative.
    fn calculate_locking_data(
        &mut self,
        event: Event,
        event_time: Instant,
        prev_piece_data: Option<(ActivePiece, LockingData)>,
        next_piece_dat: Option<(ActivePiece, bool)>,
    ) -> Option<LockingData> {
        /*
        Table (touches_ground):
        | ∅t0 !t1  :  [1] init locking data
        | ∅t0  t1  :  [3.1] init locking data, track touchdown etc., add LockTimer
        | ∅t0 ∅t1  :  [4]  -
        | !t0 !t1  :  [4]  -
        | !t0  t1  :  [3.2] track touchdown etc., add LockTimer
        | !t0 ∅t1  :  [4]  -
        |  t0 !t1  :  [2] track liftoff etc., RMV LockTimer
        |  t0  t1  :  [3.3] upon move/rot. add LockTimer
        |  t0 ∅t1  :  [4]  -
        */
        match (prev_piece_data, next_piece_dat) {
            // [1] Newly spawned piece does not touch ground.
            (None, Some((next_piece, false))) => Some(LockingData {
                touches_ground: false,
                last_touchdown: None,
                last_liftoff: Some(event_time),
                ground_time_left: self.config.ground_time_max,
                lowest_y: next_piece.pos.1,
            }),
            // [2] Active piece lifted off the ground.
            (Some((_prev_piece, prev_locking_data)), Some((_next_piece, false)))
                if prev_locking_data.touches_ground =>
            {
                self.events.remove(&Event::LockTimer);
                Some(LockingData {
                    touches_ground: false,
                    last_liftoff: Some(event_time),
                    ..prev_locking_data
                })
            }
            // [3] A piece is on the ground. Complex update to locking values.
            (prev_piece_data, Some((next_piece, true))) => {
                let next_locking_data = match prev_piece_data {
                    // If previous piece exists and next piece hasn't reached newest low (i.e. not a reset situation).
                    Some((_prev_piece, prev_locking_data))
                        if !(next_piece.pos.1 < prev_locking_data.lowest_y) =>
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
                                        .saturating_duration_since(last_liftoff)
                                        <= 2 * self.drop_delay()
                                    {
                                        (
                                            prev_locking_data.last_touchdown,
                                            prev_locking_data.ground_time_left,
                                        )
                                    } else {
                                        let elapsed_ground_time =
                                            last_liftoff.saturating_duration_since(last_touchdown);
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
                        lowest_y: next_piece.pos.1,
                    },
                };
                // Set lock timer if there isn't one, or refresh it if piece was moved.
                let repositioned = prev_piece_data
                    .map(|(prev_piece, _)| prev_piece != next_piece)
                    .unwrap_or(false);
                #[rustfmt::skip]
                let move_rotate = match event { Event::Rotate | Event::MoveInitial | Event::MoveRepeat => true, _ => false };
                if !self.events.contains_key(&Event::LockTimer) || (repositioned && move_rotate) {
                    // SAFETY: We know this must be `Some` in this case.
                    let current_ground_time = event_time
                        .saturating_duration_since(next_locking_data.last_touchdown.unwrap());
                    let remaining_ground_time = next_locking_data
                        .ground_time_left
                        .saturating_sub(current_ground_time);
                    let lock_timer = std::cmp::min(self.lock_delay(), remaining_ground_time);
                    // TODO: Remove debug.
                    // let dbgstr = format!("HEREEEEEEEEEEEEEEEEEEE {:?}", lock_timer);
                    // crossterm::ExecutableCommand::execute(&mut std::io::stderr(), crossterm::cursor::MoveTo(0,0)).unwrap();
                    // crossterm::ExecutableCommand::execute(&mut std::io::stderr(), crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)).unwrap();
                    // crossterm::ExecutableCommand::execute(&mut std::io::stderr(), crossterm::style::Print(dbgstr)).unwrap();
                    // crossterm::ExecutableCommand::execute(&mut std::io::stderr(), crossterm::cursor::MoveToNextLine(1)).unwrap();
                    self.events
                        .insert(Event::LockTimer, event_time + lock_timer);
                }
                Some(next_locking_data)
            }
            // [4] No change to state or no active piece.
            (prev_piece_data, _next_piece_dat) => prev_piece_data.unzip().1,
        }
    }

    // TODO: This curve could be tweaked.
    #[rustfmt::skip]
    const fn drop_delay(&self) -> Duration {
        Duration::from_nanos(match self.level.get() {
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
             _ =>       823_907, // NOTE: 20G is at `833_333`, but falling speeds at that level are handled especially by the engine.
        })
    }

    // TODO: This curve could be tweaked.
    #[rustfmt::skip]
    const fn lock_delay(&self) -> Duration {
        Duration::from_millis(match self.level.get() {
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
                 _ => 150,
        })
    }
}

pub(crate) fn add((x0, y0): Coord, (x1, y1): Offset) -> Option<Coord> {
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
