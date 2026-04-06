use crate::settings::SlotMachine;

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
            glyphset: Glyphset::ASCII,
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
            glyphset: Glyphset::Elektronika_60,
            shadow_piece: false,
            button_state: false,
            show_fps: false,
        }
    }
}
