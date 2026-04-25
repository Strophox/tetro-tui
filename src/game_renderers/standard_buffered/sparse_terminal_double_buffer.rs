use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt::Debug,
    io::{self, Write},
};

use crossterm::{
    QueueableCommand, cursor,
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal,
};

use crate::tui_settings::TileTexture;

use super::{TermCell, TerminalBuffer};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Default)]
pub struct SparseTerminalDoubleBuffer {
    prev_buf: BTreeMap<(u16, u16), TermCell>,
    next_buf: BTreeMap<(u16, u16), TermCell>,
    x_vp: u16,
    y_vp: u16,
    w_vp: u16,
    h_vp: u16,
}

impl TerminalBuffer for SparseTerminalDoubleBuffer {
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

    fn reset_with_offset_and_area(&mut self, (x, y): (u16, u16), (w, h): (u16, u16)) {
        self.prev_buf.clear();
        self.next_buf.clear();
        self.x_vp = x;
        self.y_vp = y;
        self.w_vp = w;
        self.h_vp = h;
    }

    fn write_char(&mut self, x: u16, y: u16, cell: TermCell) {
        if x < self.w_vp && y < self.h_vp {
            self.next_buf.insert((x, y), cell);
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
        self.next_buf.insert((x, y), TermCell { ch: ch0, fg });

        if x + 1 >= self.w_vp {
            return;
        }
        self.next_buf.insert((x + 1, y), TermCell { ch: ch1, fg });
    }

    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color) {
        if y >= self.h_vp {
            return;
        }
        for (dx, ch) in str.chars().enumerate() {
            if x + dx as u16 >= self.w_vp {
                return;
            }
            self.next_buf
                .insert((x + dx as u16, y), TermCell { ch, fg });
        }
    }

    fn write_str_wrapping(&mut self, x: u16, y: u16, str: &str, fg: Color) {
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
            self.next_buf
                .insert((x + dx as u16, y + dy as u16), TermCell { ch, fg });
            dx += 1;
        }
    }

    fn flush(&mut self, term: &mut impl Write) -> io::Result<()> {
        term.queue(terminal::BeginSynchronizedUpdate)?;

        // We'll be consuming both iterators and compare.
        let mut old_buffer = self.prev_buf.iter();
        let mut new_buffer = self.next_buf.iter();

        let mut old_pos_cell = old_buffer.next();
        let mut new_pos_cell = new_buffer.next();
        loop {
            match (old_pos_cell, new_pos_cell) {
                // Both are empty, nothing to do.
                (None, None) => break,

                #[rustfmt::skip]
                // Old buffer contains something the new one doesn't: Overwrite it to clear it.
                (Some((old_x_y, TermCell { ch: _old_ch, fg: old_fg })),
                 None
                ) => {
                    term.queue(cursor::MoveTo(self.x_vp + old_x_y.0, self.y_vp + old_x_y.1));
                    term.queue(PrintStyledContent(' '.with(Color::Reset)))?;
                    old_pos_cell = old_buffer.next();
                }

                #[rustfmt::skip]
                // New buffer contains something the old one doesn't: Write it.
                (None,
                 Some((new_x_y, TermCell { ch: new_ch, fg: new_fg })),
                ) => {
                    term.queue(cursor::MoveTo(self.x_vp + new_x_y.0, self.y_vp + new_x_y.1));
                    term.queue(PrintStyledContent(new_ch.with(*new_fg)))?;
                    new_pos_cell = new_buffer.next();
                }

                #[rustfmt::skip]
                (Some((old_x_y, TermCell { ch: old_ch, fg: old_fg })),
                 Some((new_x_y, TermCell { ch: new_ch, fg: new_fg })),
                ) => {
                    match old_x_y.cmp(new_x_y) {
                        // Old buffer contains something the new one doesn't: Overwrite it to clear it.
                        Ordering::Less => {
                            term.queue(cursor::MoveTo(self.x_vp + old_x_y.0, self.y_vp + old_x_y.1));
                            term.queue(PrintStyledContent(' '.with(Color::Reset)))?;
                            old_pos_cell = old_buffer.next();
                        }

                        // New buffer contains something the old one doesn't: Write it.
                        Ordering::Greater => {
                            term.queue(cursor::MoveTo(self.x_vp + new_x_y.0, self.y_vp + new_x_y.1));
                            term.queue(PrintStyledContent(new_ch.with(*new_fg)))?;
                            new_pos_cell = new_buffer.next();
                        }

                        // Old and new overlap! Handle possible difference.
                        Ordering::Equal => {
                            if new_fg != old_fg || new_ch != old_ch {
                                term.queue(cursor::MoveTo(self.x_vp + new_x_y.0, self.y_vp + new_x_y.1));
                                term.queue(PrintStyledContent(new_ch.with(*new_fg)))?;
                            }
                            old_pos_cell = old_buffer.next();
                            new_pos_cell = new_buffer.next();
                        }
                    }
                }
            }
        }

        term.queue(cursor::MoveTo(0, 0))?
            .queue(terminal::EndSynchronizedUpdate)?
            .flush()?;

        std::mem::swap(&mut self.prev_buf, &mut self.next_buf);
        self.next_buf.clear();

        Ok(())
    }
}
