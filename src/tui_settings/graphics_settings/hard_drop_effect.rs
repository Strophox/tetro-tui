use std::time::Duration;

use falling_tetromino_engine::{InGameTime, TileID};

use crate::tui_settings::{
    graphics_settings::{QuickTileFromStr, TileTexture},
    Palette, SlotMachine,
};

type AnimateHardDrop = Vec<(TileTexture, Option<TileID>)>;

#[serde_with::serde_as]
#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HardDropEffect {
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    #[serde(rename = "dur")]
    pub duration: InGameTime,

    /// Note:
    /// - Empty vec means no effect.
    /// - 'Empty'=space tile texture is automatically retextured to `air`.
    /// - `None` tile id falls back to dropped piece tile id.
    #[serde(rename = "anim")]
    pub animation: AnimateHardDrop,

    /// The extent to which the lifetime decays faster toward the top when the pieces are spawned.
    /// - 1.0 means the upmost particle will have 100% of its `normalized_height` scaling.
    /// - 0.5 means the upmost particle will have 50% of its `normalized_height` scaling, which means it survives twice as long.
    /// - 2.0 means the upmost particle will have 200% of its `normalized_height` scaling, which means it survives half as long.
    #[serde(rename = "decay")]
    pub y_decay: f32,
}

pub fn default_hard_drop_effect_slots() -> SlotMachine<HardDropEffect> {
    let slots = vec![
        ("None".to_owned(), HardDropEffect::none()),
        (
            "Particles ASCII".to_owned(),
            HardDropEffect::ascii_particles(),
        ),
        ("Streak ASCII".to_owned(), HardDropEffect::ascii_streak()),
        ("Beam ASCII".to_owned(), HardDropEffect::ascii_beam()),
        ("Beam".to_owned(), HardDropEffect::block_beam()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Hard drop".to_owned())
}

impl HardDropEffect {
    pub fn none() -> Self {
        HardDropEffect {
            duration: Duration::ZERO,
            animation: Vec::new(),
            y_decay: 0.0,
        }
    }

    pub fn ascii_particles() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(150),
            animation: ["@@", "$$", /*"##", */ "%%", "**", "++", "~~", ".."]
                .map(|ss| (ss.tile(), None))
                .into(),
            y_decay: 0.75,
        }
    }

    pub fn ascii_streak() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(150),
            animation: ["||", "¦¦", "::", ".."].map(|ss| (ss.tile(), None)).into(),
            y_decay: 1.0,
        }
    }

    pub fn ascii_beam() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(150),
            animation: ["||", "¦¦", "::", ".."].map(|ss| (ss.tile(), None)).into(),
            y_decay: 0.0,
        }
    }

    pub fn block_beam() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(67),
            animation: ["▒▒", "░░", "░░"]
                .map(|ss| (ss.tile(), Some(Palette::WHITE)))
                .into(),
            y_decay: 0.0,
        }
    }
}
