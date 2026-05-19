use either::Either::{self, Left, Right};

use crate::{
    core_game_engine::{Tetromino, TileType},
    settings::{
        SlotMachine,
        graphics_settings::{TileTexture, UnwrapTileFromStr},
    },
};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct TileSymbols {
    pub grid: TileTexture,
    /// Contains either a uniform tile texture, or individual textures for all types of tiles that can be on a board.
    pub locked: Either<TileTexture, [TileTexture; TileType::VARIANTS.len()]>,
    /// Contains either a uniform tile texture, or individual textures for all types of tetrominos that can be played.
    pub player: Either<TileTexture, [TileTexture; Tetromino::VARIANTS.len()]>,
    pub shadow: TileTexture,
    pub hatched: TileTexture,
    pub crossed: TileTexture,
}

pub fn tile_symbols_presets() -> SlotMachine<TileSymbols> {
    let slots = vec![
        ("ASCII".to_owned(), TileSymbols::ascii()),
        ("Blocks UTF8".to_owned(), TileSymbols::blocks()),
        ("Braille".to_owned(), TileSymbols::braille()),
        ("NES simulacra".to_owned(), TileSymbols::nes()),
        ("Elektronika 60".to_owned(), TileSymbols::elektronika_60()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Tile symbols".to_owned())
}

impl TileSymbols {
    pub fn ascii() -> Self {
        TileSymbols {
            grid: " .".tile(),
            locked: Left("##".tile()), // "[]" "$$" ?
            player: Left("[]".tile()),
            shadow: "::".tile(),
            hatched: "//".tile(), // r"\\" ?
            crossed: "XX".tile(),
        }
    }

    pub fn blocks() -> Self {
        TileSymbols {
            grid: " ⢀".tile(), // " ⌟" ?
            locked: Left("██".tile()),
            player: Left("▓▓".tile()), // "▒▒"
            shadow: "░░".tile(),
            hatched: "╱╱".tile(),
            crossed: "╳╳".tile(),
        }
    }

    pub fn braille() -> Self {
        TileSymbols {
            grid: " ⢀".tile(),
            locked: Left("⣿⣿".tile()),
            player: Left("⣏⣹".tile()),
            shadow: "⠰⠆".tile(), // "⡁⢈" "⡐⠌"
            hatched: "⡜⡜".tile(),
            crossed: "⡱⢎".tile(),
        }
    }

    pub fn nes() -> Self {
        let oit = "▙▟"; //"▛▜" "🬴🬸" "█▀" "▛▀" "▄▟" "⣎⣽" "⣏⣹" "L]"
        let szlj = "Γ "; // "🭽 " "◤ " "⠋ " "\" "
        TileSymbols {
            grid: " .".tile(),
            locked: Right([oit, oit, szlj, szlj, oit, szlj, szlj, szlj].map(|s| s.tile())),
            player: Right([oit, oit, szlj, szlj, oit, szlj, szlj].map(|s| s.tile())),
            shadow: "!!".tile(), // "⠠⠂" "⡁⢈" "⡐⠌"
            hatched: "//".tile(),
            crossed: "XX".tile(),
        }
    }

    pub fn elektronika_60() -> Self {
        TileSymbols {
            grid: " .".tile(),
            locked: Left("▮▮".tile()),
            player: Left("▮▮".tile()),
            shadow: "▯▯".tile(),
            hatched: "//".tile(),
            crossed: "XX".tile(),
        }
    }

    pub fn player(&self, tet: Tetromino) -> TileTexture {
        match self.player {
            Left(textr) => textr,
            Right(textrs) => textrs[tet as usize],
        }
    }

    pub fn locked(&self, tile: TileType) -> TileTexture {
        match self.locked {
            Left(textr) => textr,
            Right(textrs) => textrs[usize::from(tile)],
        }
    }
}
