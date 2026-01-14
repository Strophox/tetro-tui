pub mod cached_renderer;
pub mod debug_renderer;

use std::{
    collections::HashMap,
    io::{self, Write},
};

use crossterm::style::Color;
use tetrs_engine::{Button, FeedbackMessages, Game, Tetromino};

use crate::terminal_user_interface::Application;

pub type Palette = HashMap<u8, Color>;

pub trait Renderer {
    fn render<T: Write>(
        &mut self,
        app: &mut Application<T>,
        game: &Game,
        new_feedback_events: FeedbackMessages,
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

pub fn experimental_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_EXPERIMENTAL: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r: 14,g:198,b:244}),
        (  2, Color::Rgb{r:242,g:192,b: 29}),
        (  3, Color::Rgb{r: 70,g:201,b: 50}),
        (  4, Color::Rgb{r:230,g: 53,b:197}),
        (  5, Color::Rgb{r:147,g: 41,b:229}),
        (  6, Color::Rgb{r: 36,g:118,b:242}),
        (  7, Color::Rgb{r:244,g: 50,b: 48}),
        (253, Color::Rgb{r:  0,g:  0,b:  0}),
        (254, Color::Rgb{r:127,g:127,b:127}),
        (255, Color::Rgb{r:255,g:255,b:255}),
    ];
    HashMap::from(COLORS_EXPERIMENTAL)
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
