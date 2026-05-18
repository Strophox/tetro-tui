use std::num::NonZeroUsize;

use crate::core_game_engine::ExtNonNegF64;

use crate::settings::SlotMachine;

pub mod hard_drop_effect;
pub mod line_clear_effect;
pub mod lock_effect;
pub mod mini_tetromino_symbols;
pub mod small_tetromino_symbols;
pub mod tile_coloring;
pub mod tile_symbols;
pub mod tui_coloring;
pub mod tui_symbols;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GraphicsSettings {
    #[serde(rename = "tilecol_sel")]
    pub tile_coloring_selected: usize,
    #[serde(rename = "tuisymb_sel")]
    pub tui_symbols_selected: usize,
    #[serde(rename = "tuicol_sel")]
    pub tui_coloring_selected: usize,
    #[serde(rename = "tilesymb_sel")]
    pub tile_symbols_selected: usize,
    #[serde(rename = "harddrop_sel")]
    pub hard_drop_selected: usize,
    #[serde(rename = "lock_sel")]
    pub lock_effect_selected: usize,
    #[serde(rename = "lineclear_sel")]
    pub line_clear_selected: usize,
    #[serde(rename = "minitetsymb_sel")]
    pub mini_tetromino_symbols_selected: usize,
    #[serde(rename = "smalltetsymb_sel")]
    pub small_tetromino_symbols_selected: usize,
    #[serde(rename = "normsize_prev_limit")]
    pub normalsize_preview_limit: Option<NonZeroUsize>,
    #[serde(rename = "fps")]
    pub fps: ExtNonNegF64,
    #[serde(rename = "lockedtilecol_sel")]
    pub locked_tile_coloring_selected: usize,
    #[serde(rename = "s_hud")]
    pub show_main_hud: bool,
    #[serde(rename = "s_keybinds")]
    pub show_keybinds: bool,
    #[serde(rename = "s_buttons")]
    pub show_buttons: bool,
    #[serde(rename = "s_lockdelay")]
    pub show_lockdelay: bool,
    #[serde(rename = "s_shadow")]
    pub show_shadow: bool,
    #[serde(rename = "s_spawn")]
    pub show_spawn: bool,
    #[serde(rename = "s_grid")]
    pub show_grid: bool,
    #[serde(rename = "s_fps")]
    pub show_fps: bool,
}

pub fn graphics_settings_presets() -> SlotMachine<GraphicsSettings> {
    let slots = vec![
        ("Default".to_owned(), GraphicsSettings::default()),
        ("Guideline".to_owned(), GraphicsSettings::guideline()),
        ("Focus+".to_owned(), GraphicsSettings::extra_focus()),
        (
            "Terminal compatibility".to_owned(),
            GraphicsSettings::compatibility(),
        ),
        (
            "Elektronika 60".to_owned(),
            GraphicsSettings::elektronika_60(),
        ),
        ("I⠐⢷⠗ Braille".to_owned(), GraphicsSettings::braille()), // ⠺⡾⠂
        ("Minimal".to_owned(), GraphicsSettings::minimal()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Graphics".to_owned())
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 1,            // No color TODO: Revert this to idx=0
            tile_coloring_selected: 2,           // Okpalette
            tui_symbols_selected: 1,             // Unicode
            tile_symbols_selected: 1,            // Unicode
            hard_drop_selected: 1,               // ASCII particles
            lock_effect_selected: 3,             // Unicode pulse
            line_clear_selected: 11,             // Stardust
            mini_tetromino_symbols_selected: 1,  // Braille
            small_tetromino_symbols_selected: 1, // Blocks
            normalsize_preview_limit: Some(NonZeroUsize::MIN),
            fps: ExtNonNegF64::from(60),
            locked_tile_coloring_selected: 2, // Okpalette
            show_main_hud: true,
            show_lockdelay: false,
            show_keybinds: true,
            show_buttons: false,
            show_shadow: true,
            show_spawn: true,
            show_grid: false,
            show_fps: false,
        }
    }
}

