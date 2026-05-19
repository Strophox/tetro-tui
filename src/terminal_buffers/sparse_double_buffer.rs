use std::{
    cmp::Ordering,
    collections::BTreeMap,
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
pub struct SparseDoubleBuffer {
    #[allow(clippy::type_complexity)]
    prev_buf_and_ambience: Option<(BTreeMap<(u16, u16), TermCell>, TermCell)>,
    curr_buf: BTreeMap<(u16, u16), TermCell>,
    curr_ambience: TermCell,
    x_vp: u16,
    y_vp: u16,
    w_vp: u16,
    h_vp: u16,
}

impl TerminalBuffer for SparseDoubleBuffer {
    // fn with_offset_and_area((x, y): (u16, u16), (w, h): (u16, u16)) -> Self {
    //     SparseTerminalBuffer {
    //         prev_buf: BTreeMap::new(),
    //         next_buf: BTreeMap::new(),
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
        self.curr_buf.clear();
    }

    fn reset_with_ambience(&mut self, cell: TermCell) {
        self.curr_ambience = cell;
        self.reset_buffer();
    }

    fn reset_with_offset_and_area(&mut self, (x, y): (u16, u16), (w, h): (u16, u16)) {
        self.x_vp = x;
        self.y_vp = y;
        self.w_vp = w;
        self.h_vp = h;

        self.reset_buffer();

        // Explicitly force redraw on next flush.
        self.prev_buf_and_ambience = None;
    }

    fn write_char(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Option<Color>) {
        if x < self.w_vp && y < self.h_vp {
            let idx = (x, y);
            let cell = TermCell {
                ch,
                fg,
                bg: bg.unwrap_or_else(|| self.curr_buf.get(&idx).unwrap_or(&self.curr_ambience).bg),
            };
            self.curr_buf.insert(idx, cell);
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
            bg: bg.unwrap_or_else(|| self.curr_buf.get(&idx).unwrap_or(&self.curr_ambience).bg),
        };
        self.curr_buf.insert(idx, cell);

        if x + 1 >= self.w_vp {
            return;
        }
        let idx = (x + 1, y);
        let cell = TermCell {
            ch: ch1,
            fg,
            bg: bg.unwrap_or_else(|| self.curr_buf.get(&idx).unwrap_or(&self.curr_ambience).bg),
        };
        self.curr_buf.insert(idx, cell);
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
                bg: bg.unwrap_or_else(|| self.curr_buf.get(&idx).unwrap_or(&self.curr_ambience).bg),
            };
            self.curr_buf.insert(idx, cell);
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
                bg: bg.unwrap_or_else(|| self.curr_buf.get(&idx).unwrap_or(&self.curr_ambience).bg),
            };
            self.curr_buf.insert(idx, cell);
            dx += 1;
        }
    }

    fn flush<W: Write>(&mut self, term: &mut W) -> io::Result<()> {
        if let Some((prev_buf, prev_ambience)) = &mut self.prev_buf_and_ambience
            && self.curr_ambience == *prev_ambience
        {
            // Use flag to possibly avoid having to do any I/O at all.
            let mut diff_issued = false;

            // We'll be consuming both iterators and compare.
            let mut prev_buffer = prev_buf.iter();
            let mut curr_buffer = self.curr_buf.iter();

            let mut prev_pos_and_cell = prev_buffer.next();
            let mut curr_pos_and_cell = curr_buffer.next();
            loop {
                let ((x, y), TermCell { ch, fg, bg }) = match (prev_pos_and_cell, curr_pos_and_cell)
                {
                    // Both are empty, nothing to do.
                    (None, None) => break,

                    // Old buffer contains something the new one doesn't: Overwrite it to clear it.
                    (Some((prev_pos, _prev_cell)), None) => {
                        prev_pos_and_cell = prev_buffer.next();
                        (prev_pos, self.curr_ambience)
                    }

                    // New buffer contains something the old one doesn't: Write it.
                    (None, Some((curr_pos, curr_cell))) => {
                        curr_pos_and_cell = curr_buffer.next();
                        (curr_pos, *curr_cell)
                    }

                    (Some((prev_pos, prev_cell)), Some((curr_pos, curr_cell))) => {
                        match prev_pos.cmp(curr_pos) {
                            // Old buffer contains something the new one doesn't: Overwrite it to clear it.
                            Ordering::Less => {
                                prev_pos_and_cell = prev_buffer.next();
                                (prev_pos, self.curr_ambience)
                            }

                            // New buffer contains something the old one doesn't: Write it.
                            Ordering::Greater => {
                                curr_pos_and_cell = curr_buffer.next();
                                (curr_pos, *curr_cell)
                            }

                            // Old and new overlap! Handle possible difference.
                            Ordering::Equal => {
                                prev_pos_and_cell = prev_buffer.next();
                                curr_pos_and_cell = curr_buffer.next();
                                if curr_cell != prev_cell {
                                    (curr_pos, *curr_cell)
                                } else {
                                    continue;
                                }
                            }
                        }
                    }
                };

                if !diff_issued {
                    diff_issued = true;
                    term.queue(terminal::BeginSynchronizedUpdate)?;
                }
                term.queue(cursor::MoveTo(self.x_vp + x, self.y_vp + y))?;
                term.queue(PrintStyledContent(ch.with(fg).on(bg)))?;
            }

            term.queue(cursor::MoveTo(0, 0))?
                .queue(terminal::EndSynchronizedUpdate)?
                .flush()?;

            std::mem::swap(prev_buf, &mut self.curr_buf);
            std::mem::swap(prev_ambience, &mut self.curr_ambience);
            self.reset_buffer();
        } else {
            // Flush everything.
            term.queue(terminal::BeginSynchronizedUpdate)?;

            for x in 0..self.w_vp {
                for y in 0..self.h_vp {
                    term.queue(cursor::MoveTo(self.x_vp + x, self.y_vp + y))?;
                    if let Some(TermCell { ch, fg, bg }) = self.curr_buf.get(&(x, y)).copied() {
                        term.queue(PrintStyledContent(ch.with(fg).on(bg)))?;
                    } else {
                        #[rustfmt::skip] let TermCell { ch, fg, bg } = self.curr_ambience;
                        term.queue(PrintStyledContent(ch.with(fg).on(bg)))?;
                    }
                }
            }

            term.queue(cursor::MoveTo(0, 0))?
                .queue(terminal::EndSynchronizedUpdate)?
                .flush()?;

            // Create new blank canvas for next_buf.
            let mut temp_buf = BTreeMap::new();
            // Swap buffers so `prev_buf` correctly contains the one we just wrote and want to keep for next time.
            std::mem::swap(&mut temp_buf, &mut self.curr_buf);
            // Store previous buffer.
            self.prev_buf_and_ambience = Some((temp_buf, self.curr_ambience));
        }

        Ok(())
    }
}
