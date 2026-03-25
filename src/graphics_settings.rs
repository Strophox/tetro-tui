use crate::application::SlotMachine;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
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
    pub palette_pick: usize,
    pub lockpalette_pick: usize,
    pub glyphset: Glyphset,
    pub show_effects: bool,
    pub lineclear_style: u8,
    pub show_shadow_piece: bool,
    pub show_button_state: bool,
    pub game_fps: f64,
    pub show_fps: bool,
}

pub fn default_graphics_slots() -> SlotMachine<GraphicsSettings> {
    let slots = vec![
        ("Default".to_owned(), GraphicsSettings::default()),
        ("Focus+".to_owned(), GraphicsSettings::extra_focus()),
        ("Guideline".to_owned(), GraphicsSettings::guideline()),
        ("High Compat.".to_owned(), GraphicsSettings::compatibility()),
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
            palette_pick: 3,
            lockpalette_pick: 3,
            show_effects: true,
            lineclear_style: 0,
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
            palette_pick: 2,
            lockpalette_pick: 0,
            show_effects: false,
            lineclear_style: 0,
            game_fps: 60.0,
            glyphset: Glyphset::Unicode,
            show_shadow_piece: true,
            show_button_state: false,
            show_fps: false,
        }
    }

    pub fn guideline() -> Self {
        Self {
            glyphset: Glyphset::Unicode,
            palette_pick: 2,
            lockpalette_pick: 2,
            show_effects: true,
            lineclear_style: 0,
            show_shadow_piece: true,
            show_button_state: false,
            game_fps: 60.0,
            show_fps: false,
        }
    }

    pub fn compatibility() -> Self {
        Self {
            palette_pick: 1,
            lockpalette_pick: 1,
            show_effects: true,
            lineclear_style: 0,
            game_fps: 30.0,
            glyphset: Glyphset::ASCII,
            show_shadow_piece: true,
            show_button_state: false,
            show_fps: false,
        }
    }

    pub fn elektronika_60() -> Self {
        Self {
            palette_pick: 0,
            lockpalette_pick: 0,
            show_effects: true,
            lineclear_style: 0,
            game_fps: 24.0,
            glyphset: Glyphset::Elektronika_60,
            show_shadow_piece: false,
            show_button_state: false,
            show_fps: false,
        }
    }
}
