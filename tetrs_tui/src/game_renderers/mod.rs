pub mod braille;
/* FIXME: Note how this module does 'diff's on the strings it wants to have displayed, not on the
'underlying' game logic: An idealized renderer might actually figure out before that which game
state changes lead to exactly which minimal changes in visuals, and save itself the effort of
simulating everything it wants to print and manually diffing that like we do now? (diff_state) */
pub mod diff_print;
pub mod legacy_debug;

use std::io::{self, Write};

use tetrs_engine::{Feedback, Game, InGameTime};

use crate::{
    application::{GameMetaData, Settings},
    fmt_helpers::KeybindsLegend,
};

pub trait Renderer {
    fn push_game_feedback_msgs(
        &mut self,
        feedback_msgs: impl IntoIterator<Item = (InGameTime, Feedback)>,
    );

    fn render<T: Write>(
        &mut self,
        game: &Game,
        meta_data: &GameMetaData,
        settings: &Settings,
        keybinds_legend: &KeybindsLegend,
        term: &mut T,
        refresh_entire_view: bool,
    ) -> io::Result<()>;
}
