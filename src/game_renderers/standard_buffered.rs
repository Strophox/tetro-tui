use std::{cmp::Ordering, collections::BTreeMap, fmt::Debug};

use crossterm::{
    cursor,
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal, QueueableCommand,
};
use falling_tetromino_engine::{Coordinate, NotificationFeed, TileID};

use super::*;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct TermCell {
    ch: char,
    fg: Color,
}

const EMPTY_CELL: TermCell = TermCell {
    ch: ' ',
    fg: Color::Reset,
};

pub trait TerminalBuffer {
    fn with_dimensions(width: u16, height: u16) -> Self;
    fn dimensions(&self) -> (u16, u16);
    fn reset_with_dimensions(&mut self, width: u16, height: u16);

    fn write(&mut self, x: u16, y: u16, cell: TermCell);
    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color);
    fn flush(&mut self, term: &mut impl Write) -> io::Result<()>;
}

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
struct DenseTerminalDoubleBuffer {
    width: u16,
    height: u16,
    /// INVARIANT:
    /// - `prev_buf.len() == width * height`.
    prev_buf: Vec<TermCell>,
    /// INVARIANT:
    /// - `next_buf.len() == width * height`.
    next_buf: Vec<TermCell>,
}

impl TerminalBuffer for DenseTerminalDoubleBuffer {
    fn with_dimensions(width: u16, height: u16) -> Self {
        DenseTerminalDoubleBuffer {
            width,
            height,
            prev_buf: vec![EMPTY_CELL; (width * height).into()],
            next_buf: vec![EMPTY_CELL; (width * height).into()],
        }
    }

    fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn reset_with_dimensions(&mut self, width: u16, height: u16) {
        let old_len = (self.width * self.height).into();
        let new_len = (width * height).into();
        if new_len > old_len {
            self.prev_buf.fill(EMPTY_CELL);
            self.next_buf.fill(EMPTY_CELL);
            self.prev_buf.resize(new_len, EMPTY_CELL);
            self.next_buf.resize(new_len, EMPTY_CELL);
        } else {
            self.prev_buf.resize(new_len, EMPTY_CELL);
            self.next_buf.resize(new_len, EMPTY_CELL);
            self.prev_buf.fill(EMPTY_CELL);
            self.next_buf.fill(EMPTY_CELL);
        }
        self.width = width;
        self.height = height;
    }

    fn write(&mut self, x: u16, y: u16, cell: TermCell) {
        if x < self.width && y < self.height {
            let idx = x as usize + self.width as usize * y as usize;
            self.next_buf[idx] = cell;
        }
    }

    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color) {
        if y >= self.height {
            return;
        }
        for (dx, ch) in str.chars().enumerate() {
            if x + dx as u16 >= self.width {
                return;
            }
            let idx = x as usize + dx + self.width as usize * y as usize;
            self.next_buf[idx] = TermCell { ch, fg };
        }
    }

    fn flush(&mut self, term: &mut impl Write) -> io::Result<()> {
        term.queue(terminal::BeginSynchronizedUpdate)?;

        for x in 0..self.width {
            for y in 0..self.height {
                let idx = x as usize + self.width as usize * y as usize;
                #[rustfmt::skip] let TermCell { ch: old_ch, fg: old_fg } = self.prev_buf[idx];
                #[rustfmt::skip] let TermCell { ch: new_ch, fg: new_fg } = self.next_buf[idx];

                term.queue(cursor::MoveTo(x, y))?;
                if new_fg != old_fg {
                    // Always reprint styled if style changed.
                    term.queue(PrintStyledContent(new_ch.with(new_fg)))?;
                } else if new_ch != old_ch {
                    // Style did not change, but character did, so reprint it.
                    term.queue(Print(new_ch))?;
                }
            }
        }

        term.queue(cursor::MoveTo(0, 0))?
            .queue(terminal::EndSynchronizedUpdate)?
            .flush()?;

        // Swap buffers so `prev_buf` correctly contains the one we just wrote and want to keep for next time.
        std::mem::swap(&mut self.prev_buf, &mut self.next_buf);

        // Reset buffer by overwriting nonempty cells.
        self.next_buf.fill(EMPTY_CELL);

        Ok(())
    }
}

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
struct SparseTerminalBuffer {
    width: u16,
    height: u16,
    prev_buf: BTreeMap<(u16, u16), TermCell>,
    next_buf: BTreeMap<(u16, u16), TermCell>,
}

