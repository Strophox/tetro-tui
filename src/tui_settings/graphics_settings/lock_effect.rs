use std::time::Duration;

use falling_tetromino_engine::{InGameTime, TileID};

use crate::tui_settings::{
    graphics_settings::{QuickTileFromStr, TileTexture},
    Palette, SlotMachine,
};

type AnimateLock = Vec<(Option<TileTexture>, Option<TileID>)>;

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
    pub animation: AnimateLock,
}

pub fn default_lock_effect_slots() -> SlotMachine<LockEffect> {
    let slots = vec![
        ("None".to_owned(), LockEffect::none()),
        ("ASCII transform".to_owned(), LockEffect::ascii_transform()),
        ("Unicode pulse".to_owned(), LockEffect::unicode_pulse()),
        ("Color white".to_owned(), LockEffect::color_white()),
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

    pub fn ascii_transform() -> Self {
        LockEffect {
            duration: Duration::from_millis(150),
            animation: ["()", "{}", "<>"]
                .map(|t| (Some(t.tile()), Some(Palette::WHITE)))
                .into(),
        }
    }

    pub fn unicode_pulse() -> Self {
        LockEffect {
            duration: Duration::from_millis(150),
            animation: ["██", "▓▓", "▒▒", "░░", "▒▒", "▓▓"]
                .map(|t| (Some(t.tile()), Some(Palette::WHITE)))
                .into(),
        }
    }

    pub fn color_white() -> Self {
        LockEffect {
            duration: Duration::from_millis(150),
            animation: vec![(None, Some(Palette::WHITE))],
        }
    }
}
