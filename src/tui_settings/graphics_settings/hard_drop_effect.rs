use std::time::Duration;

use falling_tetromino_engine::{InGameTime, TileID};

use crate::tui_settings::{
    graphics_settings::{
        MaybeOverride::{self, Keep, Override},
        TileTexture, UnwrapTileFromStr,
    },
    Palette, SlotMachine,
};

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
    pub animation: Vec<(TileTexture, MaybeOverride<TileID>)>,

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
            "Particle trail ASCII".to_owned(),
            HardDropEffect::particle_trail_ascii(),
        ),
        (
            "Streak trail ASCII".to_owned(),
            HardDropEffect::streak_trail_ascii(),
        ),
        (
            "Streak beam ASCII".to_owned(),
            HardDropEffect::streak_beam_ascii(),
        ),
        (
            "Solid beam Unicode".to_owned(),
            HardDropEffect::solid_beam_unicode(),
        ),
        (
            "White beam Unicode".to_owned(),
            HardDropEffect::white_beam_unicode(),
        ),
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

    pub fn particle_trail_ascii() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(150),
            animation: ["@@", "$$", /*"##", */ "%%", "**", "++", "~~", ".."]
                .map(|ss| (ss.tile(), Keep))
                .into(),
            y_decay: 0.75,
        }
    }

    pub fn streak_trail_ascii() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(150),
            animation: ["||", "¦¦", "::", ".."].map(|ss| (ss.tile(), Keep)).into(),
            y_decay: 1.0,
        }
    }

    pub fn streak_beam_ascii() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(150),
            animation: ["||", "¦¦", "::", ".."].map(|ss| (ss.tile(), Keep)).into(),
            y_decay: 0.25,
        }
    }

    pub fn solid_beam_unicode() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(75),
            animation: ["▒▒", "░░"].map(|ss| (ss.tile(), Keep)).into(),
            y_decay: 0.00,
        }
    }

    pub fn white_beam_unicode() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(75),
            animation: ["  ", "░░"]
                .map(|ss| (ss.tile(), Override(Palette::WHITE)))
                .into(),
            y_decay: 0.00,
        }
    }
}