impl GraphicsSettings {
    pub fn extra_focus() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 3,           // Standard
            tui_symbols_selected: 4,             // Unicode
            tile_symbols_selected: 1,            // Unicode
            hard_drop_selected: 0,               // None
            lock_effect_selected: 0,             // None
            line_clear_selected: 0,              // None (retain)
            mini_tetromino_symbols_selected: 1,  // Braille
            small_tetromino_symbols_selected: 1, // Blocks
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(60),
            locked_tile_coloring_selected: 0, // Monochrome
            show_main_hud: true,
            show_lockdelay: true,
            show_keybinds: false,
            show_buttons: true,
            show_shadow: true,
            show_spawn: true,
            show_grid: false,
            show_fps: false,
        }
    }

    pub fn guideline() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 3,           // Standard
            tui_symbols_selected: 2,             // Rounded Unicode
            tile_symbols_selected: 1,            // Unicode
            hard_drop_selected: 4,               // Solid beam Unicode
            lock_effect_selected: 3,             // Pulse Unicode
            line_clear_selected: 6,              // Clear inward
            mini_tetromino_symbols_selected: 1,  // Braille
            small_tetromino_symbols_selected: 1, // Blocks
            normalsize_preview_limit: Some(NonZeroUsize::new(4).unwrap()),
            fps: ExtNonNegF64::from(60),
            locked_tile_coloring_selected: 3, // Standard
            show_main_hud: true,
            show_lockdelay: false,
            show_keybinds: true,
            show_buttons: false,
            show_shadow: true,
            show_spawn: true,
            show_grid: false,
            show_fps: false,
        }
    }

    pub fn braille() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 3,           // Standard
            tui_symbols_selected: 5,             // Braille
            tile_symbols_selected: 2,            // Braille
            hard_drop_selected: 6,               // Braille helix
            lock_effect_selected: 4,             // Spiral Braille
            line_clear_selected: 14,             // Sparks Braille
            mini_tetromino_symbols_selected: 1,  // Braille
            small_tetromino_symbols_selected: 2, // Braille
            normalsize_preview_limit: Some(NonZeroUsize::new(4).unwrap()),
            fps: ExtNonNegF64::from(60),
            locked_tile_coloring_selected: 3, // Standard
            show_main_hud: true,
            show_lockdelay: false,
            show_keybinds: true,
            show_buttons: false,
            show_shadow: true,
            show_spawn: true,
            show_grid: false,
            show_fps: false,
        }
    }

    pub fn compatibility() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 1,           // ANSI
            tui_symbols_selected: 0,             // ASCII
            tile_symbols_selected: 0,            // ASCII
            hard_drop_selected: 1,               // ASCII particles
            lock_effect_selected: 2,             // ASCII transform
            line_clear_selected: 15,             // Sparks ASCII
            mini_tetromino_symbols_selected: 0,  // Letters
            small_tetromino_symbols_selected: 0, // ASCII
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(30),
            locked_tile_coloring_selected: 1, // ANSI
            show_main_hud: true,
            show_lockdelay: false,
            show_keybinds: true,
            show_buttons: false,
            show_shadow: true,
            show_spawn: true,
            show_grid: false,
            show_fps: false,
        }
    }

    pub fn elektronika_60() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 0,           // Monochrome
            tui_symbols_selected: 6,             // Elektronika 60
            tile_symbols_selected: 4,            // Elektronika 60
            hard_drop_selected: 0,               // None
            lock_effect_selected: 0,             // None
            line_clear_selected: 5,              // Clear left-to-right
            mini_tetromino_symbols_selected: 0,  // Letters
            small_tetromino_symbols_selected: 0, // ASCII
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(60),
            locked_tile_coloring_selected: 0, // Monochrome
            show_main_hud: true,
            show_lockdelay: false,
            show_keybinds: true,
            show_buttons: false,
            show_shadow: false,
            show_spawn: false,
            show_grid: true,
            show_fps: false,
        }
    }

    pub fn minimal() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 0,           // Monochrome
            tui_symbols_selected: 3,             // Borderless-Next/Hold Unicode
            tile_symbols_selected: 1,            // Unicode
            hard_drop_selected: 0,               // None
            lock_effect_selected: 0,             // None
            line_clear_selected: 0,              // None
            mini_tetromino_symbols_selected: 1,  // Braille
            small_tetromino_symbols_selected: 1, // Blocks
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(60),
            locked_tile_coloring_selected: 0, // Monochrome
            show_main_hud: false,
            show_lockdelay: false,
            show_keybinds: false,
            show_buttons: false,
            show_shadow: false,
            show_spawn: false,
            show_grid: true, // We make an exception here, because without anything it's hard to play but with grid this arguably already becomes a viable preset.
            show_fps: false,
        }
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(into = "String", try_from = "String")]
pub struct TileTexture(pub [char; 2]);

impl TileTexture {
    pub const EMPTY: Self = TileTexture([' '; 2]);
}

impl TryFrom<String> for TileTexture {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let tile = value
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(|x| format!("Error: {x:?}"))?;
        Ok(TileTexture(tile))
    }
}

// -- Serialization boilerplate --

pub trait UnwrapTileFromStr {
    fn tile(&self) -> TileTexture;
}

impl UnwrapTileFromStr for str {
    fn tile(&self) -> TileTexture {
        let tile = self.chars().collect::<Vec<char>>().try_into().unwrap();
        TileTexture(tile)
    }
}

impl From<TileTexture> for String {
    fn from(value: TileTexture) -> Self {
        value.0.iter().collect()
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum MaybeOverride<T> {
    Keep,
    Override(T),
}

impl<T> MaybeOverride<T> {
    pub fn unwrap_or(self, keep: T) -> T {
        match self {
            MaybeOverride::Keep => keep,
            MaybeOverride::Override(r#override) => r#override,
        }
    }
}
