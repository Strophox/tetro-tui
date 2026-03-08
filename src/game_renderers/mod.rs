pub mod braille;
/* FIXME: Note how this module does 'diff's on the strings it wants to have displayed, not on the
'underlying' game logic: An idealized renderer might actually figure out before that which game
state changes lead to exactly which minimal changes in visuals, and save itself the effort of
simulating everything it wants to print and manually diffing that like we do now? (diff_state) */
mod diff_print;
mod legacy_debug;
mod smallascii;

use std::io::{self, Write};

use falling_tetromino_engine::{Feedback, Game, InGameTime};

use crate::{
    application::{GameMetaData, Settings},
    fmt_helpers::KeybindsLegend,
};

#[allow(unused)]
pub use braille::BrailleRenderer;

pub use diff_print::DiffPrintRenderer;

#[allow(unused)]
pub use legacy_debug::DebugRenderer;

#[allow(unused)]
pub use smallascii::SmallAsciiRenderer;

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
        game: &Game,
        meta_data: &GameMetaData,
        settings: &Settings,
        keybinds_legend: &KeybindsLegend,
        replay_extra: Option<(InGameTime, f64)>,
        term: &mut T,
    ) -> io::Result<()>;
}
