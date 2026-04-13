use falling_tetromino_engine::{InGameTime, TileID};

use crate::settings::{graphics_settings::TileTexture, SlotMachine};

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum LineClearEffect {
    Inline {
        style: InlineClearStyle,
        /// Note:
        /// - `None` tile id falls back to dropped piece tile id.
        animation: Vec<Option<TileID>>,
    },
    // FIXME: Check.
    /// The formulas used to generate the momentum values:
    /// => xmm := xmm_init + xmm_rand ⋅ [-1..1(random)] + xmm_xpos ⋅ [-1..1(x position)]
    /// => ymm := ymm_init + ymm_rand ⋅ [0..1(random)]
    /// Formula used to generate the position at time:
    /// => pos = origin + momentum ⋅ Δtime + acceleration ⋅ (Δtime)² / 2
    MinoParticles {
        duration: Option<InGameTime>,
        /// Note:
        /// - Empty (space) tile texture is automatically retextured to `air`.
        /// - `None` tile texture falls back to dropped piece tile texture.
        /// - `None` tile id falls back to dropped piece tile id.
        animation: Vec<(Option<TileTexture>, Option<TileID>)>,
        acceleration: (f32, f32),
        momentum_base: (f32, f32),
        x_momentum_rand_m1p1: f32,
        y_momentum_rand_p0p1: f32,
        x_momentum_xpos_m1p1: f32,
    },
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum InlineClearStyle {
    Retain,
    Vacate,
    Inward,
    Outward,
    Leftward,
    Rightward,
}

pub fn default_line_clear_effect_slots() -> SlotMachine<LineClearEffect> {
    let slots = vec![
        ("None /retain".to_owned(), LineClearEffect::retain()),
        ("None /vacate".to_owned(), LineClearEffect::vacate()),
        ("Left-to-right".to_owned(), LineClearEffect::left_to_right()),
        ("Inward".to_owned(), LineClearEffect::inward()),
        (
            "Outward (rainbow)".to_owned(),
            LineClearEffect::outward_rainbow(),
        ),
        ("Blink".to_owned(), LineClearEffect::blink()),
        ("Flash (white)".to_owned(), LineClearEffect::flash_white()),
        ("Pop minos".to_owned(), LineClearEffect::pop()),
        (
            "Pop minos (chaotic)".to_owned(),
            LineClearEffect::pop_chaotic(),
        ),
        (
            "Pop minos (ASCII fade)".to_owned(),
            LineClearEffect::pop_ascii_fade(),
        ),
        ("Blast minos".to_owned(), LineClearEffect::blast()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Lineclear".to_owned())
}

/*- TODO Line clear effect SLOT = ['None', 'Left->Right', 'Right->Left', 'Interleaved', 'Blink']
* <!--Not accessible in TUI-->
* Mino animation = "██" - "██  " - "@@$$##%%**++~~" - "||¦¦::.." - "░░  ░░  ░░  "
* Mino momentum = TODO up+outward, up+random, down+outward, random
* Mino acceleration = ...
* Mino animation delay pattern = TODO Constant - L2R - R2L - interleave LR/RL

Inline?
* Inward, Outward, Leftward, Rightward
* AnimateMinos { [""] use "" for underlying }
Particle
* Pop struct MinoParticle {
animation: String,
creation_time: InGameTime,
origin: (usize, usize),
momentum: (f32, f32),
acceleration: (f32, f32),
actually_render: bool,
tile_id: TileID,
}
*/

impl LineClearEffect {
    pub fn retain() -> Self {
        todo!()
    }

    pub fn vacate() -> Self {
        todo!()
    }

    pub fn left_to_right() -> Self {
        todo!()
    }

    pub fn inward() -> Self {
        todo!()
    }

    pub fn outward_rainbow() -> Self {
        todo!()
    }

    pub fn blink() -> Self {
        todo!()
    }

    pub fn flash_white() -> Self {
        todo!()
    }

    pub fn pop() -> Self {
        todo!()
    }

    pub fn pop_chaotic() -> Self {
        todo!()
    }

    pub fn pop_ascii_fade() -> Self {
        todo!()
    }

    pub fn blast() -> Self {
        todo!()
    }
}
