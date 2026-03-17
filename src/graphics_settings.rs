#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum Glyphset {
    #[allow(non_camel_case_types)]
    Elektronika_60,
    #[allow(clippy::upper_case_acronyms)]
    ASCII,
    Unicode,
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphicsSettings {
    pub palette_active: usize,
    pub palette_active_lockedtiles: usize,
    pub glyphset: Glyphset,
    pub show_effects: bool,
    pub blindfolded: bool,
    pub show_shadow_piece: bool,
    pub show_button_state: bool,
    pub game_fps: f64,
    pub show_fps: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            glyphset: Glyphset::Unicode,
            palette_active: 3,
            palette_active_lockedtiles: 3,
            show_effects: true,
            blindfolded: false,
            show_shadow_piece: true,
            show_button_state: false,
            game_fps: 30.0,
            show_fps: false,
        }
    }
}

impl GraphicsSettings {
    pub fn extra_focus() -> Self {
        Self {
            palette_active: 2,
            palette_active_lockedtiles: 0,
            show_effects: false,
            game_fps: 60.0,
            glyphset: Glyphset::Unicode,
            blindfolded: false,
            show_shadow_piece: true,
            show_button_state: false,
            show_fps: false,
            
        }
    }

    pub fn guideline() -> Self {
        Self {
            glyphset: Glyphset::Unicode,
            palette_active: 2,
            palette_active_lockedtiles: 2,
            show_effects: true,
            blindfolded: false,
            show_shadow_piece: true,
            show_button_state: false,
            game_fps: 60.0,
            show_fps: false,
        }
    }
    
    pub fn compatibility() -> Self {
        Self {
            palette_active: 1,
            palette_active_lockedtiles: 1,
            show_effects: true,
            game_fps: 30.0,
            glyphset: Glyphset::ASCII,
            blindfolded: false,
            show_shadow_piece: true,
            show_button_state: false,
            show_fps: false,
            
        }
    }
    
    pub fn elektronika_60() -> Self {
        Self {
            palette_active: 0,
            palette_active_lockedtiles: 0,
            show_effects: true,
            game_fps: 30.0,
            glyphset: Glyphset::Elektronika_60,
            blindfolded: false,
            show_shadow_piece: false,
            show_button_state: false,
            show_fps: false,
        }
    }
}

