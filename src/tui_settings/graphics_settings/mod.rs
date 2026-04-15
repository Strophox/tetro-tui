use std::num::NonZeroUsize;

use falling_tetromino_engine::ExtNonNegF64;

use crate::tui_settings::SlotMachine;

pub mod hard_drop_effect;
pub mod line_clear_effect;
pub mod lock_effect;
pub mod mini_tet_style;
pub mod mino_textures;
pub mod palette;
pub mod small_tet_style;
pub mod tui_style;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GraphicsSettings {
    #[serde(rename = "palette")]
    pub palette_picked: usize,
    #[serde(rename = "tuistyle")]
    pub tui_style_picked: usize,
    #[serde(rename = "minotextures")]
    pub mino_textures_picked: usize,
    #[serde(rename = "harddrop")]
    pub hard_drop_picked: usize,
    #[serde(rename = "lock")]
    pub lock_effect_picked: usize,
    #[serde(rename = "lineclear")]
    pub line_clear_picked: usize,
    #[serde(rename = "minitet")]
    pub mini_tet_picked: usize,
    #[serde(rename = "smalltet")]
    pub small_tet_picked: usize,
    #[serde(rename = "normalszprev")]
    pub normalsize_preview_limit: Option<NonZeroUsize>,
    #[serde(rename = "fps")]
    pub fps: ExtNonNegF64,
    #[serde(rename = "boardpalette")]
    pub boardpalette_picked: usize,
    #[serde(rename = "s_hud")]
    pub show_main_hud: bool,
    #[serde(rename = "s_keybinds")]
    pub show_keybinds: bool,
    #[serde(rename = "s_buttons")]
    pub show_buttons: bool,
    #[serde(rename = "s_shadow")]
    pub show_shadow: bool,
    #[serde(rename = "s_spawn")]
    pub show_spawn: bool,
    #[serde(rename = "s_grid")]
    pub show_grid: bool,
    #[serde(rename = "s_fps")]
    pub show_fps: bool,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(into = "String", try_from = "String")]
pub struct TileTexture(pub [char; 2]);

pub fn default_graphics_slots() -> SlotMachine<GraphicsSettings> {
    let slots = vec![
        ("Default".to_owned(), GraphicsSettings::default()),
        ("Focus+".to_owned(), GraphicsSettings::extra_focus()),
        ("Guideline".to_owned(), GraphicsSettings::guideline()),
        (
            "Max terminal compatibility".to_owned(),
            GraphicsSettings::compatibility(),
        ),
        (
            "Elektronika 60".to_owned(),
            GraphicsSettings::elektronika_60(),
        ),
        ("Blank slate".to_owned(), GraphicsSettings::blank_slate()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Graphics".to_owned())
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        GraphicsSettings {
            palette_picked: 3,       // Okpalette
            tui_style_picked: 1,     // Unicode
            mino_textures_picked: 1, // Unicode
            hard_drop_picked: 1,     // ASCII particles
            lock_effect_picked: 2,   // Unicode pulse
            line_clear_picked: 10,   // Blast
            mini_tet_picked: 1,      // Braille
            small_tet_picked: 1,     // Blocks
            normalsize_preview_limit: Some(NonZeroUsize::MIN),
            fps: ExtNonNegF64::from(60),
            boardpalette_picked: 3, // Okpalette
            show_main_hud: true,
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
            palette_picked: 2,       // Standard
            tui_style_picked: 1,     // Unicode
            mino_textures_picked: 1, // Unicode
            hard_drop_picked: 0,     // None
            lock_effect_picked: 0,   // None
            line_clear_picked: 1,    // None (vacate)
            mini_tet_picked: 1,      // Braille
            small_tet_picked: 1,     // Blocks
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(60),
            boardpalette_picked: 0, // Monochrome
            show_main_hud: true,
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
            palette_picked: 2,       // Standard
            tui_style_picked: 1,     // Unicode
            mino_textures_picked: 1, // Unicode
            hard_drop_picked: 1,     // ASCII particles
            lock_effect_picked: 2,   // Unicode pulse
            line_clear_picked: 7,    // Mino pop
            mini_tet_picked: 1,      // Braille
            small_tet_picked: 1,     // Blocks
            normalsize_preview_limit: Some(NonZeroUsize::MIN),
            fps: ExtNonNegF64::from(60),
            boardpalette_picked: 2, // Standard
            show_main_hud: true,
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
            palette_picked: 1,       // ANSI
            tui_style_picked: 0,     // ASCII
            mino_textures_picked: 0, // ASCII
            hard_drop_picked: 1,     // ASCII particles
            lock_effect_picked: 1,   // ASCII transform
            line_clear_picked: 9,    // Mino pop
            mini_tet_picked: 0,      // Letters
            small_tet_picked: 0,     // ASCII
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(30),
            boardpalette_picked: 1, // ANSI
            show_main_hud: true,
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
            palette_picked: 0,       // Monochrome
            tui_style_picked: 2,     // Elektronika 60
            mino_textures_picked: 2, // Elektronika 60
            hard_drop_picked: 0,     // None
            lock_effect_picked: 0,   // None
            line_clear_picked: 2,    // Left-to-right
            mini_tet_picked: 0,      // Letters
            small_tet_picked: 0,     // ASCII
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(60),
            boardpalette_picked: 0, // ANSI
            show_main_hud: true,
            show_keybinds: true,
            show_buttons: false,
            show_shadow: false,
            show_spawn: false,
            show_grid: true,
            show_fps: false,
        }
    }

    pub fn blank_slate() -> Self {
        GraphicsSettings {
            palette_picked: 0,       // Monochrome
            tui_style_picked: 1,     // Unicode
            mino_textures_picked: 1, // Unicode
            hard_drop_picked: 0,     // None
            lock_effect_picked: 0,   // None
            line_clear_picked: 0,    // None
            mini_tet_picked: 0,      // Letters
            small_tet_picked: 1,     // Blocks
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(60),
            boardpalette_picked: 0, // Monochrome
            show_main_hud: false,
            show_keybinds: false,
            show_buttons: false,
            show_shadow: false,
            show_spawn: false,
            show_grid: false,
            show_fps: false,
        }
    }
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

/* FIXME: Does ANYONE know what the error means that results from replacing the TryFrom<String> above with this???
impl<S: AsRef<str>> TryFrom<S> for TileTexture {
    type Error = String;

    fn try_from(value: S) -> Result<Self, Self::Error> {
        let tile = value.as_ref()
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(|x| format!("Error: {x:?}"))?;
        Ok(TileTexture(tile))
    }
}*/

pub trait QuickTileFromStr {
    fn tile(&self) -> TileTexture;
}

impl QuickTileFromStr for str {
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
