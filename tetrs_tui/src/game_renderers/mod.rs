pub mod cached_renderer;
pub mod debug_renderer;

use std::io::{self, Write};

use tetrs_engine::{FeedbackEvents, Game};

use crate::terminal_app::{RunningGameStats, TerminalApp};

pub trait Renderer {
    fn render<T>(
        &mut self,
        app: &mut TerminalApp<T>,
        running_game_stats: &mut RunningGameStats,
        game: &Game,
        new_feedback_events: FeedbackEvents,
        screen_resized: bool,
    ) -> io::Result<()>
    where
        T: Write;
}
