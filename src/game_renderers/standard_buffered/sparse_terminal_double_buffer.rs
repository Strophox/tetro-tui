use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt::Debug,
    io::{self, Write},
};

use crossterm::{
    cursor,
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal, QueueableCommand,
};

use super::{TermCell, TerminalBuffer};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Default)]
pub struct SparseTerminalBuffer {
    prev_buf: BTreeMap<(u16, u16), TermCell>,
    next_buf: BTreeMap<(u16, u16), TermCell>,
    x_vp: u16,
    y_vp: u16,
    w_vp: u16,
    h_vp: u16,
}

impl TerminalBuffer for SparseTerminalBuffer {
    fn with_offset_and_area((x, y): (u16, u16), (w, h): (u16, u16)) -> Self {
        SparseTerminalBuffer {
            prev_buf: BTreeMap::new(),
            next_buf: BTreeMap::new(),
            x_vp: x,
            y_vp: y,
            w_vp: w,
            h_vp: h,
        }
    }

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

    fn write(&mut self, x: u16, y: u16, cell: TermCell) {
        if x < self.w_vp && y < self.h_vp {
            self.next_buf.insert((x, y), cell);
        }
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

    fn flush(&mut self, term: &mut impl Write) -> io::Result<()> {
        term.queue(terminal::BeginSynchronizedUpdate)?;

        // We'll be consuming both iterators and compare.
        let mut old_buffer = self.prev_buf.iter();
        let mut new_buffer = self.next_buf.iter();

        let mut term_queue = |(x, y): (u16, u16), ch: char, fg: Option<Color>| -> io::Result<()> {
            term.queue(cursor::MoveTo(self.x_vp + x, self.y_vp + y))?;

            if let Some(color) = fg {
                term.queue(PrintStyledContent(ch.with(color)))?;
            } else {
                term.queue(Print(ch))?;
            }

            Ok(())
        };

        let mut old_pos_cell = old_buffer.next();
        let mut new_pos_cell = new_buffer.next();
        loop {
            match (old_pos_cell, new_pos_cell) {
                // Both are empty, nothing to do.
                (None, None) => break,

                #[rustfmt::skip]
                // Old buffer contains something the new one doesn't: Overwrite it to clear it.
                (Some((old_pos, TermCell { ch: _old_ch, fg: old_fg })),
                 None
                ) => {
                    // Only explicitly reset color if necessary.
                    let new_fg = (*old_fg != Color::Reset).then_some(Color::Reset);
                    term_queue(*old_pos, ' ', new_fg)?;

                    old_pos_cell = old_buffer.next();
                }

                #[rustfmt::skip]
                // New buffer contains something the old one doesn't: Write it.
                (None,
                 Some((new_x_y, TermCell { ch: new_ch, fg: new_fg })),
                ) => {
                    // Only explicitly reset color if necessary.
                    let new_fg = (*new_fg != Color::Reset).then_some(*new_fg);
                    term_queue(*new_x_y, *new_ch, new_fg)?;

                    new_pos_cell = new_buffer.next();
                }

                #[rustfmt::skip]
                (Some((old_pos, TermCell { ch: old_ch, fg: old_fg })),
                 Some((new_pos, TermCell { ch: new_ch, fg: new_fg })),
                ) => {
                    match old_pos.cmp(new_pos) {
                        // Old buffer contains something the new one doesn't: Overwrite it to clear it.
                        Ordering::Less => {
                            // Only explicitly reset color if necessary.
                            let new_fg = (*old_fg != Color::Reset).then_some(Color::Reset);
                            term_queue(*old_pos, ' ', new_fg)?;

                            old_pos_cell = old_buffer.next();
                        }

                        // New buffer contains something the old one doesn't: Write it.
                        Ordering::Greater => {
                            // Only explicitly reset color if necessary.
                            let new_fg = (*new_fg != Color::Reset).then_some(*new_fg);
                            term_queue(*new_pos, *new_ch, new_fg)?;

                            new_pos_cell = new_buffer.next();
                        }

                        // Old and new overlap! Handle possible difference.
                        Ordering::Equal => {
                            if *old_fg != *new_fg {
                                // Definitely need to change if color changed.
                                term_queue(*new_pos, *new_ch, Some(*new_fg))?;
                            } else if *old_ch != *new_ch {
                                // Only content changed, just print.
                                term_queue(*new_pos, *new_ch, None)?;
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

        Ok(())
    }
}
