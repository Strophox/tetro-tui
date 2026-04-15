use falling_tetromino_engine::Tetromino;

use crate::tui_settings::SlotMachine;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
// #[serde(transparent)]
#[serde(into = "String", try_from = "String")]
pub struct MiniTetStyle {
    pub tets: [char; Tetromino::VARIANTS.len()],
}

pub fn default_mini_tet_style_slots() -> SlotMachine<MiniTetStyle> {
    let slots = vec![
        ("ASCII".to_owned(), MiniTetStyle::letters()),
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

impl From<MiniTetStyle> for String {
    fn from(value: MiniTetStyle) -> Self {
        value.tets.iter().collect()
    }
}

impl TryFrom<String> for MiniTetStyle {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let tets = value
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(|x| format!("Error: {x:?}"))?;
        Ok(MiniTetStyle { tets })
    }
}
