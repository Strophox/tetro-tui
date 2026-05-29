mod braille;
#[allow(unused)]
mod legacy_buffered;
mod main_buffered;
#[allow(unused)]
mod prototype;
mod twoxel;

use std::io::{self, Write};

use crate::{
    core_game_engine::{Game, InGameTime, Notification},
    terminal_buffers::TermCell,
};

use crate::{GameMetaData, Settings, TemporaryAppData, fmt_helpers::KeybindsLegend};

pub use braille::BrailleRenderer;
pub use main_buffered::MainBufRenderer;
pub use twoxel::TwoxelRenderer;
// pub use prototype::PrototypeRenderer;
// pub use legacy_buffered::LegacyBufferedRenderer;

/// Trait for things that are able to render one frame of game state to a terminal buffer.
// FIXME: Remove GameRenderer enum and make trait Renderer dyn-safe? It currently is not because:
// We have this constructor call attached to it. In practice we'll have a separate `render_from_num_and_stat_selection` function
pub trait GameRenderer {
    fn update_feed(
        &mut self,
        notification_feed: impl IntoIterator<Item = (Notification, InGameTime)>,
        settings: &Settings,
    );

    fn reset_veffects_state(&mut self);

    fn reset_viewport_state(&mut self, offset: (u16, u16), area: (u16, u16), ambience: TermCell);

    #[allow(clippy::too_many_arguments)]
    fn render<W: Write>(
        &mut self,
        term: &mut W,
        game: &Game,
        meta_data: &GameMetaData,
        settings: &Settings,
        temp_data: &TemporaryAppData,
        keybinds_legend: &KeybindsLegend,
        replay_extra: Option<(InGameTime, f64)>,
    ) -> io::Result<()>;
}

bitflags::bitflags! {
    #[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug, Default, serde::Serialize, serde::Deserialize)]
    pub struct ShowStatsHud: u8 {
        const TIME = 0b0000_0001;
        const LINES = 0b0000_0010;
        const POINTS = 0b0000_0100;
        const PIECES = 0b0000_1000;
        const PIECES_COUNTS = 0b0001_0000;
        const GRAVITY = 0b0010_0000;
        const LOCKDELAY = 0b0100_0000;
    }
}

pub const MAX_LEGEND_ENTRIES: u16 = 5;

#[derive(PartialEq, Clone, Debug)]
pub enum MiscGameRenderers {
    MainBuf(MainBufRenderer),
    Twoxel(TwoxelRenderer),
    Braille(BrailleRenderer),
    // Prototype(PrototypeRenderer),
    // LegacyBuf(LegacyBufRenderer),
}

impl MiscGameRenderers {
    pub const NUM_VARIANTS: usize = 3; //5;

    pub fn from_num(n: usize) -> Self {
        match n {
            0 => Self::MainBuf(MainBufRenderer::default()),
            1 => Self::Twoxel(TwoxelRenderer::default()),
            2 => Self::Braille(BrailleRenderer::default()),
            // 3 => Self::Prototype(PrototypeRenderer::default()),
            // 4 => Self::LegacyBuffered(LegacyBufferedRenderer::default()),
            _ => Self::MainBuf(MainBufRenderer::default()),
        }
    }

    pub fn name_from_num(n: usize) -> &'static str {
        match n {
            0 => "Main",
            1 => "Half-block",
            2 => "Braille",
            // 3 => "Prototype",
            // 4 => "Legacy",
            _ => "Main",
        }
    }
}

impl GameRenderer for MiscGameRenderers {
    fn update_feed(
        &mut self,
        feed: impl IntoIterator<Item = (Notification, InGameTime)>,
        settings: &Settings,
    ) {
        match self {
            MiscGameRenderers::MainBuf(r) => r.update_feed(feed, settings),
            MiscGameRenderers::Twoxel(r) => r.update_feed(feed, settings),
            MiscGameRenderers::Braille(r) => r.update_feed(feed, settings),
            // TetroTUIRenderer::Prototype(r) => r.update_feed(feed, settings),
            // TetroTUIRenderer::LegacyBuffered(r) => r.update_feed(feed, settings),
        }
    }

    fn reset_veffects_state(&mut self) {
        match self {
            MiscGameRenderers::MainBuf(r) => r.reset_veffects_state(),
            MiscGameRenderers::Twoxel(r) => r.reset_veffects_state(),
            MiscGameRenderers::Braille(r) => r.reset_veffects_state(),
            // TetroTUIRenderer::Prototype(r) => r.reset_veffects_state(),
            // TetroTUIRenderer::LegacyBuffered(r) => r.reset_veffects_state(),
        }
    }

    fn reset_viewport_state(&mut self, offset: (u16, u16), area: (u16, u16), ambience: TermCell) {
        match self {
            MiscGameRenderers::MainBuf(r) => r.reset_viewport_state(offset, area, ambience),
            MiscGameRenderers::Twoxel(r) => r.reset_viewport_state(offset, area, ambience),
            MiscGameRenderers::Braille(r) => r.reset_viewport_state(offset, area, ambience), // TetroTUIRenderer::Prototype(r) => {
                                                                                             //     r.reset_viewport_state_with_offset_and_area(offsets, dimensions)
                                                                                             // }
                                                                                             // TetroTUIRenderer::LegacyBuffered(r) => {
                                                                                             //     r.reset_viewport_state_with_offset_and_area(offsets, dimensions)
                                                                                             // }
        }
    }

    #[rustfmt::skip]
    fn render<W: Write>(
        &mut self,
        term: &mut W,
        game: &Game,
        meta_data: &GameMetaData,
        settings: &Settings,
        temp_data: &TemporaryAppData,
        keybinds_legend: &KeybindsLegend,
        replay_extra: Option<(InGameTime, f64)>,
    ) -> io::Result<()> {
        match self {
            MiscGameRenderers::MainBuf(r) => r.render(term, game, meta_data, settings, temp_data, keybinds_legend, replay_extra),
            MiscGameRenderers::Twoxel(r) => r.render(term, game, meta_data, settings, temp_data, keybinds_legend, replay_extra),
            MiscGameRenderers::Braille(r) => r.render(term, game, meta_data, settings, temp_data, keybinds_legend, replay_extra),
            // TetroTUIRenderer::Prototype(r) => r.render(term, game, meta_data, settings, temp_data, keybinds_legend, replay_extra),
            // TetroTUIRenderer::LegacyBuffered(r) => r.render(term, game, meta_data, settings, temp_data, keybinds_legend, replay_extra),
        }
    }
}
