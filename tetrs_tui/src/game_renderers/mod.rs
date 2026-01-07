pub mod cached_renderer;
pub mod debug_renderer;

use std::io::{self, Write};

use crossterm::style::Color;
use tetrs_engine::{FeedbackMessages, Game, Tetromino, TileTypeID};

use crate::terminal_user_interface::{Application, GraphicsColoring, RunningGameStats};

pub trait Renderer {
    fn render<T>(
        &mut self,
        app: &mut Application<T>,
        running_game_stats: &mut RunningGameStats,
        game: &Game,
        new_feedback_events: FeedbackMessages,
        screen_resized: bool,
    ) -> io::Result<()>
    where
        T: Write;
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

pub fn tile_to_color(mode: GraphicsColoring) -> fn(TileTypeID) -> Option<Color> {
    match mode {
        GraphicsColoring::Monochrome => |_tile: TileTypeID| None,
        GraphicsColoring::Color16 => |tile: TileTypeID| {
            Some(match tile.get() {
                1 => Color::Yellow,
                2 => Color::DarkCyan,
                3 => Color::Green,
                4 => Color::DarkRed,
                5 => Color::DarkMagenta,
                6 => Color::Red,
                7 => Color::Blue,
                253 => Color::Black,
                254 => Color::DarkGrey,
                255 => Color::White,
                t => unimplemented!("formatting unknown tile id {t}"),
            })
        },
        GraphicsColoring::Fullcolor => |tile: TileTypeID| {
            Some(match tile.get() {
                1 => Color::Rgb {
                    r: 254,
                    g: 203,
                    b: 0,
                },
                2 => Color::Rgb {
                    r: 0,
                    g: 159,
                    b: 218,
                },
                3 => Color::Rgb {
                    r: 105,
                    g: 190,
                    b: 40,
                },
                4 => Color::Rgb {
                    r: 237,
                    g: 41,
                    b: 57,
                },
                5 => Color::Rgb {
                    r: 149,
                    g: 45,
                    b: 152,
                },
                6 => Color::Rgb {
                    r: 255,
                    g: 121,
                    b: 0,
                },
                7 => Color::Rgb {
                    r: 0,
                    g: 101,
                    b: 189,
                },
                253 => Color::Rgb { r: 0, g: 0, b: 0 },
                254 => Color::Rgb {
                    r: 127,
                    g: 127,
                    b: 127,
                },
                255 => Color::Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                },
                t => unimplemented!("formatting unknown tile id {t}"),
            })
        },
        GraphicsColoring::Experimental => |tile: TileTypeID| {
            Some(match tile.get() {
                1 => Color::Rgb {
                    r: 14,
                    g: 198,
                    b: 244,
                },
                2 => Color::Rgb {
                    r: 242,
                    g: 192,
                    b: 29,
                },
                3 => Color::Rgb {
                    r: 70,
                    g: 201,
                    b: 50,
                },
                4 => Color::Rgb {
                    r: 230,
                    g: 53,
                    b: 197,
                },
                5 => Color::Rgb {
                    r: 147,
                    g: 41,
                    b: 229,
                },
                6 => Color::Rgb {
                    r: 36,
                    g: 118,
                    b: 242,
                },
                7 => Color::Rgb {
                    r: 244,
                    g: 50,
                    b: 48,
                },
                253 => Color::Rgb { r: 0, g: 0, b: 0 },
                254 => Color::Rgb {
                    r: 127,
                    g: 127,
                    b: 127,
                },
                255 => Color::Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                },
                t => unimplemented!("formatting unknown tile id {t}"),
            })
        },
    }
}
