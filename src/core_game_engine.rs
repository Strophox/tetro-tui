//! Re-exports of all core game library types (with our custom defaults).
#![allow(unused)]

// Expose all base types.
pub use falling_tetromino_engine::prelude_base::*;

// In the following, expose customized generic types.
use falling_tetromino_engine::prelude_generic;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum TileType {
    Tet(Tetromino),
    Generic,
}

impl TileType {
    pub const VARIANTS: [TileType; 8] = {
        use TileType::*;
        [
            Tet(Tetromino::I),
            Tet(Tetromino::O),
            Tet(Tetromino::S),
            Tet(Tetromino::Z),
            Tet(Tetromino::T),
            Tet(Tetromino::L),
            Tet(Tetromino::J),
            Generic,
        ]
    };
}

impl From<Tetromino> for TileType {
    fn from(value: Tetromino) -> Self {
        TileType::Tet(value)
    }
}

impl From<TileType> for usize {
    fn from(value: TileType) -> Self {
        match value {
            TileType::Tet(t) => t as usize,
            TileType::Generic => Tetromino::VARIANTS.len(),
        }
    }
}

impl From<TileType> for u8 {
    fn from(value: TileType) -> Self {
        usize::from(value) as u8
    }
}

type StdTetGen = MiscTetGens;
type StdPceRot = MiscPceRots;
type StdTileData = TileType;

pub type Board = prelude_generic::Board<StdTileData>;
pub type Line = prelude_generic::Line<StdTileData>;
pub type Notification = prelude_generic::Notification<StdTileData>;
pub type NotificationFeed = prelude_generic::NotificationFeed<StdTileData>;
pub type GameBuilder = prelude_generic::GameBuilder<StdTetGen, StdPceRot, StdTileData>;
pub type Configuration = prelude_generic::Configuration<StdPceRot>;
pub type StateInitialization = prelude_generic::StateInitialization<StdTetGen>;
pub type State = prelude_generic::State<StdTetGen, StdTileData>;
pub type Game = prelude_generic::Game<StdTetGen, StdPceRot, StdTileData>;
pub type GameAccess<'a> = prelude_generic::GameAccess<'a, StdTetGen, StdPceRot, StdTileData>;

pub use prelude_generic::GameModifier;

pub trait GameExt {
    fn level(&self) -> usize;
}

impl GameExt for Game {
    fn level(&self) -> usize {
        self.state()
            .lineclears
            .checked_div(self.config.update_delays_every_n_lineclears)
            .unwrap_or(0) as usize
    }
}
