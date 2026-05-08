use std::time::Duration;

use falling_tetromino_engine::{InGameTime, TileID};

use crate::tui_settings::{
    Palette, SlotMachine,
    graphics_settings::{
        MaybeOverride::{self, Keep, Override},
        TileTexture, UnwrapTileFromStr,
    },
};

#[serde_with::serde_as]
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct LockEffect {
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    #[serde(rename = "dur")]
    pub duration: InGameTime,

    /// Note:
    /// - Empty vec means no effect.
    /// - 'Empty'=space tile texture is automatically retextured to `air`.
    /// - `None` tile texture falls back to locked piece tile texture.
    /// - `None` tile id falls back to locked piece tile id.
    #[serde(rename = "anim")]
    pub animation: Vec<(MaybeOverride<TileTexture>, MaybeOverride<TileID>)>,
}

pub fn lock_effect_presets() -> SlotMachine<LockEffect> {
    let slots = vec![
        ("None".to_owned(), LockEffect::none()),
        ("Highlight white".to_owned(), LockEffect::color_white()),
        ("Transform ASCII".to_owned(), LockEffect::ascii_transform()),
        ("Pulse blocks UTF8".to_owned(), LockEffect::unicode_pulse()),
        ("Spiral Braille".to_owned(), LockEffect::braille()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Lock effect".to_owned())
}

impl LockEffect {
    pub fn none() -> Self {
        LockEffect {
            duration: Duration::ZERO,
            animation: Vec::new(),
        }
    }

    pub fn color_white() -> Self {
        LockEffect {
            duration: Duration::from_millis(125),
            animation: vec![(Keep, Override(Palette::WHITE))],
        }
    }

    pub fn ascii_transform() -> Self {
        LockEffect {
            duration: Duration::from_millis(175),
            animation: ["[]", "()", "{}", "<>", "=="]
                .map(|t| (Override(t.tile()), Override(Palette::WHITE)))
                .into(),
        }
    }

    pub fn unicode_pulse() -> Self {
        LockEffect {
            duration: Duration::from_millis(150),
            animation: ["██", "▓▓", "▒▒", "░░", "▒▒", "▓▓"]
                .map(|t| (Override(t.tile()), Override(Palette::WHITE)))
                .into(),
        }
    }

    pub fn braille() -> Self {
        LockEffect {
            duration: Duration::from_millis(200),
            animation: [
                /*"⠠⠂", "⢠⠃",*/ "⢀⠁", "⢈⡁", "⢊⡡", "⢎⡱", "⢮⡳", "⢾⡷",
            ]
            .map(|t| (Override(t.tile()), Override(Palette::WHITE)))
            .into(),
        }
    }
}
