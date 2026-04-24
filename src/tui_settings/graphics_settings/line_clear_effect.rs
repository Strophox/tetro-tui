use std::{num::NonZeroU8, time::Duration};

use falling_tetromino_engine::{InGameTime, TileID, WIDTH};

use crate::tui_settings::{
    Palette, SlotMachine,
    graphics_settings::{
        MaybeOverride::{self, Keep, Override},
        TileTexture, UnwrapTileFromStr,
    },
};

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum LineClearEffect {
    Inline(LineClearInlineEffect),
    Particle(LineClearParticleEffect),
}

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LineClearInlineEffect {
    #[serde(rename = "anim_idcs")]
    pub anim_indices: [usize; 2 * WIDTH],

    /// The last/max index that is animated.
    #[serde(rename = "anim_lastidx")]
    pub anim_lastidx: usize,

    /// Color animation ids.
    /// Note:
    /// - Empty vec means no recoloring (locked piece tile id).
    /// - `None` tile id falls back to locked piece tile id.
    #[serde(rename = "col_anim")]
    pub color_animation: Vec<MaybeOverride<TileID>>,
}

/// The formulas used to generate the momentum values:
/// * `xmm := xmm_init + xmm_rand ⋅ [-1..1(random)] + xmm_xpos ⋅ [-1..1(x position)]`
/// * `ymm := ymm_init + ymm_rand ⋅ [-1..1(random)]`
///
/// Formula used to generate the position at time:
/// * `pos = origin + momentum ⋅ Δtime + acceleration ⋅ (Δtime)² / 2`
#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LineClearParticleEffect {
    #[serde(rename = "dur_override")]
    pub duration_override: MaybeOverride<InGameTime>,

    /// Note:
    /// - Empty vec means no effect.
    /// - 'Empty'=space tile texture is automatically retextured to `air`.
    /// - `None` tile texture falls back to dropped piece tile texture.
    /// - `None` tile id falls back to locked piece tile id.
    #[serde(rename = "anim")]
    pub animation: Vec<(MaybeOverride<TileTexture>, MaybeOverride<TileID>)>,

    #[serde(rename = "accel")]
    pub acceleration: (f32, f32),
    #[serde(rename = "mm_base")]
    pub momentum_base: (f32, f32),
    #[serde(rename = "mm_rand")]
    pub momentum_rand: (f32, f32),
    #[serde(rename = "mm_xpos")]
    pub momentum_xpos: f32,
}

