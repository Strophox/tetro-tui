pub mod braille;
// FIXME: Note that this module does 'diff's on the strings it prints, not on the 'underlying' game logic.
// An ideal renderer would know what game state changes lead to exactly which minimal changes in display...
pub mod diff_print;
pub mod legacy_debug;

use std::io::{self, Write};

use tetrs_engine::{FeedbackMessages, Game};

use crate::application::{Application, GameMetaData};

pub trait Renderer {
    fn render<T: Write>(
        &mut self,
        app: &mut Application<T>,
        game: &Game,
        meta_data: &GameMetaData,
        new_msgs: FeedbackMessages,
        screen_resized: bool,
    ) -> io::Result<()>;
}
