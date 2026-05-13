mod dense_double_buffer;
mod dense_single_buffer;
mod sparse_double_buffer;

use std::io::{self, Write};

use crossterm::style::Color;
#[allow(unused)]
pub use dense_double_buffer::DenseDoubleBuffer;
#[allow(unused)]
pub use dense_single_buffer::DenseSingleBuffer;
#[allow(unused)]
pub use sparse_double_buffer::SparseDoubleBuffer;

use crate::settings::TileTexture;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct TermCell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

impl TermCell {
    const EMPTY: TermCell = TermCell {
        ch: ' ',
        fg: Color::Reset,
        bg: Color::Reset,
    };
}

pub trait TerminalBuffer {
    // fn with_offset_and_area(offsets: (u16, u16), dimensions: (u16, u16)) -> Self;
    fn offset_and_area(&self) -> ((u16, u16), (u16, u16));
    fn reset_with_offset_and_area(&mut self, offsets: (u16, u16), dimensions: (u16, u16));

    fn write_char(&mut self, x: u16, y: u16, cell: TermCell);
    fn write_tile(&mut self, x: u16, y: u16, tile: TileTexture, fg: Color, bg: Color);
    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color, bg: Color);
    fn write_str_wrapping(&mut self, x: u16, y: u16, str: &str, fg: Color, bg: Color);
    fn flush(&mut self, term: &mut impl Write) -> io::Result<()>;
}
