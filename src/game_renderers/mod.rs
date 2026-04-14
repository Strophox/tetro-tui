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
    fn update_feed(
        &mut self,
        notification_feed: impl IntoIterator<Item = (Notification, InGameTime)>,
        settings: &Settings,
    );

    fn reset_veffects_state(&mut self);

    fn reset_viewport_with_offset_and_area(&mut self, offsets: (u16, u16), dimensions: (u16, u16));

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

#[derive(PartialEq, PartialOrd, Clone, Debug)]
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

    pub fn name(&self) -> &'static str {
        match self {
            TetroTUIRenderer::StandardBuffered(_) => "Standard",
            TetroTUIRenderer::LegacyBuffered(_) => "Legacy",
            TetroTUIRenderer::Prototype(_) => "Prototype",
            TetroTUIRenderer::Twoxel(_) => "Twoxel",
            TetroTUIRenderer::Braille(_) => "Braille",
        }
    }
}

impl Default for TetroTUIRenderer {
    fn default() -> Self {
        Self::with_number(0)
    }
}

impl Renderer for TetroTUIRenderer {
    fn update_feed(
        &mut self,
        feed: impl IntoIterator<Item = (Notification, InGameTime)>,
        settings: &Settings,
    ) {
        match self {
            TetroTUIRenderer::StandardBuffered(r) => r.update_feed(feed, settings),
            TetroTUIRenderer::LegacyBuffered(r) => r.update_feed(feed, settings),
            TetroTUIRenderer::Prototype(r) => r.update_feed(feed, settings),
            TetroTUIRenderer::Twoxel(r) => r.update_feed(feed, settings),
            TetroTUIRenderer::Braille(r) => r.update_feed(feed, settings),
        }
    }

    fn reset_veffects_state(&mut self) {
        match self {
            TetroTUIRenderer::StandardBuffered(r) => r.reset_veffects_state(),
            TetroTUIRenderer::LegacyBuffered(r) => r.reset_veffects_state(),
            TetroTUIRenderer::Prototype(r) => r.reset_veffects_state(),
            TetroTUIRenderer::Twoxel(r) => r.reset_veffects_state(),
            TetroTUIRenderer::Braille(r) => r.reset_veffects_state(),
        }
    }

    fn reset_viewport_with_offset_and_area(&mut self, offsets: (u16, u16), dimensions: (u16, u16)) {
        match self {
            TetroTUIRenderer::StandardBuffered(r) => {
                r.reset_viewport_with_offset_and_area(offsets, dimensions)
            }
            TetroTUIRenderer::LegacyBuffered(r) => {
                r.reset_viewport_with_offset_and_area(offsets, dimensions)
            }
            TetroTUIRenderer::Prototype(r) => {
                r.reset_viewport_with_offset_and_area(offsets, dimensions)
            }
            TetroTUIRenderer::Twoxel(r) => {
                r.reset_viewport_with_offset_and_area(offsets, dimensions)
            }
            TetroTUIRenderer::Braille(r) => {
                r.reset_viewport_with_offset_and_area(offsets, dimensions)
            }
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
