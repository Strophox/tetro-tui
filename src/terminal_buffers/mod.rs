mod dense_double_buffer;
mod dense_single_buffer;
mod sparse_double_buffer;
mod sparse_single_buffer;

use std::io::{self, Write};

use crossterm::style::Color;

pub use dense_double_buffer::DenseDoubleBuffer;
#[allow(unused)]
pub use dense_single_buffer::DenseSingleBuffer;
#[allow(unused)]
pub use sparse_double_buffer::SparseDoubleBuffer;
#[allow(unused)]
pub use sparse_single_buffer::SparseSingleBuffer;

use crate::settings::TileTexture;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct TermCell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    // pub attr: Attr,
}

impl TermCell {
    pub const BLANK: TermCell = TermCell {
        ch: ' ',
        fg: Color::Reset,
        bg: Color::Reset,
        // attr: Attr::empty(),
    };
}

impl Default for TermCell {
    fn default() -> Self {
        TermCell::BLANK
    }
}

pub trait TerminalBuffer {
    // fn with_offset_and_area(offsets: (u16, u16), dimensions: (u16, u16)) -> Self;

    /// Resets the current buffer contents.
    fn reset_buffer(&mut self);
    /// Resets the buffer contents using a new background ('ambience').
    fn reset_with_ambience(&mut self, cell: TermCell);
    /// This resets the buffer using a new offset and area of the buffer.
    /// This should also correctly handle the-redrawing of the terminal on the next `flush`, which (in the case of double-/diff-/cached buffers may mean that the whole screen is redrawn instead of diff'ed.)
    fn reset_with_offset_and_area(&mut self, offsets: (u16, u16), dimensions: (u16, u16));
    /// Retrives the current offset and area.
    fn offset_and_area(&self) -> ((u16, u16), (u16, u16));

    fn write_char(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Option<Color>);
    fn write_tile(&mut self, x: u16, y: u16, tile: TileTexture, fg: Color, bg: Option<Color>);
    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color, bg: Option<Color>);
    fn write_str_wrapping(&mut self, x: u16, y: u16, str: &str, fg: Color, bg: Option<Color>);
    fn flush<W: Write>(&mut self, term: &mut W) -> io::Result<()>;
}

#[allow(unused)]
#[derive(PartialEq, Clone, Debug)]
pub enum MiscTermBufs {
    DenseDouble(DenseDoubleBuffer),
    SparseDouble(SparseDoubleBuffer),
    SparseSingle(SparseSingleBuffer),
}

impl TerminalBuffer for MiscTermBufs {
    fn reset_buffer(&mut self) {
        match self {
            MiscTermBufs::DenseDouble(this) => this.reset_buffer(),
            MiscTermBufs::SparseDouble(this) => this.reset_buffer(),
            MiscTermBufs::SparseSingle(this) => this.reset_buffer(),
        }
    }

    fn reset_with_ambience(&mut self, cell: TermCell) {
        match self {
            MiscTermBufs::DenseDouble(this) => this.reset_with_ambience(cell),
            MiscTermBufs::SparseDouble(this) => this.reset_with_ambience(cell),
            MiscTermBufs::SparseSingle(this) => this.reset_with_ambience(cell),
        }
    }

    fn reset_with_offset_and_area(&mut self, offsets: (u16, u16), dimensions: (u16, u16)) {
        match self {
            MiscTermBufs::DenseDouble(this) => this.reset_with_offset_and_area(offsets, dimensions),
            MiscTermBufs::SparseDouble(this) => {
                this.reset_with_offset_and_area(offsets, dimensions)
            }
            MiscTermBufs::SparseSingle(this) => {
                this.reset_with_offset_and_area(offsets, dimensions)
            }
        }
    }

    fn offset_and_area(&self) -> ((u16, u16), (u16, u16)) {
        match self {
            MiscTermBufs::DenseDouble(this) => this.offset_and_area(),
            MiscTermBufs::SparseDouble(this) => this.offset_and_area(),
            MiscTermBufs::SparseSingle(this) => this.offset_and_area(),
        }
    }

    fn write_char(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Option<Color>) {
        match self {
            MiscTermBufs::DenseDouble(this) => this.write_char(x, y, ch, fg, bg),
            MiscTermBufs::SparseDouble(this) => this.write_char(x, y, ch, fg, bg),
            MiscTermBufs::SparseSingle(this) => this.write_char(x, y, ch, fg, bg),
        }
    }

    fn write_tile(&mut self, x: u16, y: u16, tile: TileTexture, fg: Color, bg: Option<Color>) {
        match self {
            MiscTermBufs::DenseDouble(this) => this.write_tile(x, y, tile, fg, bg),
            MiscTermBufs::SparseDouble(this) => this.write_tile(x, y, tile, fg, bg),
            MiscTermBufs::SparseSingle(this) => this.write_tile(x, y, tile, fg, bg),
        }
    }

    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color, bg: Option<Color>) {
        match self {
            MiscTermBufs::DenseDouble(this) => this.write_str(x, y, str, fg, bg),
            MiscTermBufs::SparseDouble(this) => this.write_str(x, y, str, fg, bg),
            MiscTermBufs::SparseSingle(this) => this.write_str(x, y, str, fg, bg),
        }
    }

    fn write_str_wrapping(&mut self, x: u16, y: u16, str: &str, fg: Color, bg: Option<Color>) {
        match self {
            MiscTermBufs::DenseDouble(this) => this.write_str_wrapping(x, y, str, fg, bg),
            MiscTermBufs::SparseDouble(this) => this.write_str_wrapping(x, y, str, fg, bg),
            MiscTermBufs::SparseSingle(this) => this.write_str_wrapping(x, y, str, fg, bg),
        }
    }

    fn flush<W: Write>(&mut self, term: &mut W) -> io::Result<()> {
        match self {
            MiscTermBufs::DenseDouble(this) => this.flush(term),
            MiscTermBufs::SparseDouble(this) => this.flush(term),
            MiscTermBufs::SparseSingle(this) => this.flush(term),
        }
    }
}
