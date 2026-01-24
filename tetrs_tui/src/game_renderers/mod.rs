pub mod braille;
pub mod diff_print;
pub mod legacy_debug;

use std::{
    collections::HashMap,
    io::{self, Write},
};

use crossterm::style::Color;
use tetrs_engine::{Button, FeedbackMessages, Game, Tetromino};

use crate::application::{Application, GameMetaData};

pub type Palette = HashMap<u8, Color>;

pub trait Renderer {
    fn render<T: Write>(
        &mut self,
        app: &mut Application<T>,
        game: &Game,
        meta_data: &GameMetaData,
        new_msgs: FeedbackMessages,
        screen_resized: bool,
    ) -> io::Result<()>;
}

pub fn empty_palette() -> Palette {
    HashMap::new()
}

pub fn fullcolor_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_DEFAULT: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:254,g:203,b:  1}),
        (  2, Color::Rgb{r:  0,g:159,b:219}),
        (  3, Color::Rgb{r:105,g:190,b: 41}),
        (  4, Color::Rgb{r:237,g: 41,b: 58}),
        (  5, Color::Rgb{r:149,g: 45,b:153}),
        (  6, Color::Rgb{r:255,g:121,b:  1}),
        (  7, Color::Rgb{r:  0,g:101,b:190}),
        (253, Color::Rgb{r:  0,g:  0,b:  0}),
        (254, Color::Rgb{r:127,g:127,b:127}),
        (255, Color::Rgb{r:255,g:255,b:255}),
    ];
    HashMap::from(COLORS_DEFAULT)
}

pub fn color16_palette() -> Palette {
    const COLORS_COLOR16: [(u8, Color); 7 + 3] = [
        (1, Color::Yellow),
        (2, Color::DarkCyan),
        (3, Color::Green),
        (4, Color::DarkRed),
        (5, Color::DarkMagenta),
        (6, Color::Red),
        (7, Color::Blue),
        (253, Color::Black),
        (254, Color::DarkGrey),
        (255, Color::White),
    ];
    HashMap::from(COLORS_COLOR16)
}

// pub fn oklch1_palette() -> Palette {
//     #[rustfmt::skip]
//     const COLORS_OKLCH: [(u8, Color); 7 + 3] = [
//         (  1, Color::Rgb{r:234,g:173,b: 55}),
//         (  2, Color::Rgb{r:  0,g:188,b:184}),
//         (  3, Color::Rgb{r:110,g:183,b: 76}),
//         (  4, Color::Rgb{r:242,g:113,b:141}),
//         (  5, Color::Rgb{r:168,g:138,b:250}),
//         (  6, Color::Rgb{r:240,g:124,b: 67}),
//         (  7, Color::Rgb{r: 49,g:169,b:253}),
//         (253, Color::Rgb{r:  0,g:  0,b:  0}),
//         (254, Color::Rgb{r:127,g:127,b:127}),
//         (255, Color::Rgb{r:255,g:255,b:255}),
//     ];
//     HashMap::from(COLORS_OKLCH)
// }

pub fn oklch2_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_OKLCH_HUE_INC: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:239,g:175,b: 50}),
        (  2, Color::Rgb{r:  0,g:199,b:198}),
        (  3, Color::Rgb{r:108,g:189,b: 70}),
        (  4, Color::Rgb{r:255,g: 99,b:133}),
        (  5, Color::Rgb{r:164,g:130,b:255}),
        (  6, Color::Rgb{r:245,g:122,b: 62}),
        (  7, Color::Rgb{r: 49,g:159,b:253}),
        (253, Color::Rgb{r:  0,g:  0,b:  0}),
        (254, Color::Rgb{r:127,g:127,b:127}),
        (255, Color::Rgb{r:255,g:255,b:255}),
    ];
    HashMap::from(COLORS_OKLCH_HUE_INC)
}

pub fn gruvbox_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_GRUVBOX_NORMAL: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:215,g:153,b: 33}),
        (  2, Color::Rgb{r:104,g:157,b:106}),
        (  3, Color::Rgb{r:152,g:151,b: 26}),
        (  4, Color::Rgb{r:204,g: 36,b: 29}),
        (  5, Color::Rgb{r:177,g: 98,b:134}),
        (  6, Color::Rgb{r:214,g: 93,b: 14}),
        (  7, Color::Rgb{r: 69,g:133,b:136}),
        (253, Color::Rgb{r:  0,g:  0,b:  0}),
        (254, Color::Rgb{r:127,g:127,b:127}),
        (255, Color::Rgb{r:255,g:255,b:255}),
    ];
    HashMap::from(COLORS_GRUVBOX_NORMAL)
}

pub fn gruvbox_light_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_GRUVBOX_LIGHT: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:250,g:189,b: 47}),
        (  2, Color::Rgb{r:142,g:192,b:124}),
        (  3, Color::Rgb{r:184,g:187,b: 38}),
        (  4, Color::Rgb{r:251,g: 73,b: 52}),
        (  5, Color::Rgb{r:211,g:134,b:155}),
        (  6, Color::Rgb{r:254,g:128,b: 25}),
        (  7, Color::Rgb{r:131,g:165,b:152}),
        (253, Color::Rgb{r:  0,g:  0,b:  0}),
        (254, Color::Rgb{r:127,g:127,b:127}),
        (255, Color::Rgb{r:255,g:255,b:255}),
    ];
    HashMap::from(COLORS_GRUVBOX_LIGHT)
}

pub fn the_matrix_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_THE_MATRIX: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:233,g:226,b:  0}),
        (  2, Color::Rgb{r: 80,g:180,b: 90}),
        (  3, Color::Rgb{r:144,g:215,b: 98}),
        (  4, Color::Rgb{r: 35,g:117,b: 90}),
        (  5, Color::Rgb{r: 64,g:153,b: 49}),
        (  6, Color::Rgb{r: 47,g:192,b:121}),
        (  7, Color::Rgb{r: 79,g:126,b:126}),
        (253, Color::Rgb{r: 15,g: 25,b: 28}),
        (254, Color::Rgb{r:109,g:127,b:113}),
        (255, Color::Rgb{r:234,g:255,b:244}),
    ];
    HashMap::from(COLORS_THE_MATRIX)
}

pub fn tet_str_small(t: &Tetromino) -> &'static str {
    match t {
        Tetromino::O => "██",
        Tetromino::I => "▄▄▄▄",
        Tetromino::S => "▄█▀",
        Tetromino::Z => "▀█▄",
        Tetromino::T => "▄█▄",
        Tetromino::L => "▄▄█",
        Tetromino::J => "█▄▄",
    }
}

pub fn tet_str_minuscule(t: &Tetromino) -> &'static str {
    match t {
        Tetromino::O => "⠶", //"⠶",
        Tetromino::I => "⡇", //"⠤⠤",
        Tetromino::S => "⠳", //"⠴⠂",
        Tetromino::Z => "⠞", //"⠲⠄",
        Tetromino::T => "⠗", //"⠴⠄",
        Tetromino::L => "⠧", //"⠤⠆",
        Tetromino::J => "⠼", //"⠦⠄",
    }
}

pub fn button_str(b: &Button) -> &'static str {
    match b {
        Button::MoveLeft => "←",
        Button::MoveRight => "→",
        Button::RotateLeft => "↺",
        Button::RotateRight => "↻",
        Button::RotateAround => "↔",
        Button::DropSoft => "↓",
        Button::DropHard => "⤓",
        Button::DropSonic => "⇓",
        Button::HoldPiece => "h",
    }
}