impl TerminalBuffer for SparseTerminalBuffer {
    fn with_dimensions(width: u16, height: u16) -> Self {
        SparseTerminalBuffer {
            width,
            height,
            prev_buf: BTreeMap::new(),
            next_buf: BTreeMap::new(),
        }
    }

    fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn reset_with_dimensions(&mut self, width: u16, height: u16) {
        self.prev_buf.clear();
        self.next_buf.clear();
        self.width = width;
        self.height = height;
    }

    fn write(&mut self, x: u16, y: u16, cell: TermCell) {
        if x < self.width && y < self.height {
            self.next_buf.insert((x, y), cell);
        }
    }

    fn write_str(&mut self, x: u16, y: u16, str: &str, fg: Color) {
        if y >= self.height {
            return;
        }
        for (dx, ch) in str.chars().enumerate() {
            if x + dx as u16 >= self.width {
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
            term.queue(cursor::MoveTo(x, y))?;

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

/* The renderer must take care of:
Textures:
- locked + air tiles (=board + grid)
   * possibly blindfolded
- active piece
- shadow piece
- spawn piece shadow
- next pieces (normal, small, mini)
- hold piece
- slashed/crossed piece (if game over, forfeit etc.)
Effects:
- hard drop effect
- lock effect
- line clear effect
General TUI:
- TUI frames (main, hold, next, heading lines)
- stats hud
   * time, lines, points, gravity; adaptive lock delay
   * replay speed, replay length
- keybinds
- buttons
- goal hud
- message feed
  * gather/generate msgs from Accolades, Custom, Debug, GameEnded */

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
)]
// TODO: implement.
pub struct StandardBufferedRenderer {
    term_buf: DenseTerminalDoubleBuffer,
    notification_feed: NotificationFeed,
    message_render_buf: Vec<(InGameTime, String)>,
    hard_drop_effect_buf: Vec<HardDropEffectTile>,
    lock_effect_buf: Vec<LockEffectTile>,
    line_clear_effect_buf1: Vec<LineClearEffectTile>,
    line_clear_effect_buf2: Vec<LineClearEffectLine>,
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
)]
pub struct HardDropEffectTile {
    creation_time: InGameTime,
    pos: Coordinate,
    offset??: ??,
    tile_id: TileID,
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
)]
pub struct LockEffectTile {
    ??: ??,
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
)]
pub struct LineClearEffectTile {
    creation_time: InGameTime,
    origin: (usize, usize),
    momentum: (f32, f32),
    acceleration: (f32, f32),
    animation??: ??,
    tile_id: TileID,
}

pub struct LineClearEffectLine {
    
}

// TODO: implement.
impl Renderer for StandardBufferedRenderer {
    fn push_game_notification_feed(
        &mut self,
        feed: impl IntoIterator<Item = (Notification, InGameTime)>,
    ) {
        todo!()
    }

    fn reset_game_associated_state(&mut self) {
        todo!()
    }

    fn reset_view_diff_state(&mut self) {
        todo!()
    }

    fn set_render_offset(&mut self, x: usize, y: usize) {
        todo!()
    }

    fn render<T: Write>(
        &mut self,
        term: &mut T,
        game: &Game,
        meta_data: &GameMetaData,
        settings: &Settings,
        temp_data: &TemporaryAppData,
        keybinds_legend: &KeybindsLegend,
        replay_extra: Option<(InGameTime, f64)>,
    ) -> io::Result<()> {
        todo!()
    }
}
