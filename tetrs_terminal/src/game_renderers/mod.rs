pub mod cached;
pub mod immediate;
pub mod naive;

use std::io::{self, Write};

use tetrs_engine::{FeedbackEvents, Game};

use crate::terminal_tetrs::{App, RunningGameStats};

pub trait GameScreenRenderer {
    fn render<T>(
        &mut self,
        app: &mut App<T>,
        game: &mut Game,
        action_stats: &mut RunningGameStats,
        new_feedback_events: FeedbackEvents,
        screen_resized: bool,
    ) -> io::Result<()>
    where
        T: Write;
}
