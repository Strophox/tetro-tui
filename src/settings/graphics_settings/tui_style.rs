use crate::settings::SlotMachine;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct TuiStyle {
    // TODO: How?
    pub glyphmap: Vec<char>,
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
        TuiStyle {
            glyphmap: "─┌└|╓╶╖║╙▀╜─┐│╴┤┬─┘".chars().collect(),
        }
    }
    pub fn ascii() -> Self {
        TuiStyle {
            glyphmap: "".chars().collect(),
        }
    }
    pub fn elektronika_60() -> Self {
        TuiStyle {
            glyphmap: "".chars().collect(),
        }
    }
}
