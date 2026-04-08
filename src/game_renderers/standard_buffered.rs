use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt::{Debug, Display},
    num::NonZeroU8,
    time::Duration,
};

use crossterm::{
    cursor,
    style::{self, Color, Print, PrintStyledContent, Stylize},
    terminal, QueueableCommand,
};

use falling_tetromino_engine::{
    Button, Coordinate, GameEndCause, InGameTime, Orientation, Phase, Stat, Tetromino, TileID,
};
use rand::RngExt;

use super::*;

use crate::{
    application::TemporaryAppData,
    fmt_helpers::{fmt_button, fmt_button_ascii, fmt_duration, fmt_hertz, FmtTetromino},
    graphics_settings::Glyphset,
};

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
struct TerminalBuffer1 {
    pub chars: BTreeMap<(u16, u16), (char, Color)>,
}

impl TerminalBuffer1 {
    fn get_terminal_size(&self) -> (u16, u16) {
        terminal::size().unwrap_or_default()
    }

    fn flush_diff(&self, term: &mut impl Write, previous: TerminalBuffer1) -> io::Result<()> {
        term.queue(terminal::BeginSynchronizedUpdate)?;

        // We'll be consuming both iterators and compare.
        let mut old_items = previous.chars.into_iter();
        let mut new_items = self.chars.iter();

        let mut term_queue =
            |(x, y): (u16, u16), (c, fg): (char, Option<Color>)| -> io::Result<()> {
                term.queue(cursor::MoveTo(x, y))?;

                if let Some(color) = fg {
                    term.queue(PrintStyledContent(c.with(color)))?;
                } else {
                    term.queue(Print(c))?;
                }

                Ok(())
            };

        let mut old_item = old_items.next();
        let mut new_item = new_items.next();
        loop {
            match (old_item, new_item) {
                // Both are empty, nothing to do.
                (None, None) => break,

                // Old buffer contains something the new one doesn't: Overwrite it to clear it.
                (Some((old_x_y, (old_c, old_fg))), None) => {
                    // Only explicitly reset color if necessary.
                    let new_fg = (old_fg != Color::Reset).then_some(Color::Reset);
                    term_queue(old_x_y, (' ', new_fg))?;

                    old_item = old_items.next();
                }

                // New buffer contains something the old one doesn't: Write it.
                (None, Some((new_x_y, (new_c, new_fg)))) => {
                    // Only explicitly reset color if necessary.
                    let new_fg = (*new_fg != Color::Reset).then_some(*new_fg);
                    term_queue(*new_x_y, (*new_c, new_fg))?;

                    new_item = new_items.next();
                }

                (Some((old_x_y, (old_c, old_fg))), Some((new_x_y, (new_c, new_fg)))) => {
                    match old_x_y.cmp(new_x_y) {
                        // Old buffer contains something the new one doesn't: Overwrite it to clear it.
                        Ordering::Less => {
                            // Only explicitly reset color if necessary.
                            let new_fg = (old_fg != Color::Reset).then_some(Color::Reset);
                            term_queue(old_x_y, (' ', new_fg))?;

                            old_item = old_items.next();
                        }

                        // New buffer contains something the old one doesn't: Write it.
                        Ordering::Greater => {
                            // Only explicitly reset color if necessary.
                            let new_fg = (*new_fg != Color::Reset).then_some(*new_fg);
                            term_queue(*new_x_y, (*new_c, new_fg))?;

                            new_item = new_items.next();
                        }

                        // Old and new overlap! Handle possible difference.
                        Ordering::Equal => {
                            if old_fg != *new_fg {
                                // Definitely need to change if color changed.
                                term_queue(*new_x_y, (*new_c, Some(*new_fg)))?;
                            } else if old_c != *new_c {
                                // Only content changed, just print.
                                term_queue(*new_x_y, (*new_c, None))?;
                            }

                            old_item = old_items.next();
                            new_item = new_items.next();
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

// TODO: Structs for various effects.

// pub struct DiffPrintRenderer {
//     buffer: TerminalBuffer,
//     notification_feed_buffer: Vec<(Notification, InGameTime, bool)>,
//     buffered_text_msgs: Vec<(InGameTime, String)>,
//     hard_drop_tiles: Vec<(HardDropTile, bool)>,
//     mino_particles: Vec<(MinoParticle, bool)>,
// }
