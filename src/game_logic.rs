use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub type Board = [[Option<TileTypeID>; Game::WIDTH]; Game::HEIGHT];
pub type Coord = (usize,usize);
pub type TileTypeID = u32;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Orientation {
  N, E, S, W,
}

impl Orientation {
    pub fn rotate_r(&self, right_turns: i32) -> Self {
        use Orientation::*;
        let base = match self { N => 0, E => 1, S => 2, W => 3, };
        match (base + right_turns).rem_euclid(4) { 0 => N, 1 => E, 2 => S, 3 => W, }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub(crate) enum Tetromino {
  O,
  I,
  S,
  Z,
  T,
  L,
  J,
}

impl TryFrom<usize> for Tetromino {
    type Error = ();

    fn try_from(n: usize) -> Result<Self, Self::Error> {
        use Tetromino::*;
        Ok(match n {
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

pub(crate) struct GamePiece(pub Tetromino, pub Orientation, pub Coord);

impl GamePiece {
    // Given a piece, return a list of (x,y) mino positions
    fn minos(&self) -> [Coord; 4] {
        let Self(shape, o, (x,y)) = self;
        use Orientation::*;
        match shape {
            Tetromino::O => [(0,0),(1,0),(0,1),(1,1)], // ⠶
            Tetromino::I => match o {
                N | S => [(0,0),(1,0),(2,0),(3,0)], // ⠤⠤
                E | W => [(0,0),(0,1),(0,2),(0,3)], // ⡇
            },
            Tetromino::S => match o {
                N | S => [(1,0),(2,0),(0,1),(1,1)], // ⠴⠂
                E | W => [(0,0),(0,1),(1,1),(1,2)], // ⠳
            },
            Tetromino::Z => match o {
                N | S => [(0,0),(1,0),(1,1),(2,1)], // ⠲⠄
                E | W => [(1,0),(0,1),(1,1),(0,2)], // ⠞
            },
            Tetromino::T => match o {
                N => [(1,0),(0,1),(1,1),(2,1)], // ⠴⠄
                E => [(0,0),(0,1),(1,1),(0,2)], // ⠗
                S => [(0,0),(1,0),(2,0),(1,1)], // ⠲⠂
                W => [(1,0),(0,1),(1,1),(1,2)], // ⠺
            },
            Tetromino::L => match o {
                N => [(2,0),(0,1),(1,1),(2,1)], // ⠤⠆
                E => [(0,0),(0,1),(0,2),(1,2)], // ⠧
                S => [(0,0),(1,0),(2,0),(0,1)], // ⠖⠂
                W => [(0,0),(1,0),(1,1),(1,2)], // ⠹
            },
            Tetromino::J => match o {
                N => [(0,0),(0,1),(1,1),(2,1)], // ⠦⠄
                E => [(0,0),(1,0),(0,1),(0,2)], // ⠏
                S => [(0,0),(1,0),(2,0),(2,1)], // ⠒⠆
                W => [(1,0),(1,1),(0,2),(1,2)], // ⠼
            },
        }.map(|(dx,dy)| (x+dx,y+dy))
    }

    pub fn fits(&self, board: Board) -> bool {
        let has_space = |&(x,y)| x < Game::WIDTH && y < Game::HEIGHT && board[y][x].is_none();
        self.minos().iter().all(has_space)
    }
    
}


#[derive(Default, Debug)]
pub struct ButtonMap<T> {
    move_left: T,
    move_right: T,
    rotate_left: T,
    rotate_right: T,
    drop_soft: T,
    drop_hard: T,
    rotate_180: T,
    hold: T,
}

pub enum ButtonChange {
    Press,
    Release,
}

// Stores the complete game state at a given instant.
pub struct Game {
    // Settings internal
    time_started: Instant, // TODO
    last_updated: Instant, // TODO
    piece_generator: Box<dyn Iterator<Item=Tetromino>>,
    // State
    buttons_pressed: ButtonMap<bool>,
    board: Board,
    active_piece: Option<(Tetromino, Orientation, Coord)>,
    preview_pieces: VecDeque<Tetromino>,
    // Statistics
    score: u64,
    level: u64,
    lines_cleared: u64,
}

impl Game {
    pub const HEIGHT: usize = 22;
    pub const WIDTH: usize = 10;

    pub fn new() -> Self {
        let time_started = Instant::now();
        let generator = crate::tetromino_generators::RecencyProbGen::new();
        let preview_size = 1;
        let preview_pieces = generator.take(preview_size).collect();
        Game {
            time_started,
            last_updated: time_started,
            piece_generator: Box::new(generator),
            
            buttons_pressed: ButtonMap::default(),
            board: Default::default(),
            active_piece: None,
            preview_pieces,
            
            score: 0,
            level: 0,
            lines_cleared: 0,
        }
    }

    pub fn get<'a>(&'a self) -> (&'a Board,) {
        (&self.board, self.score, )
    }

    pub fn update(&mut self, buttons: ButtonMap<Option<ButtonChange>>, now: Instant) -> Instant {
        // TODO
    }
}