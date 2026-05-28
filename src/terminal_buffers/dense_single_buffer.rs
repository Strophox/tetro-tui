use std::{
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

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Default)]
pub struct DenseSingleBuffer {
    /// INVARIANT:
    /// - `buf.len() == width * height`.
    buf: Vec<TermCell>,
    ambience: TermCell,
    x_vp: u16,
    y_vp: u16,
    w_vp: u16,
    h_vp: u16,
}

impl TerminalBuffer for DenseSingleBuffer {
    // fn with_offset_and_area((x, y): (u16, u16), (w, h): (u16, u16)) -> Self {
    //     DenseTerminalSingleBuffer {
    //         buf: vec![TermCell::EMPTY; (w as usize * h as usize)],
    //         x_vp: x,
    //         y_vp: y,
    //         w_vp: w,
    //         h_vp: h,
    //     }
    // }
    fn reset_buffer(&mut self) {
        self.buf.fill(self.ambience);
    }

    fn reset_with_ambience(&mut self, cell: TermCell) {
        self.ambience = cell;
        self.reset_buffer();
    }

    fn reset_with_offset_and_area(&mut self, (x, y): (u16, u16), (w, h): (u16, u16)) {
        let old_len = (self.w_vp * self.h_vp).into();
        let new_len = w as usize * h as usize;

        self.x_vp = x;
        self.y_vp = y;
        self.w_vp = w;
        self.h_vp = h;

        // Reset current buffer.
        // We think it is faster to first fill and resize with rest if the buffer is expected to grow,
        // and faster to resize and then fill if the buffer will shrink.
        if new_len > old_len {
            self.buf.fill(self.ambience);
            self.buf.resize(new_len, self.ambience);
        } else {
            self.buf.resize(new_len, self.ambience);
            self.buf.fill(self.ambience);
        }
    }

    fn offset_and_area(&self) -> ((u16, u16), (u16, u16)) {
        ((self.x_vp, self.y_vp), (self.w_vp, self.h_vp))
    }

    fn write_char(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Option<Color>) {
        if x < self.w_vp && y < self.h_vp {
            let idx = x as usize + self.w_vp as usize * y as usize;
            self.buf[idx] = TermCell {
                ch,
                fg,
                bg: bg.unwrap_or_else(|| self.buf[idx].bg),
            };
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
        let idx = x as usize + self.w_vp as usize * y as usize;
        self.buf[idx] = TermCell {
            ch: ch0,
            fg,
            bg: bg.unwrap_or_else(|| self.buf[idx].bg),
        };

        if x + 1 >= self.w_vp {
            return;
        }
        self.buf[idx + 1] = TermCell {
            ch: ch1,
            fg,
            bg: bg.unwrap_or_else(|| self.buf[idx + 1].bg),
        };
    }

    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color, bg: Option<Color>) {
        if y >= self.h_vp {
            return;
        }
        for (dx, ch) in str.chars().enumerate() {
            if x + dx as u16 >= self.w_vp {
                return;
            }
            let idx = x as usize + dx + self.w_vp as usize * y as usize;
            self.buf[idx] = TermCell {
                ch,
                fg,
                bg: bg.unwrap_or_else(|| self.buf[idx].bg),
            };
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
            let idx = (x as usize + dx) + (self.w_vp as usize) * (y as usize + dy);
            self.buf[idx] = TermCell {
                ch,
                fg,
                bg: bg.unwrap_or_else(|| self.buf[idx].bg),
            };
            dx += 1;
        }
    }

    fn flush<W: Write>(&mut self, term: &mut W) -> io::Result<()> {
        term.queue(terminal::BeginSynchronizedUpdate)?;
        for x in 0..self.w_vp {
            for y in 0..self.h_vp {
                let idx = x as usize + self.w_vp as usize * y as usize;
                #[rustfmt::skip] let TermCell { ch, fg, bg } = self.buf[idx];
                term.queue(cursor::MoveTo(self.x_vp + x, self.y_vp + y))?;
                term.queue(PrintStyledContent(ch.with(fg).on(bg)))?;
            }
        }
        term.queue(cursor::MoveTo(0, 0))?
            .queue(terminal::EndSynchronizedUpdate)?
            .flush()?;

        // Reset buffer.
        self.reset_buffer();

        Ok(())
    }
}
