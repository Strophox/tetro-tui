use std::time::Duration;

use crate::core_game_engine::InGameTime;

use crate::settings::{
    SlotMachine,
    graphics_settings::{
        MaybeOverride::{self, Keep, Override},
        TileTexture, UnwrapTileFromStr,
        tile_coloring::ColorID,
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
    pub animation: Vec<(MaybeOverride<TileTexture>, MaybeOverride<ColorID>)>,
}

pub fn lock_effect_presets() -> SlotMachine<LockEffect> {
    let slots = vec![
        ("None".to_owned(), LockEffect::none()),
        ("Flash white".to_owned(), LockEffect::color_white()),
        (
            "Transforming ASCII".to_owned(),
            LockEffect::ascii_transform(),
        ),
        ("Pulsing block UTF8".to_owned(), LockEffect::unicode_pulse()),
        ("Spiraling Braille".to_owned(), LockEffect::braille()),
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
            animation: vec![(Keep, Override(ColorID::WHITE))],
        }
    }

    pub fn ascii_transform() -> Self {
        let mut tileanim = ["[]", "()", "{}", "<>", "=="].map(|t| (Override(t.tile()), Keep));
        tileanim[0].1 = Override(ColorID::WHITE);

        LockEffect {
            duration: Duration::from_millis(175),
            animation: tileanim.into(),
        }
    }

    pub fn unicode_pulse() -> Self {
        let tileanim = ["██", "▓▓", "▒▒", "░░", "▒▒", "▓▓"]
            .map(|t| (Override(t.tile()), Override(ColorID::WHITE)));

        LockEffect {
            duration: Duration::from_millis(150),
            animation: tileanim.into(),
        }
    }

    pub fn braille() -> Self {
        let tileanim = [
            /*"⠠⠂", "⢠⠃",*/ "⢀⠁", "⢈⡁", "⢊⡡", "⢎⡱", "⢮⡳", "⢾⡷",
        ]
        .map(|t| (Override(t.tile()), Keep));

        LockEffect {
            duration: Duration::from_millis(200),
            animation: tileanim.into(),
        }
    }
}
