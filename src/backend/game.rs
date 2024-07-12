// TODO: Too many (unnecessary) derives for all the structs?
use std::{
    collections::{BTreeMap, VecDeque},
    num::NonZeroU64,
    time::{Duration, Instant},
};

use crate::backend::{rotation_systems, tetromino_generators};

pub type ButtonChange = ButtonMap<Option<bool>>;
// NOTE: Would've liked to use `impl Game { type Board = ...` (https://github.com/rust-lang/rust/issues/8995)
pub type Board = [[Option<TileTypeID>; Game::WIDTH]; Game::HEIGHT];
pub type Coord = (usize, usize);
pub type TileTypeID = u32;
type EventMap<T> = BTreeMap<TimingEvent, T>;

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
pub(crate) struct ActivePiece(pub Tetromino, pub Orientation, pub Coord);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
pub enum MeasureStat {
    Lines(u64),
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
    mode_limit: Option<MeasureStat>,
    optimization_goal: MeasureStat,
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
    Hold,
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Default, Debug)]
pub struct ButtonMap<T>(T, T, T, T, T, T, T, T);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
enum TimingEvent {
    Spawn,
    GroundCap,
    Lock,
    HardDrop,
    SoftDrop,
    Move,
    Rotate,
    Fall, // TODO: Fall timer gets reset upon manual drop.
}

// TODO: `#[derive(Debug)]`.
pub struct Game {
    // INVARIANT: `finish_status.is_some() || !next_events.is_empty()`, "Until the game has finished there will always be more events".
    // INVARIANT: `self.next_pieces().size()` stays constant.
    // Game "state" fields.
    finish_status: Option<bool>,
    events: EventMap<Instant>,
    buttons_pressed: ButtonMap<bool>,
    board: Board,
    active_piece: Option<ActivePiece>,
    next_pieces: VecDeque<Tetromino>,
    time_started: Instant,
    time_updated: Instant,
    level: u64, // TODO: Make this into NonZeroU64 or explicitly allow level 0.
    lines_cleared: u64,
    score: u64,
    // Game "settings" fields.
    mode: Gamemode,
    piece_generator: Box<dyn Iterator<Item = Tetromino>>,
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
    lines_cleared: u64,
    level: u64,
    score: u64,
    time_started: Instant,
    time_updated: Instant,
    board: &'a Board,
    active_piece: Option<[Coord; 4]>,
    ghost_piece: Option<[Coord; 4]>,
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

impl TryFrom<usize> for Tetromino {
    type Error = ();

    fn try_from(n: usize) -> Result<Self, Self::Error> {
        Ok(match n {
            0 => Tetromino::O,
            1 => Tetromino::I,
            2 => Tetromino::S,
            3 => Tetromino::Z,
            4 => Tetromino::T,
            5 => Tetromino::L,
            6 => Tetromino::J,
            _ => Err(())?,
        })
    }
}

impl ActivePiece {
    pub fn tiles(&self) -> [Coord; 4] {
        let Self(shape, o, (x, y)) = self;
        use Orientation::*;
        match shape {
            Tetromino::O => [(0, 0), (1, 0), (0, 1), (1, 1)], // ⠶
            Tetromino::I => match o {
                N | S => [(0, 0), (1, 0), (2, 0), (3, 0)], // ⠤⠤
                E | W => [(0, 0), (0, 1), (0, 2), (0, 3)], // ⡇
            },
            Tetromino::S => match o {
                N | S => [(0, 0), (1, 0), (1, 1), (2, 1)], // ⠴⠂
                E | W => [(1, 0), (0, 1), (1, 1), (0, 2)], // ⠳
            },
            Tetromino::Z => match o {
                N | S => [(1, 0), (2, 0), (0, 1), (1, 1)], // ⠲⠄
                E | W => [(0, 0), (0, 1), (1, 1), (1, 2)], // ⠞
            },
            Tetromino::T => match o {
                N => [(0, 0), (1, 0), (2, 0), (1, 1)], // ⠴⠄
                E => [(0, 0), (0, 1), (1, 1), (0, 2)], // ⠗
                S => [(1, 0), (0, 1), (1, 1), (2, 1)], // ⠲⠂
                W => [(1, 0), (0, 1), (1, 1), (1, 2)], // ⠺
            },
            Tetromino::L => match o {
                N => [(0, 0), (1, 0), (2, 0), (2, 1)], // ⠤⠆
                E => [(0, 0), (1, 0), (0, 1), (0, 2)], // ⠧
                S => [(0, 0), (0, 1), (1, 1), (2, 1)], // ⠖⠂
                W => [(1, 0), (1, 1), (0, 2), (1, 2)], // ⠹
            },
            Tetromino::J => match o {
                N => [(0, 0), (1, 0), (2, 0), (0, 1)], // ⠦⠄
                E => [(0, 0), (0, 1), (0, 2), (1, 2)], // ⠏
                S => [(2, 0), (0, 1), (1, 1), (2, 1)], // ⠒⠆
                W => [(0, 0), (1, 0), (1, 1), (1, 2)], // ⠼
            },
        }
        .map(|(dx, dy)| (x + dx, y + dy))
    }

