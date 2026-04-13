use falling_tetromino_engine::{InGameTime, TileID};

use crate::settings::{graphics_settings::TileTexture, SlotMachine};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct LockEffect {
    pub duration: InGameTime,
    /// Note:
    /// - Empty (space) tile texture is automatically retextured to `air`.
    /// - `None` tile texture falls back to dropped piece tile texture.
    /// - `None` tile id falls back to dropped piece tile id.
    pub animation: Vec<(Option<TileTexture>, Option<TileID>)>,
}

pub fn default_lock_effect_slots() -> SlotMachine<LockEffect> {
    let slots = vec![
        ("None".to_owned(), LockEffect::none()),
        ("ASCII transform".to_owned(), LockEffect::ascii_transform()),
        (
            "Unicode white pulse".to_owned(),
            LockEffect::unicode_pulse(),
        ),
        ("White pulse".to_owned(), LockEffect::white_pulse()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Lock effect".to_owned())
}

/*TODO:
- Lock effect SLOT = ['None', 'ASCII', 'Unicode'] `Slots<LockEffect>`
* <!--Not accessible in TUI-->
* Effect duration = [200ms, ...]
* Color = [None, Some(White), ...]
* Mino animation = [None, Some("(){}<>"), Some("██▓▓▒▒░░▒▒▓▓")]*/

impl LockEffect {
    pub fn none() -> Self {
        todo!()
    }

    pub fn ascii_transform() -> Self {
        todo!()
    }

    pub fn unicode_pulse() -> Self {
        todo!()
    }

    pub fn white_pulse() -> Self {
        todo!()
    }
}
