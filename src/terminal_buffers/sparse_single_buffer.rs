use std::{
    collections::HashMap,
    fmt::Debug,
    io::{self, Write},
};

use crossterm::{
    QueueableCommand, cursor,
    style::{Color, PrintStyledContent, Stylize},
    terminal,
};

use crate::settings::TileTexture;

use super::{TermCell, TerminalBuffer};

#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub struct SparseSingleBuffer {
    /// INVARIANT:
    /// - `buf.len() == width * height`.
    buf: HashMap<(u16, u16), TermCell>,
    ambience: TermCell,
    x_vp: u16,
    y_vp: u16,
    w_vp: u16,
    h_vp: u16,
}

impl TerminalBuffer for SparseSingleBuffer {
    // fn with_offset_and_area((x, y): (u16, u16), (w, h): (u16, u16)) -> Self {
    //     DenseTerminalSingleBuffer {
    //         buf: vec![TermCell::EMPTY; (w * h).into()],
    //         x_vp: x,
    //         y_vp: y,
    //         w_vp: w,
    //         h_vp: h,
    //     }
    // }

    fn offset_and_area(&self) -> ((u16, u16), (u16, u16)) {
        ((self.x_vp, self.y_vp), (self.w_vp, self.h_vp))
    }

    fn reset_buffer(&mut self) {
        self.buf.clear();
    }

    fn reset_with_ambience(&mut self, cell: TermCell) {
        self.ambience = cell;
        self.reset_buffer();
    }

    fn reset_with_offset_and_area(&mut self, (x, y): (u16, u16), (w, h): (u16, u16)) {
        self.x_vp = x;
        self.y_vp = y;
        self.w_vp = w;
        self.h_vp = h;

        self.reset_buffer();
    }

    fn write_char(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Option<Color>) {
        if x < self.w_vp && y < self.h_vp {
            let idx = (x, y);
            let cell = TermCell {
                ch,
                fg,
                bg: bg.unwrap_or_else(|| self.buf.get(&idx).unwrap_or(&self.ambience).bg),
            };
            self.buf.insert(idx, cell);
        }
    }

    fn write_tile(&mut self, x: u16, y: u16, tile: TileTexture, fg: Color, bg: Option<Color>) {
        if y >= self.h_vp {
            return;
        }
        let [ch0, ch1] = tile.0;
        if x >= self.w_vp {
            return;
        }
        let idx = (x, y);
        let cell = TermCell {
            ch: ch0,
            fg,
            bg: bg.unwrap_or_else(|| self.buf.get(&idx).unwrap_or(&self.ambience).bg),
        };
        self.buf.insert(idx, cell);

        if x + 1 >= self.w_vp {
            return;
        }
        let idx = (x + 1, y);
        let cell = TermCell {
            ch: ch1,
            fg,
            bg: bg.unwrap_or_else(|| self.buf.get(&idx).unwrap_or(&self.ambience).bg),
        };
        self.buf.insert(idx, cell);
    }

    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color, bg: Option<Color>) {
        if y >= self.h_vp {
            return;
        }
        for (dx, ch) in str.chars().enumerate() {
            if x + dx as u16 >= self.w_vp {
                return;
            }
            let idx = (x + dx as u16, y);
            let cell = TermCell {
                ch,
                fg,
                bg: bg.unwrap_or_else(|| self.buf.get(&idx).unwrap_or(&self.ambience).bg),
            };
            self.buf.insert(idx, cell);
        }
    }

    fn write_str_wrapping(&mut self, x: u16, y: u16, str: &str, fg: Color, bg: Option<Color>) {
        let mut dx = 0;
        let mut dy = 0;
        for ch in str.chars() {
            if x + dx as u16 >= self.w_vp {
                dx = 0;
                dy += 1;
            }
            if y + dy as u16 >= self.h_vp {
                return;
            }
            let idx = (x + dx as u16, y + dy as u16);
            let cell = TermCell {
                ch,
                fg,
                bg: bg.unwrap_or_else(|| self.buf.get(&idx).unwrap_or(&self.ambience).bg),
            };
            self.buf.insert(idx, cell);
            dx += 1;
        }
    }

    fn flush<W: Write>(&mut self, term: &mut W) -> io::Result<()> {
        term.queue(terminal::BeginSynchronizedUpdate)?;

        for x in 0..self.w_vp {
            for y in 0..self.h_vp {
                term.queue(cursor::MoveTo(self.x_vp + x, self.y_vp + y))?;
                if let Some(TermCell { ch, fg, bg }) = self.buf.get(&(x, y)).copied() {
                    term.queue(PrintStyledContent(ch.with(fg).on(bg)))?;
                } else {
                    #[rustfmt::skip] let TermCell { ch, fg, bg } = self.ambience;
                    term.queue(PrintStyledContent(ch.with(fg).on(bg)))?;
                }
            }
        }

        term.queue(cursor::MoveTo(0, 0))?
            .queue(terminal::EndSynchronizedUpdate)?
            .flush()?;

        Ok(())
    }
}