    pub(crate) fn fits(&self, board: Board) -> bool {
        self.tiles()
            .iter()
            .all(|&(x, y)| x < Game::WIDTH && y < Game::HEIGHT && board[y][x].is_none())
    }
}

impl Gamemode {
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
            mode_limit,
            optimization_goal,
        }
    }

    pub fn sprint(start_level: NonZeroU64) -> Self {
        let start_level = start_level.get();
        Self {
            name: String::from("Sprint"),
            start_level,
            increase_level: false,
            mode_limit: Some(MeasureStat::Lines(40)),
            optimization_goal: MeasureStat::Time(Duration::ZERO),
        }
    }

    pub fn ultra(start_level: NonZeroU64) -> Self {
        let start_level = start_level.get();
        Self {
            name: String::from("Ultra"),
            start_level,
            increase_level: false,
            mode_limit: Some(MeasureStat::Time(Duration::from_secs(3 * 60))),
            optimization_goal: MeasureStat::Lines(0),
        }
    }

    pub fn marathon() -> Self {
        Self {
            name: String::from("Marathon"),
            start_level: 1,
            increase_level: true,
            mode_limit: Some(MeasureStat::Level(30)), // TODO: This depends on the highest level available.
            optimization_goal: MeasureStat::Score(0),
        }
    }

    pub fn endless() -> Self {
        Self {
            name: String::from("Endless"),
            start_level: 1,
            increase_level: true,
            mode_limit: None,
            optimization_goal: MeasureStat::Score(0),
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
            Button::Hold => &self.7,
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
            Button::Hold => &mut self.7,
        }
    }
}

impl Game {
    pub const HEIGHT: usize = 32;
    pub const WIDTH: usize = 10;

    pub fn with_gamemode(mode: Gamemode) -> Self {
        let time_started = Instant::now();
        let mut generator = tetromino_generators::RecencyProbGen::new();
        let preview_size = 1;
        let next_pieces = generator.by_ref().take(preview_size).collect();
        Game {
            finish_status: None,
            events: BTreeMap::from([(TimingEvent::Spawn, time_started)]),
            buttons_pressed: Default::default(),
            board: Default::default(),
            active_piece: None,
            next_pieces,
            time_started,
            time_updated: time_started,
            level: mode.start_level,
            lines_cleared: 0,
            score: 0,
            mode,
            piece_generator: Box::new(generator),
            rotate_fn: rotation_systems::rotate_classic,
            appearance_delay: Duration::from_millis(100),
            delayed_auto_shift: Duration::from_millis(300),
            auto_repeat_rate: Duration::from_millis(100),
            soft_drop_factor: 20.0,
            hard_drop_delay: Duration::from_micros(100),
            ground_time_cap: Duration::from_millis(2500),
            line_clear_delay: Duration::from_millis(200),
        }
    }

    pub fn info(&self) -> GameInfo {
        GameInfo {
            // TODO: Return current GameState, timeinterval (so we can render e.g. lineclears with intermediate states).
            board: &self.board,
            active_piece: self.active_piece.as_ref().map(|p| p.tiles()),
            ghost_piece: self.ghost_piece(),
            next_pieces: &self.next_pieces,
            gamemode: &self.mode,
            lines_cleared: self.lines_cleared,
            level: self.level,
            score: self.score,
            time_started: self.time_started,
            time_updated: self.time_updated,
        }
    }

    pub fn finish_status(&self) -> Option<bool> {
        self.finish_status
    }

    // TODO: Take `self` and don't leave behind `Game` in finished state where `update` can be called but does nothing.
    pub fn update(&mut self, interaction: Option<ButtonChange>, up_to: Instant) {
        // TODO: Complete state machine.
        // Handle game over: return immediately
        //
        // Spawn piece
        // Move piece
        // Drop piece
        // Check pattern (lineclear)
        // Update score (B2B?? Combos?? Perfect clears??)
        // Update level
        // Return desired next update

        if self.finish_status.is_some() {
            return;
        }
        loop {
            // SAFETY: `Game` invariant guarantees there's some event.
            let (event, time) = self.events.iter().min_by_key(|(_, &time)| time).unwrap();
            // Next event would be beyond desired point in time up to which update is requested, break out.
            if up_to < *time {
                // Update button inputs
                if let Some(button_change) = interaction {
                    // TODO: Update `ButtonMap`.
                    // Button::MoveLeft
                    // Button::MoveRight
                    // Button::RotateLeft
                    // Button::RotateRight
                    // Button::RotateAround
                    // Button::Drop
                    // Button::DropHard
                    // Button::Hold
                }
                break;
            }
            match event {
                TimingEvent::Spawn => {
                    assert!(
                        self.active_piece.is_none(),
                        "spawning a new piece while active piece is still in play"
                    );
                    let gen_tetromino = self
                        .piece_generator
                        .next()
                        .expect("random piece generator ran out of values before end of game");
                    let new_tetromino = if let Some(cached_tetromino) = self.next_pieces.pop_front()
                    {
                        self.next_pieces.push_back(gen_tetromino);
                        cached_tetromino
                    } else {
                        gen_tetromino
                    };
                    let starting_location = match new_tetromino {
                        Tetromino::O => todo!(),
                        Tetromino::I => todo!(),
                        Tetromino::S => todo!(),
                        Tetromino::Z => todo!(),
                        Tetromino::T => todo!(),
                        Tetromino::L => todo!(),
                        Tetromino::J => todo!(),
                    };
                    self.active_piece = Some(ActivePiece(
                        new_tetromino,
                        Orientation::N,
                        starting_location,
                    ));
                }
                TimingEvent::GroundCap => todo!(),
                TimingEvent::Lock => todo!(),
                TimingEvent::HardDrop => todo!(),
                TimingEvent::SoftDrop => todo!(),
                TimingEvent::Move => todo!(),
                TimingEvent::Rotate => todo!(),
                TimingEvent::Fall => todo!(),
            }
        }
    }

    #[rustfmt::skip]
    fn drop_delay(&self) -> Duration {
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
    fn lock_delay(&self) -> Duration {
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

    fn ghost_piece(&self) -> Option<[Coord; 4]> {
        todo!() // TODO: Compute ghost piece.
    }
}
