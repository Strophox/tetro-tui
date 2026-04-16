use crate::tui_settings::{
    graphics_settings::{TileTexture, UnwrapTileFromStr},
    SlotMachine,
};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct MinoTextures {
    pub play: TileTexture,
    pub locked: TileTexture,
    pub shadow: TileTexture,
    pub grid: TileTexture,
    pub slashed: TileTexture,
    pub crossed: TileTexture,
}

pub fn default_mino_textures_slots() -> SlotMachine<MinoTextures> {
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
            play: "[]".tile(),
            locked: "##".tile(), // "[]" "$$" ?
            shadow: "::".tile(),
            grid: " .".tile(),
            slashed: "//".tile(), // r"\\" ?
            crossed: "XX".tile(),
        }
    }

    pub fn unicode() -> Self {
        MinoTextures {
            play: "▓▓".tile(),
            locked: "██".tile(), // "▒▒"
            shadow: "░░".tile(),
            grid: " ⢀".tile(), // " ⌟" ?
            slashed: "╱╱".tile(),
            crossed: "╳╳".tile(),
        }
    }

    pub fn elektronika_60() -> Self {
        MinoTextures {
            play: "▮▮".tile(),
            locked: "▮▮".tile(),
            shadow: "▯▯".tile(),
            grid: " .".tile(),
            slashed: "//".tile(),
            crossed: "XX".tile(),
        }
    }
}
