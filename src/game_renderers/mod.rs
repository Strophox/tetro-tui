pub mod braille;
/* FIXME: Note how this module does 'diff's on the strings it wants to have displayed, not on the
'underlying' game logic: An idealized renderer might actually figure out before that which game
state changes lead to exactly which minimal changes in visuals, and save itself the effort of
simulating everything it wants to print and manually diffing that like we do now? (diff_state) */
mod alpha;
mod diff_print;
mod halfcell;

use std::io::{self, Write};

use falling_tetromino_engine::{Feedback, Game, InGameTime};

use crate::{
    application::{GameMetaData, Settings, TemporaryData},
    fmt_helpers::KeybindsLegend,
};

#[allow(unused)]
pub use braille::BrailleRenderer;

pub use diff_print::DiffPrintRenderer;

#[allow(unused)]
pub use alpha::DebugRenderer;

#[allow(unused)]
pub use halfcell::HalfCellRenderer;

pub trait Renderer: Default {
    fn push_game_feedback_msgs(
        &mut self,
        feedback_msgs: impl IntoIterator<Item = (InGameTime, Feedback)>,
    );

    fn reset_game_associated_state(&mut self);

    fn reset_view_diff_state(&mut self);

    fn set_render_offset(&mut self, x: usize, y: usize);

    #[allow(clippy::too_many_arguments)]
    fn render<T: Write>(
        &mut self,
        term: &mut T,
        game: &Game,
        meta_data: &GameMetaData,
        settings: &Settings,
        temp_data: &TemporaryData,
        keybinds_legend: &KeybindsLegend,
        replay_extra: Option<(InGameTime, f64)>,
    ) -> io::Result<()>;
}
