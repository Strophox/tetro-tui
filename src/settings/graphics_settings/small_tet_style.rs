use falling_tetromino_engine::Tetromino;

use crate::settings::SlotMachine;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct SmallTetStyle {
    pub tets: [String; Tetromino::VARIANTS.len()],
    pub constituens: [char; 4],
}

pub fn default_small_tet_style_slots() -> SlotMachine<SmallTetStyle> {
    let slots = vec![
        ("ASCII".to_owned(), SmallTetStyle::ascii()),
        ("Blocks".to_owned(), SmallTetStyle::blocks()),
        ("Braille".to_owned(), SmallTetStyle::braille()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Small tet.".to_owned())
}

impl SmallTetStyle {
    pub fn ascii() -> Self {
        SmallTetStyle {
            tets: ["::", "....", ".:°", "°:.", ".:.", "..:", ":.."].map(ToOwned::to_owned),
            constituens: [' ', '.', '°', ':'],
        }
    }

    pub fn blocks() -> Self {
        SmallTetStyle {
            tets: ["██", "▄▄▄▄", "▄█▀", "▀█▄", "▄█▄", "▄▄█", "█▄▄"].map(ToOwned::to_owned),
            constituens: [' ', '▄', '▀', '█'],
        }
    }

    pub fn braille() -> Self {
        SmallTetStyle {
            tets: ["⣿⣿", "⣤⣤⣤⣤", "⣤⣿⠛", "⠛⣿⣤", "⣤⣿⣤", "⣤⣤⣿", "⣿⣤⣤"].map(ToOwned::to_owned),
            constituens: [' ', '⣤', '⠛', '⣿'],
        }
    }
}
