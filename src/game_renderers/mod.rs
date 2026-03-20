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

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum TetroTUIRenderer {
    DiffPrint(DiffPrintRenderer),
    Debug(DebugRenderer),
    HalfCell(HalfCellRenderer),
    Braille(BrailleRenderer),
}

impl TetroTUIRenderer {
    pub const NUM_VARIANTS: usize = 4;

    pub fn with_number(n: usize) -> Self {
        match n {
            0 => Self::DiffPrint(Default::default()),
            1 => Self::Debug(Default::default()),
            2 => Self::HalfCell(Default::default()),
            3 => Self::Braille(Default::default()),
            // Fallback
            _ => Self::DiffPrint(Default::default()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::DiffPrint(_) => "Main (diff-print)",
            Self::Debug(_) => "Alpha/Debug",
            Self::HalfCell(_) => "Half cell",
            Self::Braille(_) => "Braille",
        }
    }
}

impl Default for TetroTUIRenderer {
    fn default() -> Self {
        Self::with_number(0)
    }
}

impl Renderer for TetroTUIRenderer {
    fn push_game_feedback_msgs(
        &mut self,
        feedback_msgs: impl IntoIterator<Item = (InGameTime, Feedback)>,
    ) {
        match self {
            TetroTUIRenderer::DiffPrint(r) => r.push_game_feedback_msgs(feedback_msgs),
            TetroTUIRenderer::Debug(r) => r.push_game_feedback_msgs(feedback_msgs),
            TetroTUIRenderer::HalfCell(r) => r.push_game_feedback_msgs(feedback_msgs),
            TetroTUIRenderer::Braille(r) => r.push_game_feedback_msgs(feedback_msgs),
        }
    }

    fn reset_game_associated_state(&mut self) {
        match self {
            TetroTUIRenderer::DiffPrint(r) => r.reset_game_associated_state(),
            TetroTUIRenderer::Debug(r) => r.reset_game_associated_state(),
            TetroTUIRenderer::HalfCell(r) => r.reset_game_associated_state(),
            TetroTUIRenderer::Braille(r) => r.reset_game_associated_state(),
        }
    }

    fn reset_view_diff_state(&mut self) {
        match self {
            TetroTUIRenderer::DiffPrint(r) => r.reset_view_diff_state(),
            TetroTUIRenderer::Debug(r) => r.reset_view_diff_state(),
            TetroTUIRenderer::HalfCell(r) => r.reset_view_diff_state(),
            TetroTUIRenderer::Braille(r) => r.reset_view_diff_state(),
        }
    }

    fn set_render_offset(&mut self, x: usize, y: usize) {
        match self {
            TetroTUIRenderer::DiffPrint(r) => r.set_render_offset(x, y),
            TetroTUIRenderer::Debug(r) => r.set_render_offset(x, y),
            TetroTUIRenderer::HalfCell(r) => r.set_render_offset(x, y),
            TetroTUIRenderer::Braille(r) => r.set_render_offset(x, y),
        }
    }

    fn render<T: Write>(
        &mut self,
        term: &mut T,
        game: &Game,
        meta_data: &GameMetaData,
        settings: &Settings,
        temp_data: &TemporaryData,
        keybinds_legend: &KeybindsLegend,
        replay_extra: Option<(InGameTime, f64)>,
    ) -> io::Result<()> {
        match self {
            TetroTUIRenderer::DiffPrint(r) => r.render(
                term,
                game,
                meta_data,
                settings,
                temp_data,
                keybinds_legend,
                replay_extra,
            ),
            TetroTUIRenderer::Debug(r) => r.render(
                term,
                game,
                meta_data,
                settings,
                temp_data,
                keybinds_legend,
                replay_extra,
            ),
            TetroTUIRenderer::HalfCell(r) => r.render(
                term,
                game,
                meta_data,
                settings,
                temp_data,
                keybinds_legend,
                replay_extra,
            ),
            TetroTUIRenderer::Braille(r) => r.render(
                term,
                game,
                meta_data,
                settings,
                temp_data,
                keybinds_legend,
                replay_extra,
            ),
        }
    }
}
