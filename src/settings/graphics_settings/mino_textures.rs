use crate::settings::{graphics_settings::TileTexture, SlotMachine};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct MinoTextures {
    pub player: TileTexture,
    pub locked: TileTexture,
    pub shadow: TileTexture,
    pub air: TileTexture,
    pub slashed: TileTexture,
    pub crossed: TileTexture,
}

pub fn default_mino_textures_slots() -> SlotMachine<MinoTextures> {
    let slots = vec![
        ("Unicode".to_owned(), MinoTextures::unicode()),
        ("ASCII".to_owned(), MinoTextures::ascii()),
        ("Elektronika 60".to_owned(), MinoTextures::elektronika_60()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Mino textures".to_owned())
}

/*TODO:
- Mino texturing SLOT = ['Elektronika 60', 'ASCII', 'Unicode'] `Slots<MinoTextures>`
* <!--Not accessible in TUI-->
* In-play = ["██","▓▓","▒▒","░░","[]","##","::","▮▮","XX", "//", - `╳╱╲`, `X/\`]
* Locked = ^
* Shadow = ^
* Air = ^
* Slashed = ^
* Crossed = ^*/

impl MinoTextures {
    pub fn ascii() -> Self {
        todo!()
    }

    pub fn unicode() -> Self {
        todo!()
    }

    pub fn elektronika_60() -> Self {
        todo!()
    }
}
