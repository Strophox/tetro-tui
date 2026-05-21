use std::{
    fmt::Debug,
    io::{self, Write},
};

use crossterm::{
    QueueableCommand, cursor,
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal,
};

use crate::settings::TileTexture;

use super::{TermCell, TerminalBuffer};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Default)]
pub struct DenseDoubleBuffer {
    /// INVARIANT:
    /// - `prev_buf.len() == width * height`.
    prev_buf: Option<Vec<TermCell>>,
    /// INVARIANT:
    /// - `next_buf.len() == width * height`.
    curr_buf: Vec<TermCell>,
    curr_ambience: TermCell,
    x_vp: u16,
    y_vp: u16,
    w_vp: u16,
    h_vp: u16,
}

impl TerminalBuffer for DenseDoubleBuffer {
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
    fn reset_buffer(&mut self) {
        self.curr_buf.fill(self.curr_ambience);
    }

    fn reset_with_ambience(&mut self, cell: TermCell) {
        self.curr_ambience = cell;
        self.reset_buffer();
    }

    fn reset_with_offset_and_area(&mut self, (x, y): (u16, u16), (w, h): (u16, u16)) {
        let old_len = (self.w_vp * self.h_vp).into();
        let new_len = (w * h).into();

        self.x_vp = x;
        self.y_vp = y;
        self.w_vp = w;
        self.h_vp = h;

        // Reset current buffer.
        // We think it is faster to first fill and resize with rest if the buffer is expected to grow,
        // and faster to resize and then fill if the buffer will shrink.
        if new_len > old_len {
            self.curr_buf.fill(self.curr_ambience);
            self.curr_buf.resize(new_len, self.curr_ambience);
        } else {
            self.curr_buf.resize(new_len, self.curr_ambience);
            self.curr_buf.fill(self.curr_ambience);
        }

        // Explicitly redraw whole screen next time.
        self.prev_buf = None;
    }

    fn offset_and_area(&self) -> ((u16, u16), (u16, u16)) {
        ((self.x_vp, self.y_vp), (self.w_vp, self.h_vp))
    }

    fn write_char(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Option<Color>) {
        if x < self.w_vp && y < self.h_vp {
            let idx = x as usize + self.w_vp as usize * y as usize;
            self.curr_buf[idx] = TermCell {
                ch,
                fg,
                bg: bg.unwrap_or_else(|| self.curr_buf[idx].bg),
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
        self.curr_buf[idx] = TermCell {
            ch: ch0,
            fg,
            bg: bg.unwrap_or_else(|| self.curr_buf[idx].bg),
        };

        if x + 1 >= self.w_vp {
            return;
        }
        self.curr_buf[idx + 1] = TermCell {
            ch: ch1,
            fg,
            bg: bg.unwrap_or_else(|| self.curr_buf[idx + 1].bg),
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
            self.curr_buf[idx] = TermCell {
                ch,
                fg,
                bg: bg.unwrap_or_else(|| self.curr_buf[idx].bg),
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
            self.curr_buf[idx] = TermCell {
                ch,
                fg,
                bg: bg.unwrap_or_else(|| self.curr_buf[idx].bg),
            };
            dx += 1;
        }
    }

    fn flush<W: Write>(&mut self, term: &mut W) -> io::Result<()> {
        if let Some(prev_buf) = &mut self.prev_buf {
            // Use flag to possibly avoid having to do any I/O at all.
            let mut diff_issued = false;
            for x in 0..self.w_vp {
                for y in 0..self.h_vp {
                    let idx = x as usize + self.w_vp as usize * y as usize;
                    // Always reprint styled if anything changed.
                    if prev_buf[idx] != self.curr_buf[idx] {
                        if !diff_issued {
                            diff_issued = true;
                            term.queue(terminal::BeginSynchronizedUpdate)?;
                        }
                        #[rustfmt::skip] let TermCell { ch: _old_ch, fg: old_fg, bg: old_bg } = prev_buf[idx];
                        #[rustfmt::skip] let TermCell { ch: new_ch, fg: new_fg, bg: new_bg } = self.curr_buf[idx];
                        term.queue(cursor::MoveTo(self.x_vp + x, self.y_vp + y))?;
                        // FIXME: Check if this optimization is correct: Currently we treat 'Print' or no BG color as crossterm automatically resetting it.
                        // FIXME: Propagate this optimization to other buffers if it is correct.
                        if (new_fg, new_bg) == (old_fg, old_bg)
                            && (new_fg, new_bg) == (Color::Reset, Color::Reset)
                        {
                            term.queue(Print(new_ch))?;
                        } else if new_bg == old_bg && new_bg == Color::Reset {
                            term.queue(PrintStyledContent(new_ch.with(new_fg)))?;
                        } else if new_fg == old_fg && new_fg == Color::Reset {
                            term.queue(PrintStyledContent(new_ch.on(new_bg)))?;
                        } else {
                            term.queue(PrintStyledContent(new_ch.with(new_fg).on(new_bg)))?;
                        }
                    }
                }
            }
            if diff_issued {
                term.queue(cursor::MoveTo(0, 0))?
                    .queue(terminal::EndSynchronizedUpdate)?
                    .flush()?;
            }

            // Double buffer.
            std::mem::swap(prev_buf, &mut self.curr_buf);
            self.reset_buffer();
        } else {
            // In this case, explicitly draw entire buffer.
            term.queue(terminal::BeginSynchronizedUpdate)?;
            for x in 0..self.w_vp {
                for y in 0..self.h_vp {
                    let idx = x as usize + self.w_vp as usize * y as usize;
                    #[rustfmt::skip] let TermCell { ch: new_ch, fg: new_fg, bg: new_bg } = self.curr_buf[idx];
                    term.queue(cursor::MoveTo(self.x_vp + x, self.y_vp + y))?;
                    term.queue(PrintStyledContent(new_ch.with(new_fg).on(new_bg)))?;
                }
            }
            term.queue(cursor::MoveTo(0, 0))?
                .queue(terminal::EndSynchronizedUpdate)?
                .flush()?;

            // Create new blank canvas for next_buf.
            let mut temp_buf = vec![self.curr_ambience; (self.w_vp * self.h_vp) as usize];
            // Swap buffers so `prev_buf` correctly contains the one we just wrote and want to keep for next time.
            std::mem::swap(&mut temp_buf, &mut self.curr_buf);
            // Store previous buffer.
            self.prev_buf = Some(temp_buf);
        }

        Ok(())
    }
}
