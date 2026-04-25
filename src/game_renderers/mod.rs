mod braille;
mod legacy_buffered;
mod prototype;
mod standard_buffered;
mod twoxel;

use std::io::{self, Write};

use crossterm::event::{KeyCode, KeyModifiers};
use falling_tetromino_engine::{Button, Game, InGameTime, Notification};

use crate::{
    GameMetaData, Settings, TemporaryAppData,
    fmt_helpers::{KeybindsLegend, fmt_button_keybinds, fmt_key_with_keymods},
    tui_settings::GameKeybinds,
};

pub use braille::BrailleRenderer;
pub use legacy_buffered::LegacyBufferedRenderer;
pub use prototype::PrototypeRenderer;
pub use standard_buffered::StandardBufferedRenderer;
pub use twoxel::TwoxelRenderer;

// FIXME: Remove TetroTUIRenderer enum and make trait Renderer dyn-safe. It currently is not because:
// We have this constructor call attached to it. In practice we'll have a separate `render_from_num_and_stat_selection` function
pub trait Renderer {
    fn update_feed(
        &mut self,
        notification_feed: impl IntoIterator<Item = (Notification, InGameTime)>,
        settings: &Settings,
    );

    fn reset_veffects_state(&mut self);

    fn reset_viewport_state_with_offset_and_area(
        &mut self,
        offsets: (u16, u16),
        dimensions: (u16, u16),
    );

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

bitflags::bitflags! {
    #[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug, Default, serde::Serialize, serde::Deserialize)]
    pub struct ShowStats: u8 {
        const TIME = 0b0000_0001;
        const LINES = 0b0000_0010;
        const POINTS = 0b0000_0100;
        const PIECES = 0b0000_1000;
        const GRAVITY = 0b0001_0000;
        const LOCKDELAY = 0b0010_0000;
    }
}

pub const MAX_LEGEND_ENTRIES: u16 = 5;

pub fn calc_game_keybinds_legend(keybinds: &GameKeybinds) -> KeybindsLegend {
    let fk = |k| fmt_key_with_keymods((k, KeyModifiers::NONE));
    let fb = |b| fmt_button_keybinds(b, keybinds, " ");

    let icon_pause = fk(KeyCode::Esc);
    let icons_move = format!("{}{}", fb(Button::MoveLeft), fb(Button::MoveRight));
    let icons_rotate = format!(
        "{}{}{}",
        fb(Button::RotateLeft),
        fb(Button::Rotate180),
        fb(Button::RotateRight)
    );
    let icons_drop = format!("{}{}", fb(Button::DropSoft), fb(Button::DropHard));
    // let icons_hold = fb(Button::HoldPiece);

    // NOTE: This should be <= MAX_LEGEND_ENTRIES. Renderer relies on this for nicer visual alignment.
    vec![
        (icons_move, "move"),
        (icons_rotate, "rotate"),
        (icons_drop, "drop"),
        // (icons_hold, "hold"),
        (icon_pause, "pause"),
        ("[?]".to_owned(), "see all"),
    ]
}

pub fn replay_keybinds_legend() -> KeybindsLegend {
    let fk = |k| fmt_key_with_keymods((k, KeyModifiers::NONE));

    let icon_pause = fk(KeyCode::Char(' '));
    let icons_speed = format!("{}{}", fk(KeyCode::Down), fk(KeyCode::Up));
    let icons_skip = format!("{}{}", fk(KeyCode::Left), fk(KeyCode::Right));
    // let icons_jump = format!("{}-{}", fk(KeyCode::Char('0')), fk(KeyCode::Char('9')));
    // let icons_enter = fk(KeyCode::Enter);
    let icon_stop = fk(KeyCode::Esc);

    // NOTE: This should be <= MAX_LEGEND_ENTRIES. Renderer relies on this for nicer visual alignment.
    vec![
        (icon_pause, "pause"),
        (icons_skip, "timeskip -/+"),
        (icons_speed, "speed -/+"),
        // (icons_jump, "timejump #0%"),
        // (icons_enter, "take over"),
        (icon_stop, "exit"),
        ("[?]".to_owned(), "see all"),
    ]
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
    pub const NUM_VARIANTS: usize = 5;

    pub fn with_num(n: usize) -> Self {
        match n {
            0 => Self::StandardBuffered(StandardBufferedRenderer::default()),
            1 => Self::LegacyBuffered(LegacyBufferedRenderer::default()),
            2 => Self::Prototype(PrototypeRenderer::default()),
            3 => Self::Twoxel(TwoxelRenderer::default()),
            4 => Self::Braille(BrailleRenderer::default()),

            _ => Self::StandardBuffered(StandardBufferedRenderer::default()),
        }
    }

    pub fn name_from_num(n: usize) -> &'static str {
        match n {
            0 => "Standard",
            1 => "Legacy",
            2 => "Prototype",
            3 => "Twoxel",
            4 => "Braille",
            _ => "Standard",
        }
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

    fn reset_viewport_state_with_offset_and_area(
        &mut self,
        offsets: (u16, u16),
        dimensions: (u16, u16),
    ) {
        match self {
            TetroTUIRenderer::StandardBuffered(r) => {
                r.reset_viewport_state_with_offset_and_area(offsets, dimensions)
            }
            TetroTUIRenderer::LegacyBuffered(r) => {
                r.reset_viewport_state_with_offset_and_area(offsets, dimensions)
            }
            TetroTUIRenderer::Prototype(r) => {
                r.reset_viewport_state_with_offset_and_area(offsets, dimensions)
            }
            TetroTUIRenderer::Twoxel(r) => {
                r.reset_viewport_state_with_offset_and_area(offsets, dimensions)
            }
            TetroTUIRenderer::Braille(r) => {
                r.reset_viewport_state_with_offset_and_area(offsets, dimensions)
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
