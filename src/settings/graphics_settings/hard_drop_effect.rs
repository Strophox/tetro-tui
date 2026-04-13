use std::time::Duration;

use falling_tetromino_engine::{ExtNonNegF64, InGameTime, TileID};

use crate::settings::{
    graphics_settings::{QuickTileFromStr, TileTexture},
    Palette, SlotMachine,
};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct HardDropEffect {
    pub duration: InGameTime,
    /// Note:
    /// - Empty vec means no effect.
    /// - 'Empty'=space tile texture is automatically retextured to `air`.
    /// - `None` tile id falls back to dropped piece tile id.
    pub animation: Vec<(TileTexture, Option<TileID>)>,
    /// The extent to which the lifetime decays toward the top when the pieces are spawned.
    /// - 1.0 means the upmost particle will have the same (100% of the) lifetime as the bottommost particle.
    /// - 0.0 means the upmost particle will have 0% lifetime on spawn (and all inbetween scaled linearly).
    pub top_y_decay: ExtNonNegF64,
    // FIXME: Remove unused code or reconsider: A new toggle.
    //pub extend_to_top: bool,
}

pub fn default_hard_drop_effect_slots() -> SlotMachine<HardDropEffect> {
    let slots = vec![
        ("None".to_owned(), HardDropEffect::none()),
        (
            "ASCII particles".to_owned(),
            HardDropEffect::ascii_particles(),
        ),
        ("ASCII streak".to_owned(), HardDropEffect::ascii_streak()),
        ("ASCII beam".to_owned(), HardDropEffect::ascii_beam()),
        ("Block beam".to_owned(), HardDropEffect::block_beam()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Hard drop".to_owned())
}

impl HardDropEffect {
    pub fn none() -> Self {
        HardDropEffect {
            duration: Duration::ZERO,
            animation: Vec::new(),
            top_y_decay: ExtNonNegF64::MIN,
        }
    }

    pub fn ascii_particles() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(250),
            animation: ["@@", "$$", "##", "%%", "**", "++", "~~", ".."]
                .map(|ss| (ss.tile(), None))
                .into(),
            top_y_decay: 0.0.try_into().unwrap(),
        }
    }

    pub fn ascii_streak() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(250),
            animation: ["||", "¦¦", "::", ".."].map(|ss| (ss.tile(), None)).into(),
            top_y_decay: 0.5.try_into().unwrap(),
        }
    }

    pub fn ascii_beam() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(250),
            animation: ["||", "¦¦", "::", ".."]
                .map(|ss| (ss.tile(), Some(Palette::WHITE)))
                .into(),
            top_y_decay: 1.0.try_into().unwrap(),
        }
    }

    pub fn block_beam() -> Self {
        HardDropEffect {
            duration: Duration::from_millis(150),
            animation: ["▒▒", "▒▒", "▒▒", "▒▒", "░░", "░░", "  ", "░░"]
                .map(|ss| (ss.tile(), Some(Palette::WHITE)))
                .into(),
            top_y_decay: 1.0.try_into().unwrap(),
        }
    }
}
