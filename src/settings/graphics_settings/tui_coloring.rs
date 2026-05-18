use crossterm::style::Color;

use crate::settings::SlotMachine;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct TuiColoring {
    pub text: Color,
    pub accent: Color,
    pub bg: Color,
    pub widget_bg: Color,
    pub board_frame: Color,
    pub board_bg: Color,
    pub grid: Color,
}

pub fn tui_coloring_presets() -> SlotMachine<TuiColoring> {
    let slots = vec![
        ("None".to_owned(), TuiColoring::none()),
        ("White on black".to_owned(), TuiColoring::white_on_black()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "TUI Coloring".to_owned())
}

impl TuiColoring {
    pub fn none() -> Self {
        TuiColoring {
            text: Color::Reset,
            accent: Color::Reset,
            bg: Color::Reset,
            widget_bg: Color::Reset,
            board_frame: Color::Reset,
            board_bg: Color::Reset,
            grid: Color::Reset,
        }
    }

    pub fn white_on_black() -> Self {
        TuiColoring {
            text: Color::White,
            accent: Color::White,
            bg: Color::Black,
            widget_bg: Color::Black,
            board_frame: Color::White,
            board_bg: Color::Black,
            grid: Color::White,
        }
    }
}
