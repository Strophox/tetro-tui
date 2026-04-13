pub mod braille;
/* FIXME: Note how this module does 'diff's on the strings it wants to have displayed, not on the
'underlying' game logic: An idealized renderer might actually figure out before that which game
state changes lead to exactly which minimal changes in visuals, and save itself the effort of
simulating everything it wants to print and manually diffing that like we do now? (diff_state) */
mod legacy_buffered;
mod prototype;
mod standard_buffered;
mod twoxel;

use std::io::{self, Write};

use falling_tetromino_engine::{Game, InGameTime, Notification};

use crate::{fmt_helpers::KeybindsLegend, GameMetaData, Settings, TemporaryAppData};

pub use braille::BrailleRenderer;
pub use legacy_buffered::LegacyBufferedRenderer;
pub use prototype::PrototypeRenderer;
pub use standard_buffered::StandardBufferedRenderer;
pub use twoxel::TwoxelRenderer;

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
    StandardBuffered(StandardBufferedRenderer),
    LegacyBuffered(LegacyBufferedRenderer),
    Prototype(PrototypeRenderer),
    Twoxel(TwoxelRenderer),
    Braille(BrailleRenderer),
}

impl TetroTUIRenderer {
    pub const NUM_VARIANTS: usize = 6;

    pub fn with_number(n: usize) -> Self {
        match n {
            0 => Self::StandardBuffered(Default::default()),
            1 => Self::LegacyBuffered(Default::default()),
            2 => Self::Prototype(Default::default()),
            3 => Self::LegacyBuffered(Default::default()),
            4 => Self::Twoxel(Default::default()),
            5 => Self::Braille(Default::default()),

            _ => Self::StandardBuffered(Default::default()),
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
            TetroTUIRenderer::StandardBuffered(r) => r.push_game_notification_feed(feed),
            TetroTUIRenderer::LegacyBuffered(r) => r.push_game_notification_feed(feed),
            TetroTUIRenderer::Prototype(r) => r.push_game_notification_feed(feed),
            TetroTUIRenderer::Twoxel(r) => r.push_game_notification_feed(feed),
            TetroTUIRenderer::Braille(r) => r.push_game_notification_feed(feed),
        }
    }

    fn reset_game_associated_state(&mut self) {
        match self {
            TetroTUIRenderer::StandardBuffered(r) => r.reset_game_associated_state(),
            TetroTUIRenderer::LegacyBuffered(r) => r.reset_game_associated_state(),
            TetroTUIRenderer::Prototype(r) => r.reset_game_associated_state(),
            TetroTUIRenderer::Twoxel(r) => r.reset_game_associated_state(),
            TetroTUIRenderer::Braille(r) => r.reset_game_associated_state(),
        }
    }

    fn reset_view_diff_state(&mut self) {
        match self {
            TetroTUIRenderer::StandardBuffered(r) => r.reset_view_diff_state(),
            TetroTUIRenderer::LegacyBuffered(r) => r.reset_view_diff_state(),
            TetroTUIRenderer::Prototype(r) => r.reset_view_diff_state(),
            TetroTUIRenderer::Twoxel(r) => r.reset_view_diff_state(),
            TetroTUIRenderer::Braille(r) => r.reset_view_diff_state(),
        }
    }

    fn set_render_offset(&mut self, x: usize, y: usize) {
        match self {
            TetroTUIRenderer::StandardBuffered(r) => r.set_render_offset(x, y),
            TetroTUIRenderer::LegacyBuffered(r) => r.set_render_offset(x, y),
            TetroTUIRenderer::Prototype(r) => r.set_render_offset(x, y),
            TetroTUIRenderer::Twoxel(r) => r.set_render_offset(x, y),
            TetroTUIRenderer::Braille(r) => r.set_render_offset(x, y),
        }
    }

    #[rustfmt::skip]
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
            TetroTUIRenderer::StandardBuffered(r) => r.render(term, game, meta_data, settings, temp_data, keybinds_legend, replay_extra),
            TetroTUIRenderer::LegacyBuffered(r) => r.render(term, game, meta_data, settings, temp_data, keybinds_legend, replay_extra),
            TetroTUIRenderer::Prototype(r) => r.render(term, game, meta_data, settings, temp_data, keybinds_legend, replay_extra),
            TetroTUIRenderer::Twoxel(r) => r.render(term, game, meta_data, settings, temp_data, keybinds_legend, replay_extra),
            TetroTUIRenderer::Braille(r) => r.render(term, game, meta_data, settings, temp_data, keybinds_legend, replay_extra),
        }
    }
}
