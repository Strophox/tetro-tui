use crate::tui_settings::{
    SlotMachine,
    graphics_settings::{TileTexture, UnwrapTileFromStr},
};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct MinoTextures {
    pub grid: TileTexture,
    pub play: TileTexture,
    pub shadow: TileTexture,
    pub locked: TileTexture,
    pub hatched: TileTexture,
    pub crossed: TileTexture,
}

pub fn mino_symbols_presets() -> SlotMachine<MinoTextures> {
    let slots = vec![
        ("ASCII".to_owned(), MinoTextures::ascii()),
        ("Unicode".to_owned(), MinoTextures::unicode()),
        ("Elektronika 60".to_owned(), MinoTextures::elektronika_60()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Mino textures".to_owned())
}

impl MinoTextures {
    pub fn ascii() -> Self {
        MinoTextures {
            grid: " .".tile(),
            play: "[]".tile(),
            shadow: "::".tile(),
            locked: "##".tile(),  // "[]" "$$" ?
            hatched: "//".tile(), // r"\\" ?
            crossed: "XX".tile(),
        }
    }

    pub fn unicode() -> Self {
        MinoTextures {
            grid: " ⢀".tile(), // " ⌟" ?
            play: "▓▓".tile(),
            shadow: "░░".tile(),
            locked: "██".tile(), // "▒▒"
            hatched: "╱╱".tile(),
            crossed: "╳╳".tile(),
        }
    }

    pub fn elektronika_60() -> Self {
        MinoTextures {
            grid: " .".tile(),
            play: "▮▮".tile(),
            shadow: "▯▯".tile(),
            locked: "▮▮".tile(),
            hatched: "//".tile(),
            crossed: "XX".tile(),
        }
    }
}
