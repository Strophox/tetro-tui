use falling_tetromino_engine::{ExtNonNegF64, InGameTime, TileID};

use crate::settings::{graphics_settings::TileTexture, SlotMachine};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct HardDropEffect {
    pub duration: InGameTime,
    /// Note that empty (space) tile texture is automatically retextured to `air`.
    pub animation: Vec<(Option<TileTexture>, Option<TileID>)>,
    /// The extent to which the lifetime decays toward the top when the pieces are spawned.
    /// - 1.0 means the upmost particle will have the same (100% of the) lifetime as the bottommost particle.
    /// - 0.0 means the upmost particle will have 0% lifetime on spawn (and all inbetween scaled linearly).
    pub y_decay: ExtNonNegF64,
    // FIXME: Remove unused code or reconsider: A new toggle.
    //pub extend_to_top: bool,
}

pub fn default_hard_drop_effect_slots() -> SlotMachine<HardDropEffect> {
    let slots = vec![
        ("None".to_owned(), HardDropEffect::none()),
        (
            "ASCII particles".to_owned(),
            HardDropEffect::ascii_particle(),
        ),
        ("ASCII streak".to_owned(), HardDropEffect::ascii_streak()),
        ("ASCII beam".to_owned(), HardDropEffect::ascii_beam()),
        ("Block beam".to_owned(), HardDropEffect::block_beam()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Hard drop".to_owned())
}

/*TODO:
- Hard drop effect SLOT = ['None', 'ASCII particle', 'ASCII streak', 'ASCII beam', 'Unicode beam'] `Slots<HardDropEffect>`
* <!--Not accessible in TUI-->
* Effect duration = [200ms, ...]
* Color = [None, Some(White), ...]
* Mino animation = "@@$$##%%**++~~" - "||¦¦::.." - "▒▒▒▒░░░░  ░░  ░░"
* Mino animation delay pattern = 0.00x, 1.00äx*/

impl HardDropEffect {
    pub fn none() -> Self {
        todo!()
    }

    pub fn ascii_particle() -> Self {
        todo!()
    }

    pub fn ascii_streak() -> Self {
        todo!()
    }

    pub fn ascii_beam() -> Self {
        todo!()
    }

    pub fn block_beam() -> Self {
        todo!()
    }
}
