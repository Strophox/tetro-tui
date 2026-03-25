pub mod braille;
/* FIXME: Note how this module does 'diff's on the strings it wants to have displayed, not on the
'underlying' game logic: An idealized renderer might actually figure out before that which game
state changes lead to exactly which minimal changes in visuals, and save itself the effort of
simulating everything it wants to print and manually diffing that like we do now? (diff_state) */
mod diff_print;
mod halfcell;
mod prototype;

use std::io::{self, Write};

use falling_tetromino_engine::{Game, InGameTime, Notification};

use crate::{
    application::{GameMetaData, Settings, TemporaryAppData},
    fmt_helpers::KeybindsLegend,
};

#[allow(unused)]
pub use braille::BrailleRenderer;

pub use diff_print::DiffPrintRenderer;

#[allow(unused)]
pub use prototype::PrototypeRenderer;

#[allow(unused)]
pub use halfcell::HalfCellRenderer;

pub trait Renderer: Default {
    fn push_game_notification_feed(
        &mut self,
        feed: impl IntoIterator<Item = (Notification, InGameTime)>,
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
        temp_data: &TemporaryAppData,
        keybinds_legend: &KeybindsLegend,
        replay_extra: Option<(InGameTime, f64)>,
    ) -> io::Result<()>;
}

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum TetroTUIRenderer {
    DiffPrint(DiffPrintRenderer),
    HalfCell(HalfCellRenderer),
    Braille(BrailleRenderer),
    Prototype(PrototypeRenderer),
}

impl TetroTUIRenderer {
    pub const NUM_VARIANTS: usize = 4;

    pub fn with_number(n: usize) -> Self {
        match n {
            0 => Self::DiffPrint(Default::default()),
            1 => Self::HalfCell(Default::default()),
            2 => Self::Braille(Default::default()),
            // 4 => Self::Prototype(Default::default()),
            // Fallback
            _ => Self::DiffPrint(Default::default()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::DiffPrint(_) => "Default",
            Self::HalfCell(_) => "Halfcell",
            Self::Braille(_) => "Braille",
            Self::Prototype(_) => "Prototype",
        }
    }
}

impl Default for TetroTUIRenderer {
    fn default() -> Self {
        Self::with_number(0)
    }
}

impl Renderer for TetroTUIRenderer {
    fn push_game_notification_feed(
        &mut self,
        feed: impl IntoIterator<Item = (Notification, InGameTime)>,
    ) {
        match self {
            TetroTUIRenderer::DiffPrint(r) => r.push_game_notification_feed(feed),
            TetroTUIRenderer::HalfCell(r) => r.push_game_notification_feed(feed),
            TetroTUIRenderer::Braille(r) => r.push_game_notification_feed(feed),
            TetroTUIRenderer::Prototype(r) => r.push_game_notification_feed(feed),
        }
    }

    fn reset_game_associated_state(&mut self) {
        match self {
            TetroTUIRenderer::DiffPrint(r) => r.reset_game_associated_state(),
            TetroTUIRenderer::HalfCell(r) => r.reset_game_associated_state(),
            TetroTUIRenderer::Braille(r) => r.reset_game_associated_state(),
            TetroTUIRenderer::Prototype(r) => r.reset_game_associated_state(),
        }
    }

    fn reset_view_diff_state(&mut self) {
        match self {
            TetroTUIRenderer::DiffPrint(r) => r.reset_view_diff_state(),
            TetroTUIRenderer::HalfCell(r) => r.reset_view_diff_state(),
            TetroTUIRenderer::Braille(r) => r.reset_view_diff_state(),
            TetroTUIRenderer::Prototype(r) => r.reset_view_diff_state(),
        }
    }

    fn set_render_offset(&mut self, x: usize, y: usize) {
        match self {
            TetroTUIRenderer::DiffPrint(r) => r.set_render_offset(x, y),
            TetroTUIRenderer::HalfCell(r) => r.set_render_offset(x, y),
            TetroTUIRenderer::Braille(r) => r.set_render_offset(x, y),
            TetroTUIRenderer::Prototype(r) => r.set_render_offset(x, y),
        }
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
            TetroTUIRenderer::Prototype(r) => r.render(
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
