use crate::settings::graphics_settings::ColorSerializationType;
use crossterm::style::Color;

use crate::settings::SlotMachine;

#[serde_with::serde_as] // Do **NOT** place this after #[derive(..)] !!
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct TuiColoring {
    #[serde_as(as = "ColorSerializationType")]
    pub fg_tui: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub bg_tui: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub fg_accent: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub fg_widgetframe: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub bg_widget: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub fg_boardframe: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub bg_board: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub fg_grid: Color,
}

// TODO: Add good presets
pub fn tui_coloring_presets() -> SlotMachine<TuiColoring> {
    let slots = vec![
        ("None".to_owned(), TuiColoring::none()),
        ("White on black".to_owned(), TuiColoring::white_on_black()),
        ("Black on whit".to_owned(), TuiColoring::black_on_white()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "TUI Coloring".to_owned())
}

impl TuiColoring {
    pub fn none() -> Self {
        TuiColoring {
            fg_tui: Color::Reset,
            fg_accent: Color::Reset,
            bg_tui: Color::Reset,
            fg_widgetframe: Color::Reset,
            bg_widget: Color::Reset,
            fg_boardframe: Color::Reset,
            bg_board: Color::Reset,
            fg_grid: Color::Reset,
        }
    }

    pub fn white_on_black() -> Self {
        let fg = Color::White;
        let bg = Color::Black;
        TuiColoring {
            fg_tui: fg,
            fg_accent: fg,
            bg_tui: bg,
            fg_widgetframe: fg,
            bg_widget: bg,
            fg_boardframe: fg,
            bg_board: bg,
            fg_grid: fg,
        }
    }

    pub fn black_on_white() -> Self {
        let fg = Color::Black;
        let bg = Color::White;
        TuiColoring {
            fg_tui: fg,
            fg_accent: fg,
            bg_tui: bg,
            fg_widgetframe: fg,
            bg_widget: bg,
            fg_boardframe: fg,
            bg_board: bg,
            fg_grid: fg,
        }
    }
}
