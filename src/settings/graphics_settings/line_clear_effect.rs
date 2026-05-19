use std::time::Duration;

use crate::core_game_engine::{BOARD_WIDTH, InGameTime};

use crate::settings::{
    SlotMachine,
    graphics_settings::{
        MaybeOverride::{self, Keep, Override},
        TileTexture, UnwrapTileFromStr,
        tile_coloring::ColorID,
    },
};

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum LineClearEffect {
    Inline(LineClearInlineEffect),
    Particle(LineClearParticleEffect),
}

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LineClearInlineEffect {
    // 2 * W because 2 chars / tile.
    #[serde(rename = "anim_idcs")]
    pub anim_indices: [usize; 2 * BOARD_WIDTH],

    /// The last/max index that is animated.
    #[serde(rename = "anim_lastidx")]
    pub anim_lastidx: usize,

    /// Color animation ids.
    /// Note:
    /// - Empty vec means no recoloring (locked piece tile id).
    /// - `None` tile id falls back to locked piece tile id.
    #[serde(rename = "col_anim")]
    pub color_animation: Vec<MaybeOverride<ColorID>>,
    // FIXME: Currently, various effects are somewhat scuffed for graphics that use dual-colored tiles. E.g., an inline line clear effect that Overrides the color to be 'white' leads to weird visuals because it only recolors the glyphs of the original board tiles, even if those glyphs are very small (since the background color is mainly used for those dual-colored tiles).
    // Perhaps we should fix this somehow, e.g. by allowing a tile override. Alternatively, change the logic to re-color the background dynamically depending on whether the current graphics make actual use of the BG.
    // pub tile_override: MaybeOverride<(TileTexture, TileType)>
}

