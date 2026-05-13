use crate::tetromino_engine::Tetromino;

use crate::settings::SlotMachine;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct SmallTetrominoSymbols {
    pub tets: [String; Tetromino::VARIANTS.len()],
    pub parts: [char; 4],
}

pub fn small_tetromino_symbols_presets() -> SlotMachine<SmallTetrominoSymbols> {
    let slots = vec![
        ("Dots ASCII".to_owned(), SmallTetrominoSymbols::dots_ascii()),
        ("Blocks UTF8".to_owned(), SmallTetrominoSymbols::blocks()),
        ("Braille".to_owned(), SmallTetrominoSymbols::braille()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Small tet.".to_owned())
}

impl SmallTetrominoSymbols {
    pub fn dots_ascii() -> Self {
        SmallTetrominoSymbols {
            tets: ["::", "....", ".:'", "':.", ".:.", "..:", ":.."].map(ToOwned::to_owned),
            parts: [' ', '.', '\'', ':'],
        }
    }

    pub fn blocks() -> Self {
        SmallTetrominoSymbols {
            tets: ["██", "▄▄▄▄", "▄█▀", "▀█▄", "▄█▄", "▄▄█", "█▄▄"].map(ToOwned::to_owned),
            parts: [' ', '▄', '▀', '█'],
        }
    }

    pub fn braille() -> Self {
        SmallTetrominoSymbols {
            tets: ["⣿⣿", "⣤⣤⣤⣤", "⣤⣿⠛", "⠛⣿⣤", "⣤⣿⣤", "⣤⣤⣿", "⣿⣤⣤"].map(ToOwned::to_owned),
            parts: [' ', '⣤', '⠛', '⣿'],
        }
    }
}
