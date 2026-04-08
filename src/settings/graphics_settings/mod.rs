use std::num::NonZeroUsize;

use falling_tetromino_engine::ExtNonNegF64;

use crate::settings::SlotMachine;

pub mod hard_drop_effect;
pub mod line_clear_effect;
pub mod lock_effect;
pub mod mini_tet_style;
pub mod mino_textures;
pub mod palette;
pub mod small_tet_style;
pub mod tui_style;

// TODO: Make this ergonomic, and serialize efficiently.
pub type TileTexture = [char; 2];

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GraphicsSettingsNew {
    pub palette_picked: usize,
    pub tui_style_picked: usize,
    pub mino_textures_picked: usize,
    pub hard_drop_picked: usize,
    pub piece_lock_picked: usize,
    pub line_clear_picked: usize,
    pub mini_tet_picked: usize,
    pub small_tet_picked: usize,
    pub normalsize_previews: NonZeroUsize,
    pub fps: ExtNonNegF64,
    pub boardpalette_picked: usize,
    pub show_stats_hud: bool,
    pub show_keybinds: bool,
    pub show_buttons: bool,
    pub show_shadow: bool,
    pub show_spawn: bool,
    pub show_grid: bool,
    pub show_fps: bool,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum Glyphset {
    Elektronika60,
    Ascii,
    Unicode,
}

// TODO: Replace with new graphics settings!
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphicsSettings {
    pub palette_picked: usize,
    pub boardpalette_picked: usize,
    pub glyphset: Glyphset,
    pub effects: bool,
    pub lineclear_style: u8,
    pub shadow_piece: bool,
    pub button_state: bool,
    pub game_fps: f64,
    pub show_fps: bool,
}

// TODO: Replace with new graphics settings!
pub fn default_graphics_slots() -> SlotMachine<GraphicsSettings> {
    let slots = vec![
        ("Default".to_owned(), GraphicsSettings::default()),
        ("Focus+".to_owned(), GraphicsSettings::extra_focus()),
        ("Guideline".to_owned(), GraphicsSettings::guideline()),
        (
            "Terminal Compatibility".to_owned(),
            GraphicsSettings::compatibility(),
        ),
        (
            "Elektronika 60".to_owned(),
            GraphicsSettings::elektronika_60(),
        ),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Graphics".to_owned())
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            glyphset: Glyphset::Unicode,
            palette_picked: 3,
            boardpalette_picked: 3,
            effects: true,
            lineclear_style: 0,
            shadow_piece: true,
            button_state: false,
            game_fps: 30.0,
            show_fps: false,
        }
    }
}

impl GraphicsSettings {
    pub fn extra_focus() -> Self {
        Self {
            palette_picked: 2,
            boardpalette_picked: 0,
            effects: false,
            lineclear_style: 0,
            game_fps: 60.0,
            glyphset: Glyphset::Unicode,
            shadow_piece: true,
            button_state: false,
            show_fps: false,
        }
    }

    pub fn guideline() -> Self {
        Self {
            glyphset: Glyphset::Unicode,
            palette_picked: 2,
            boardpalette_picked: 2,
            effects: true,
            lineclear_style: 0,
            shadow_piece: true,
            button_state: false,
            game_fps: 60.0,
            show_fps: false,
        }
    }

    pub fn compatibility() -> Self {
        Self {
            palette_picked: 1,
            boardpalette_picked: 1,
            effects: true,
            lineclear_style: 0,
            game_fps: 30.0,
            glyphset: Glyphset::Ascii,
            shadow_piece: true,
            button_state: false,
            show_fps: false,
        }
    }

    pub fn elektronika_60() -> Self {
        Self {
            palette_picked: 0,
            boardpalette_picked: 0,
            effects: true,
            lineclear_style: 0,
            game_fps: 24.0,
            glyphset: Glyphset::Elektronika60,
            shadow_piece: false,
            button_state: false,
            show_fps: false,
        }
    }
}
