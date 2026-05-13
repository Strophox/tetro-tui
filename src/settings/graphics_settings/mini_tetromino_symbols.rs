use crate::tetromino_engine::Tetromino;

use crate::settings::SlotMachine;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
// #[serde(transparent)]
#[serde(into = "String", try_from = "String")]
pub struct MiniTetrominoSymbols {
    pub tets: [char; Tetromino::VARIANTS.len()],
}

pub fn mini_tetromino_symbols_presets() -> SlotMachine<MiniTetrominoSymbols> {
    let slots = vec![
        ("Letters".to_owned(), MiniTetrominoSymbols::letters()),
        ("Braille".to_owned(), MiniTetrominoSymbols::braille()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Mini tet.".to_owned())
}

impl MiniTetrominoSymbols {
    pub fn letters() -> Self {
        MiniTetrominoSymbols {
            tets: ['O', 'I', 'S', 'Z', 'T', 'L', 'J'],
        }
    }

    pub fn braille() -> Self {
        MiniTetrominoSymbols {
            tets: ['⠶', '⡇', '⠳', '⠞', '⠗', '⠧', '⠼'],
        }
    }
}

impl From<MiniTetrominoSymbols> for String {
    fn from(value: MiniTetrominoSymbols) -> Self {
        value.tets.iter().collect()
    }
}

impl TryFrom<String> for MiniTetrominoSymbols {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let tets = value
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(|x| format!("Error: {x:?}"))?;
        Ok(MiniTetrominoSymbols { tets })
    }
}
