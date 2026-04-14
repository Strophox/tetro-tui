use std::{
    fmt::Debug,
    io::{self, Write},
};

use crossterm::{
    cursor,
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal, QueueableCommand,
};

use crate::tui_settings::TileTexture;

use super::{TermCell, TerminalBuffer};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Default)]
pub struct DenseTerminalDoubleBuffer {
    /// INVARIANT:
    /// - `prev_buf.len() == width * height`.
    prev_buf: Vec<TermCell>,
    /// INVARIANT:
    /// - `next_buf.len() == width * height`.
    next_buf: Vec<TermCell>,
    x_vp: u16,
    y_vp: u16,
    w_vp: u16,
    h_vp: u16,
}

impl TerminalBuffer for DenseTerminalDoubleBuffer {
    // fn with_offset_and_area((x, y): (u16, u16), (w, h): (u16, u16)) -> Self {
    //     DenseTerminalDoubleBuffer {
    //         prev_buf: vec![TermCell::EMPTY; (w * h).into()],
    //         next_buf: vec![TermCell::EMPTY; (w * h).into()],
    //         x_vp: x,
    //         y_vp: y,
    //         w_vp: w,
    //         h_vp: h,
    //     }
    // }

    fn offset_and_area(&self) -> ((u16, u16), (u16, u16)) {
        ((self.x_vp, self.y_vp), (self.w_vp, self.h_vp))
    }

    fn reset_with_offset_and_area(&mut self, (x, y): (u16, u16), (w, h): (u16, u16)) {
        let old_len = (self.w_vp * self.h_vp).into();
        let new_len = (w * h).into();
        if new_len > old_len {
            self.prev_buf.fill(TermCell::EMPTY);
            self.next_buf.fill(TermCell::EMPTY);
            self.prev_buf.resize(new_len, TermCell::EMPTY);
            self.next_buf.resize(new_len, TermCell::EMPTY);
        } else {
            self.prev_buf.resize(new_len, TermCell::EMPTY);
            self.next_buf.resize(new_len, TermCell::EMPTY);
            self.prev_buf.fill(TermCell::EMPTY);
            self.next_buf.fill(TermCell::EMPTY);
        }
        self.x_vp = x;
        self.y_vp = y;
        self.w_vp = w;
        self.h_vp = h;
    }

    fn write_char(&mut self, x: u16, y: u16, cell: TermCell) {
        if x < self.w_vp && y < self.h_vp {
            let idx = x as usize + self.w_vp as usize * y as usize;
            self.next_buf[idx] = cell;
        }
    }

    fn write_tile(&mut self, x: u16, y: u16, tile: TileTexture, fg: Color) {
        if y >= self.h_vp {
            return;
        }
        let [ch0, ch1] = tile.0;
        if x >= self.w_vp {
            return;
        }
        let idx = x as usize + self.w_vp as usize * y as usize;
        self.next_buf[idx] = TermCell { ch: ch0, fg };

        if x + 1 >= self.w_vp {
            return;
        }
        self.next_buf[idx + 1] = TermCell { ch: ch1, fg };
    }

    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color) {
        if y >= self.h_vp {
            return;
        }
        for (dx, ch) in str.chars().enumerate() {
            if x + dx as u16 >= self.w_vp {
                return;
            }
            let idx = x as usize + dx + self.w_vp as usize * y as usize;
            self.next_buf[idx] = TermCell { ch, fg };
        }
    }

    fn flush(&mut self, term: &mut impl Write) -> io::Result<()> {
        term.queue(terminal::BeginSynchronizedUpdate)?;

        for x in 0..self.w_vp {
            for y in 0..self.h_vp {
                let idx = x as usize + self.w_vp as usize * y as usize;
                #[rustfmt::skip] let TermCell { ch: old_ch, fg: old_fg } = self.prev_buf[idx];
                #[rustfmt::skip] let TermCell { ch: new_ch, fg: new_fg } = self.next_buf[idx];

                term.queue(cursor::MoveTo(self.x_vp + x, self.y_vp + y))?;
                if new_fg != old_fg || new_ch != old_ch {
                    // Always reprint styled if style changed.
                    term.queue(PrintStyledContent(new_ch.with(new_fg)))?;
                }
            }
        }

        term.queue(cursor::MoveTo(0, 0))?
            .queue(terminal::EndSynchronizedUpdate)?
            .flush()?;

        // Swap buffers so `prev_buf` correctly contains the one we just wrote and want to keep for next time.
        std::mem::swap(&mut self.prev_buf, &mut self.next_buf);

        // Reset buffer by overwriting nonempty cells.
        self.next_buf.fill(TermCell::EMPTY);

        Ok(())
    }
}