/// The formulas used to generate the momentum values:
/// * `xmm := xmm_init + xmm_rand ⋅ [-1..1(random)] + xmm_xpos ⋅ [-1..1(x position)]`
/// * `ymm := ymm_init + ymm_rand ⋅ [-1..1(random)]`
///
/// Formula used to generate the position at time:
/// * `pos = origin + momentum ⋅ Δtime + acceleration ⋅ (Δtime)² / 2`
// #[serde_with::serde_as] // Do **NOT** place this after #[derive(..)] !!
#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LineClearParticleEffect {
    // FIXME: Make these serde shenanigans work (also for other Duration fields): #[serde_as(as = "MaybeOverride<serde_with::DurationSecondsWithFrac<f64>>")]
    #[serde(rename = "dur_override")]
    pub duration_override: MaybeOverride<InGameTime>,

    /// Note:
    /// - Empty vec means no effect.
    /// - 'Empty'=space tile texture is automatically retextured to `air`.
    /// - `None` tile texture falls back to dropped piece tile texture.
    /// - `None` tile id falls back to locked piece tile id.
    #[serde(rename = "anim")]
    pub animation: Vec<(MaybeOverride<TileTexture>, MaybeOverride<ColorID>)>,

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
        (
            "Disappear halfway".to_owned(),
            LineClearEffect::vanish_delayed(),
        ),
        ("Disappear instantly".to_owned(), LineClearEffect::instant()),
        ("Blink".to_owned(), LineClearEffect::blink()),
        ("Flash white".to_owned(), LineClearEffect::flash_white()),
        (
            "Clear left-to-right".to_owned(),
            LineClearEffect::left_to_right(),
        ),
        ("Clear outward".to_owned(), LineClearEffect::outward()),
        ("White clear inward".to_owned(), LineClearEffect::inward()),
        ("Burn outward".to_owned(), LineClearEffect::burn_outward()),
        ("Pop".to_owned(), LineClearEffect::pop()),
        ("Pop (more)".to_owned(), LineClearEffect::pop_high()),
        (
            "Confetti (gratuitous)".to_owned(),
            LineClearEffect::confetti(),
        ),
        ("Stardust".to_owned(), LineClearEffect::stardust()),
        ("Blast".to_owned(), LineClearEffect::blast()),
        ("Sparks".to_owned(), LineClearEffect::sparks()),
        (
            "Sparks Braille".to_owned(),
            LineClearEffect::sparks_braille(),
        ),
        ("Sparks ASCII".to_owned(), LineClearEffect::sparks_ascii()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Line clear".to_owned())
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

    pub fn vanish_delayed() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [1; 2 * BOARD_WIDTH],
            anim_lastidx: 2,
            color_animation: Vec::new(),
        })
    }

    pub fn instant() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [0; 2 * BOARD_WIDTH],
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
            color_animation: vec![Keep],
        })
    }

    pub fn outward() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            anim_lastidx: 9,
            color_animation: vec![Keep],
        })
    }

    pub fn inward() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
            anim_lastidx: 9,
            color_animation: vec![Override(ColorID::WHITE)],
        })
    }

    pub fn burn_outward() -> Self {
        let color_animation = [
            ColorID::WHITE,
            ColorID::WHITE,
            ColorID::YELLOW,
            ColorID::ORANGE,
            ColorID::RED,
        ]
        .map(Override)
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
            anim_indices: [1; 2 * BOARD_WIDTH],
            anim_lastidx: 0,
            color_animation: vec![Override(ColorID::WHITE), Keep, Override(ColorID::WHITE)],
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
        animation[0] = (Keep, Override(ColorID::WHITE));

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
        animation[0] = (Keep, Override(ColorID::WHITE));

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
        let color_animation = [
            ColorID::WHITE,
            ColorID::YELLOW,
            ColorID::ORANGE,
            ColorID::RED,
            ColorID::PURPLE,
            ColorID::BLUE,
            ColorID::CYAN,
            ColorID::GREEN,
            ColorID::YELLOW,
            ColorID::ORANGE,
            ColorID::RED,
            ColorID::PURPLE,
            ColorID::BLUE,
            ColorID::CYAN,
            ColorID::GREEN,
        ]
        .map(Override);
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
        let color_animation = [
            ColorID::WHITE,
            ColorID::YELLOW,
            ColorID::ORANGE,
            ColorID::RED,
            ColorID::PURPLE,
        ]
        .map(Override);
        let animation = color_animation
            .into_iter()
            .map(|recolor| (Keep, recolor))
            .collect();

        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Override(Duration::from_millis(500)),
            animation,
            acceleration: (0.0, 10.0),
            momentum_base: (0.0, -40.0),
            momentum_rand: (50.0, 10.0),
            momentum_xpos: 100.0,
        })
    }

    pub fn stardust() -> Self {
        let color_animation = [
            ColorID::WHITE,
            ColorID::GREEN,
            ColorID::CYAN,
            ColorID::CYAN,
            ColorID::BLUE,
            ColorID::PURPLE,
        ]
        .map(Override);
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
        let color_animation = [
            ColorID::WHITE,
            ColorID::CYAN,
            ColorID::YELLOW,
            ColorID::ORANGE,
            ColorID::RED,
            ColorID::PURPLE,
        ]
        .map(Override);
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

    pub fn sparks_braille() -> Self {
        let tile_animation = ["⢾⡷", "⡱⢎", "⡡⢊", "⡁⢈", "⡀⠈"].map(|ss| Override(ss.tile()));
        let color_animation = [
            ColorID::WHITE,
            ColorID::YELLOW,
            ColorID::GREEN,
            ColorID::CYAN,
            ColorID::BLUE,
            ColorID::PURPLE,
            ColorID::RED,
        ]
        .map(Override);
        let animation = tile_animation.into_iter().zip(color_animation).collect();

        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Override(Duration::from_millis(300)),
            animation,
            acceleration: (0.0, 0.0),
            momentum_base: (0.0, 40.0),
            momentum_rand: (0.0, 40.0),
            momentum_xpos: 100.0,
        })
    }

    pub fn sparks_ascii() -> Self {
        let tile_animation =
            ["@@", "$$", "##", "%%", "**", "++", "~~", ".."].map(|ss| Override(ss.tile()));
        let color_animation = [
            ColorID::WHITE,
            ColorID::GREEN,
            ColorID::CYAN,
            ColorID::YELLOW,
            ColorID::CYAN,
            ColorID::BLUE,
            ColorID::PURPLE,
            ColorID::RED,
        ]
        .map(Override);
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
