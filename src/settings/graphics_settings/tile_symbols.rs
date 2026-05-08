use crate::settings::{
    SlotMachine,
    graphics_settings::{TileTexture, UnwrapTileFromStr},
};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct TileSymbols {
    pub grid: TileTexture,
    pub play: TileTexture,
    pub shadow: TileTexture,
    pub locked: TileTexture,
    pub hatched: TileTexture,
    pub crossed: TileTexture,
}

pub fn mino_symbols_presets() -> SlotMachine<TileSymbols> {
    let slots = vec![
        ("ASCII".to_owned(), TileSymbols::ascii()),
        ("Blocks UTF8".to_owned(), TileSymbols::blocks()),
        ("Braille".to_owned(), TileSymbols::braille()),
        ("Elektronika 60".to_owned(), TileSymbols::elektronika_60()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Tile symbols".to_owned())
}

impl TileSymbols {
    pub fn ascii() -> Self {
        TileSymbols {
            grid: " .".tile(),
            play: "[]".tile(),
            shadow: "::".tile(),
            locked: "##".tile(),  // "[]" "$$" ?
            hatched: "//".tile(), // r"\\" ?
            crossed: "XX".tile(),
        }
    }

    pub fn blocks() -> Self {
        TileSymbols {
            grid: " ⢀".tile(), // " ⌟" ?
            play: "▓▓".tile(), // "▒▒"
            shadow: "░░".tile(),
            locked: "██".tile(),
            hatched: "╱╱".tile(),
            crossed: "╳╳".tile(),
        }
    }

    pub fn braille() -> Self {
        TileSymbols {
            grid: " ⢀".tile(),
            play: "⣏⣹".tile(),
            shadow: "⠰⠆".tile(), // "⡁⢈" "⡐⠌"
            locked: "⣿⣿".tile(),
            hatched: "⡜⡜".tile(),
            crossed: "⡱⢎".tile(),
        }
    }

    pub fn elektronika_60() -> Self {
        TileSymbols {
            grid: " .".tile(),
            play: "▮▮".tile(),
            shadow: "▯▯".tile(),
            locked: "▮▮".tile(),
            hatched: "//".tile(),
            crossed: "XX".tile(),
        }
    }
}
