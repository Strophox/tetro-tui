use crate::settings::SlotMachine;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct TuiStyle {
    // TODO: How?
    pub glyphs: Vec<char>,
}

pub fn default_tui_style_slots() -> SlotMachine<TuiStyle> {
    let slots = vec![
        ("Unicode".to_owned(), TuiStyle::unicode()),
        ("ASCII".to_owned(), TuiStyle::ascii()),
        ("Elektronika 60".to_owned(), TuiStyle::elektronika_60()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "TUI style".to_owned())
}

impl TuiStyle {
    pub fn unicode() -> Self {
        TuiStyle { glyphs: todo!() }
    }

    pub fn ascii() -> Self {
        TuiStyle { glyphs: todo!() }
    }

    pub fn elektronika_60() -> Self {
        TuiStyle { glyphs: todo!() }
    }
}