pub fn line_clear_effect_presets() -> SlotMachine<LineClearEffect> {
    let slots = vec![
        ("None".to_owned(), LineClearEffect::none()),
        ("Vanish instantly".to_owned(), LineClearEffect::instant()),
        ("Blink".to_owned(), LineClearEffect::blink()),
        ("Flash white".to_owned(), LineClearEffect::flash_white()),
        ("Left to right".to_owned(), LineClearEffect::left_to_right()),
        ("Clear inward".to_owned(), LineClearEffect::inward()),
        ("Burn outward".to_owned(), LineClearEffect::burn_outward()),
        ("Pop".to_owned(), LineClearEffect::pop()),
        ("Pop (higher)".to_owned(), LineClearEffect::pop_high()),
        (
            "Confetti (gratuitous)".to_owned(),
            LineClearEffect::confetti(),
        ),
        ("Stardust".to_owned(), LineClearEffect::stardust()),
        ("Blast".to_owned(), LineClearEffect::blast()),
        ("Sparks".to_owned(), LineClearEffect::sparks()),
        ("Sparks ASCII".to_owned(), LineClearEffect::sparks_ascii()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Lineclear".to_owned())
}

impl LineClearEffect {
    pub fn none() -> Self {
        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Override(Duration::ZERO),
            animation: Vec::new(),
            acceleration: (0.0, 0.0),
            momentum_base: (0.0, 0.0),
            momentum_rand: (0.0, 0.0),
            momentum_xpos: 0.0,
        })
    }

    pub fn instant() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [0; 2 * WIDTH],
            anim_lastidx: 1,
            color_animation: Vec::new(),
        })
    }

    // pub fn vanish() -> Self {
    //     LineClearEffect::Inline(LineClearInlineEffect {
    //         anim_indices: [1; 2 * Game::WIDTH],
    //         anim_lastidx: 10,
    //         color_animation: Vec::new(),
    //     })
    // }

    pub fn left_to_right() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
            ],
            anim_lastidx: 19,
            color_animation: vec![Override(Palette::WHITE)],
        })
    }

    pub fn inward() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
            anim_lastidx: 9,
            color_animation: vec![Override(Palette::WHITE)],
        })
    }

    pub fn burn_outward() -> Self {
        let color_animation = [255, 255, 1, 6, 4]
            .map(|n| Override(NonZeroU8::new(n).unwrap()))
            .into();

        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [
                18, 8, 16, 6, 14, 4, 12, 2, 10, 0, 1, 11, 3, 13, 5, 15, 7, 17, 9, 19,
            ],
            anim_lastidx: 19,
            color_animation,
        })
    }

    pub fn flash_white() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [1; 2 * WIDTH],
            anim_lastidx: 0,
            color_animation: vec![Override(Palette::WHITE), Keep, Override(Palette::WHITE)],
        })
    }

    pub fn blink() -> Self {
        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Keep,
            animation: [Keep, Override("  ".tile()), Keep, Override("  ".tile())]
                .map(|t| (t, Keep))
                .into(),
            acceleration: (0.0, 0.0),
            momentum_base: (0.0, 0.0),
            momentum_rand: (0.0, 0.0),
            momentum_xpos: 0.0,
        })
    }

    pub fn pop() -> Self {
        let mut animation = vec![(Keep, Keep); 8];
        animation[0] = (Keep, Override(Palette::WHITE));

        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Override(Duration::from_millis(1000)),
            animation,
            acceleration: (0.0, -200.0),
            momentum_base: (0.0, 30.0),
            momentum_rand: (10.0, 5.0),
            momentum_xpos: 75.0,
        })
    }

    pub fn pop_high() -> Self {
        let mut animation = vec![(Keep, Keep); 8];
        animation[0] = (Keep, Override(Palette::WHITE));

        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Override(Duration::from_millis(1000)),
            animation,
            acceleration: (0.0, -200.0),
            momentum_base: (0.0, 45.0),
            momentum_rand: (50.0, 5.0),
            momentum_xpos: 0.0,
        })
    }

    pub fn confetti() -> Self {
        let color_animation = [255, 1, 6, 4, 5, 7, 2, 3, 1, 6, 4, 5, 7, 2, 3]
            .map(|n| Override(NonZeroU8::new(n).unwrap()));
        let animation = color_animation
            .into_iter()
            .map(|recolor| (Keep, recolor))
            .collect();

        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Override(Duration::from_millis(1000)),
            animation,
            acceleration: (0.0, -200.0),
            momentum_base: (0.0, 55.0),
            momentum_rand: (55.0, 25.0),
            momentum_xpos: 45.0,
        })
    }

    pub fn blast() -> Self {
        let color_animation = [255, 1, 6, 4, 5].map(|n| Override(NonZeroU8::new(n).unwrap()));
        let animation = color_animation
            .into_iter()
            .map(|recolor| (Keep, recolor))
            .collect();

        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Override(Duration::from_millis(500)),
            animation,
            acceleration: (0.0, 10.0),
            momentum_base: (0.0, -45.0),
            momentum_rand: (50.0, 10.0),
            momentum_xpos: 50.0,
        })
    }

    pub fn stardust() -> Self {
        let color_animation = [255, 3, 2, 2, 7, 5].map(|n| Override(NonZeroU8::new(n).unwrap()));
        let animation = color_animation.map(|recolor| (Keep, recolor)).into();

        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Override(Duration::from_millis(450)),
            animation,
            acceleration: (650.0, 80.0),
            momentum_base: (0.0, 0.0),
            momentum_rand: (70.0, 10.0),
            momentum_xpos: 100.0,
        })
    }

    pub fn sparks() -> Self {
        let color_animation = [255, 2, 1, 6, 4, 5].map(|n| Override(NonZeroU8::new(n).unwrap()));
        let animation = color_animation.map(|recolor| (Keep, recolor)).into();

        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Override(Duration::from_millis(400)),
            animation,
            acceleration: (0.0, 0.0),
            momentum_base: (0.0, 0.0),
            momentum_rand: (60.0, 60.0),
            momentum_xpos: 100.0,
        })
    }

    pub fn sparks_ascii() -> Self {
        let tile_animation =
            ["@@", "$$", "##", "%%", "**", "++", "~~", ".."].map(|ss| Override(ss.tile()));
        let color_animation =
            [255, 3, 2, 1, 2, 7, 5, 4].map(|n| Override(NonZeroU8::new(n).unwrap()));
        let animation = tile_animation.into_iter().zip(color_animation).collect();

        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Override(Duration::from_millis(500)),
            animation,
            acceleration: (0.0, 0.0),
            momentum_base: (0.0, 40.0),
            momentum_rand: (0.0, 40.0),
            momentum_xpos: 100.0,
        })
    }
}
