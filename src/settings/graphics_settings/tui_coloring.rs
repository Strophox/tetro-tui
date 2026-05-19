use crate::settings::graphics_settings::{
    ColorSerializationType,
    tile_coloring::{NES_BLACK, NES_GRAY, NES_PALETTE, NES_WHITE},
};
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
    pub fg_accent: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub fg_widgetframe: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub fg_boardframe: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub fg_grid: Color,

    #[serde_as(as = "ColorSerializationType")]
    pub bg_tui: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub bg_widget: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub bg_boardframe: Color,
    #[serde_as(as = "ColorSerializationType")]
    pub bg_board: Color,
}

pub fn tui_coloring_presets() -> SlotMachine<TuiColoring> {
    let slots = vec![
        ("Terminal default".to_owned(), TuiColoring::term_default()),
        ("Just black/white".to_owned(), TuiColoring::white_on_black()),
        ("Tetro Dark".to_owned(), TuiColoring::tetro_dark()),
        ("Gruvbox Dark".to_owned(), TuiColoring::gruvbox_dark()),
        ("Solarized Light".to_owned(), TuiColoring::solarized_light()),
        ("Matrix".to_owned(), TuiColoring::matrix()),
        ("Sequoia".to_owned(), TuiColoring::sequoia()),
        ("Just amber".to_owned(), TuiColoring::amber()),
        ("NES".to_owned(), TuiColoring::nes()),
        ("OneHalfDark".to_owned(), TuiColoring::onehalfdark()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "TUI Coloring".to_owned())
}

impl TuiColoring {
    pub fn term_default() -> Self {
        TuiColoring {
            fg_tui: Color::Reset,
            fg_accent: Color::Reset,
            fg_widgetframe: Color::Reset,
            fg_boardframe: Color::Reset,
            fg_grid: Color::Reset,

            bg_tui: Color::Reset,
            bg_widget: Color::Reset,
            bg_boardframe: Color::Reset,
            bg_board: Color::Reset,
        }
    }

    pub fn tetro_dark() -> Self {
        let fg = Color::Rgb {
            r: 228,
            g: 232,
            b: 246,
        };
        let fg_accent = Color::Rgb {
            r: 92,
            g: 208,
            b: 232,
        };
        let fg_boardframe = Color::Rgb {
            r: 238,
            g: 242,
            b: 255,
        };
        let fg_grid = Color::Rgb {
            r: 218,
            g: 224,
            b: 244,
        };
        let bg = Color::Rgb {
            r: 30,
            g: 30,
            b: 48,
        };
        let bg2 = Color::Rgb {
            r: 26,
            g: 26,
            b: 46,
        };
        let bg_board = Color::Rgb {
            r: 24,
            g: 24,
            b: 42,
        };
        TuiColoring {
            fg_tui: fg,
            fg_accent,
            fg_widgetframe: fg,
            fg_boardframe,
            fg_grid,

            bg_tui: bg,
            bg_widget: bg2,
            bg_boardframe: bg2,
            bg_board,
        }
    }

    pub fn gruvbox_dark() -> Self {
        let fg = Color::Rgb {
            r: 235,
            g: 219,
            b: 178,
        };
        let fg2 = Color::Rgb {
            r: 251,
            g: 241,
            b: 199,
        };
        let bg = Color::Rgb {
            r: 40,
            g: 40,
            b: 40,
        };
        // let bg2 = Color::Rgb {
        //     r: 30,
        //     g: 33,
        //     b: 41,
        // };
        TuiColoring {
            fg_tui: fg,
            fg_accent: fg2,
            fg_widgetframe: fg,
            fg_boardframe: fg2,
            fg_grid: fg,

            bg_tui: bg,
            bg_widget: bg,
            bg_boardframe: bg,
            bg_board: bg,
        }
    }

    pub fn onehalfdark() -> Self {
        let fg_accent = Color::Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        let fg = Color::Rgb {
            r: 220,
            g: 223,
            b: 228,
        };
        let bg = Color::Rgb {
            r: 40,
            g: 44,
            b: 52,
        };
        let bg2 = Color::Rgb {
            r: 35,
            g: 38,
            b: 47,
        };
        let bg_board = Color::Rgb {
            r: 30,
            g: 33,
            b: 41,
        };
        TuiColoring {
            fg_tui: fg,
            fg_accent,
            fg_widgetframe: fg,
            fg_boardframe: fg,
            fg_grid: fg,

            bg_tui: bg2,
            bg_widget: bg,
            bg_boardframe: bg,
            bg_board,
        }
    }

    pub fn white_on_black() -> Self {
        // let fg = Color::White;
        // let bg = Color::Black;
        let fg = Color::Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        let bg = Color::Rgb { r: 0, g: 0, b: 0 };
        TuiColoring {
            fg_tui: fg,
            fg_accent: fg,
            fg_widgetframe: fg,
            fg_boardframe: fg,
            fg_grid: fg,

            bg_tui: bg,
            bg_widget: bg,
            bg_boardframe: bg,
            bg_board: bg,
        }
    }

    pub fn solarized_light() -> Self {
        let base01 = Color::Rgb {
            r: 88,
            g: 110,
            b: 117,
        };
        // let base1 = Color::Rgb {
        //     r: 147,
        //     g: 161,
        //     b: 161,
        // };
        let base2 = Color::Rgb {
            r: 238,
            g: 232,
            b: 213,
        };
        let base3 = Color::Rgb {
            r: 253,
            g: 246,
            b: 227,
        };
        TuiColoring {
            fg_tui: base01,
            fg_accent: base01,
            fg_widgetframe: base01,
            fg_boardframe: base01,
            fg_grid: base01,

            bg_tui: base3,
            bg_widget: base2,
            bg_boardframe: base2,
            bg_board: base2,
        }
    }

    pub fn matrix() -> Self {
        let fg = Color::Rgb {
            r: 193,
            g: 255,
            b: 138,
        };
        let bg = Color::Rgb {
            r: 15,
            g: 25,
            b: 28,
        };
        let bg_widget = Color::Rgb {
            r: 16,
            g: 26,
            b: 29,
        };
        let bg_boardframe = Color::Rgb {
            r: 17,
            g: 27,
            b: 30,
        };
        let bg_board = Color::Rgb {
            r: 20,
            g: 28,
            b: 31,
        };
        TuiColoring {
            fg_tui: fg,
            fg_accent: fg,
            fg_widgetframe: fg,
            fg_boardframe: fg,
            fg_grid: fg,

            bg_tui: bg,
            bg_widget,
            bg_boardframe,
            bg_board,
        }
    }

    pub fn sequoia() -> Self {
        let fg = Color::Rgb {
            r: 226,
            g: 228,
            b: 237,
        };
        let bg = Color::Rgb {
            r: 15,
            g: 16,
            b: 20,
        };
        let bg_widget = Color::Rgb {
            r: 14,
            g: 15,
            b: 19,
        };
        let bg_boardframe = Color::Rgb {
            r: 13,
            g: 14,
            b: 18,
        };
        let bg_board = Color::Rgb {
            r: 12,
            g: 13,
            b: 17,
        };
        TuiColoring {
            fg_tui: fg,
            fg_accent: fg,
            fg_widgetframe: fg,
            fg_boardframe: fg,
            fg_grid: fg,

            bg_tui: bg,
            bg_widget,
            bg_boardframe,
            bg_board,
        }
    }

    pub fn amber() -> Self {
        let fg = Color::Rgb {
            r: 255,
            g: 148,
            b: 0,
        };
        let bg = Color::Rgb { r: 37, g: 18, b: 0 };
        TuiColoring {
            fg_tui: fg,
            fg_accent: fg,
            fg_widgetframe: fg,
            fg_boardframe: fg,
            fg_grid: fg,

            bg_tui: bg,
            bg_widget: bg,
            bg_boardframe: bg,
            bg_board: bg,
        }
    }

    pub fn nes() -> Self {
        let that_blue = NES_PALETTE[0x31];
        // let red = NES_PALETTE[0x16];
        TuiColoring {
            fg_tui: NES_WHITE,
            fg_accent: that_blue,
            fg_widgetframe: that_blue,
            fg_boardframe: that_blue,
            fg_grid: NES_GRAY,

            bg_tui: NES_GRAY,
            bg_widget: NES_BLACK,
            bg_boardframe: NES_BLACK,
            bg_board: NES_BLACK,
        }
    }

    // pub fn debug() -> Self {
    //     TuiColoring {
    //         fg_tui: Color::White,
    //         fg_accent: Color::Yellow,
    //         fg_widgetframe: Color::Green,
    //         fg_boardframe: Color::Red,
    //         fg_grid: Color::Cyan,
    //
    //         bg_tui: Color::Black,
    //         bg_widget: Color::Grey,
    //         bg_boardframe: Color::DarkGrey,
    //         bg_board: Color::Blue,
    //     }
    // }
}
