use falling_tetromino_engine::Tetromino;

use crate::settings::SlotMachine;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct MiniTetStyle {
    // FIXME: Make this efficiently serialized?
    pub tets: [char; Tetromino::VARIANTS.len()],
}

pub fn default_mini_tet_style_slots() -> SlotMachine<MiniTetStyle> {
    let slots = vec![
        ("Letters".to_owned(), MiniTetStyle::letters()),
        ("Braille".to_owned(), MiniTetStyle::braille()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Mini tet.".to_owned())
}

impl MiniTetStyle {
    pub fn letters() -> Self {
        MiniTetStyle {
            tets: ['O', 'I', 'S', 'Z', 'T', 'L', 'J'],
        }
    }

    pub fn braille() -> Self {
        MiniTetStyle {
            tets: ['⠶', '⡇', '⠳', '⠞', '⠗', '⠧', '⠼'],
        }
    }
}
