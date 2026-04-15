use std::{num::NonZeroU8, time::Duration};

use falling_tetromino_engine::{Game, InGameTime, TileID};

use crate::tui_settings::{
    graphics_settings::{QuickTileFromStr, TileTexture},
    Palette, SlotMachine,
};

type AnimateLineClearTile = Vec<(Option<TileTexture>, Option<TileID>)>;

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum LineClearEffect {
    Inline(LineClearInlineEffect),
    Particle(LineClearParticleEffect),
}

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LineClearInlineEffect {
    #[serde(rename = "anim_idcs")]
    pub anim_indices: [usize; 2 * Game::WIDTH],

    /// The last/max index that is animated.
    #[serde(rename = "anim_lastidx")]
    pub anim_lastidx: usize,

    /// Color animation ids.
    /// Note:
    /// - Empty vec means no recoloring (locked piece tile id).
    /// - `None` tile id falls back to locked piece tile id.
    #[serde(rename = "col_anim")]
    pub color_animation: Vec<Option<TileID>>,
}

/// The formulas used to generate the momentum values:
/// * `xmm := xmm_init + xmm_rand ⋅ [-1..1(random)] + xmm_xpos ⋅ [-1..1(x position)]`
/// * `ymm := ymm_init + ymm_rand ⋅ [-1..1(random)]`
///
/// Formula used to generate the position at time:
/// * `pos = origin + momentum ⋅ Δtime + acceleration ⋅ (Δtime)² / 2`
#[serde_with::serde_as]
#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LineClearParticleEffect {
    #[serde_as(as = "Option<serde_with::DurationSecondsWithFrac<f64>>")]
    #[serde(rename = "dur_override")]
    pub duration_override: Option<InGameTime>,

    /// Note:
    /// - Empty vec means no effect.
    /// - 'Empty'=space tile texture is automatically retextured to `air`.
    /// - `None` tile texture falls back to dropped piece tile texture.
    /// - `None` tile id falls back to locked piece tile id.
    #[serde(rename = "anim")]
    pub animation: AnimateLineClearTile,

    #[serde(rename = "accel")]
    pub acceleration: (f32, f32),
    #[serde(rename = "mm_base")]
    pub momentum_base: (f32, f32),
    #[serde(rename = "mm_rand")]
    pub momentum_rand: (f32, f32),
    #[serde(rename = "mm_xpos")]
    pub momentum_xpos: f32,
}

pub fn default_line_clear_effect_slots() -> SlotMachine<LineClearEffect> {
    let slots = vec![
        ("None".to_owned(), LineClearEffect::none()),
        ("Instant".to_owned(), LineClearEffect::instant()),
        ("Left-to-right".to_owned(), LineClearEffect::left_to_right()),
        ("Inward (white)".to_owned(), LineClearEffect::inward()),
        ("Outward (burn)".to_owned(), LineClearEffect::outward_burn()),
        ("Flash (white)".to_owned(), LineClearEffect::flash_white()),
        ("Blink".to_owned(), LineClearEffect::blink()),
        ("Pop minos".to_owned(), LineClearEffect::pop()),
        ("Pop minos (bigger)".to_owned(), LineClearEffect::pop_big()),
        ("ASCII sparks".to_owned(), LineClearEffect::ascii_sparks()),
        ("Unicode blast".to_owned(), LineClearEffect::blast()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Lineclear".to_owned())
}

impl LineClearEffect {
    pub fn none() -> Self {
        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Some(Duration::ZERO),
            animation: Vec::new(),
            acceleration: (0.0, 0.0),
            momentum_base: (0.0, 0.0),
            momentum_rand: (0.0, 0.0),
            momentum_xpos: 0.0,
        })
    }

    pub fn instant() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [0; 2 * Game::WIDTH],
            anim_lastidx: 1,
            color_animation: Vec::new(),
        })
    }

    pub fn left_to_right() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
            ],
            anim_lastidx: 19,
            color_animation: vec![Some(Palette::WHITE)],
        })
    }

    pub fn inward() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
            anim_lastidx: 9,
            color_animation: vec![Some(Palette::WHITE)],
        })
    }

    pub fn outward_burn() -> Self {
        let color_animation = [255, 1, 6, 4, 5].map(NonZeroU8::new).into();

        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            anim_lastidx: 9,
            color_animation,
        })
    }

    pub fn flash_white() -> Self {
        LineClearEffect::Inline(LineClearInlineEffect {
            anim_indices: [1; 2 * Game::WIDTH],
            anim_lastidx: 0,
            color_animation: vec![Some(Palette::WHITE), None, Some(Palette::WHITE)],
        })
    }

    pub fn blink() -> Self {
        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: None,
            animation: [None, Some("  ".tile()), None, Some("  ".tile()), None]
                .map(|t| (t, None))
                .into(),
            acceleration: (0.0, 0.0),
            momentum_base: (0.0, 0.0),
            momentum_rand: (0.0, 0.0),
            momentum_xpos: 0.0,
        })
    }

    pub fn pop() -> Self {
        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Some(Duration::from_millis(1000)),
            animation: vec![(None, None)],
            acceleration: (0.0, -200.0),
            momentum_base: (0.0, 25.0),
            momentum_rand: (5.0, 5.0),
            momentum_xpos: 50.0,
        })
    }

    pub fn pop_big() -> Self {
        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Some(Duration::from_millis(1000)),
            animation: vec![(None, None)],
            acceleration: (0.0, -200.0),
            momentum_base: (0.0, 45.0),
            momentum_rand: (50.0, 5.0),
            momentum_xpos: 0.0,
        })
    }

    pub fn ascii_sparks() -> Self {
        let tile_animation =
            ["@@", "$$", "##", "%%", "**", "++", "~~", ".."].map(|ss| Some(ss.tile()));
        let color_animation = [255, 3, 2, 1, 2, 7, 5, 4].map(NonZeroU8::new);
        let animation = tile_animation.into_iter().zip(color_animation).collect();

        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Some(Duration::from_millis(500)),
            animation,
            acceleration: (0.0, 0.0),
            momentum_base: (0.0, 40.0),
            momentum_rand: (0.0, 40.0),
            momentum_xpos: 100.0,
        })
    }

    pub fn blast() -> Self {
        let color_animation = [255, 1, 6, 4, 5].map(NonZeroU8::new);
        let animation = color_animation
            .into_iter()
            .map(|recolor| (None, recolor))
            .collect();

        LineClearEffect::Particle(LineClearParticleEffect {
            duration_override: Some(Duration::from_millis(500)),
            animation,
            acceleration: (0.0, 10.0),
            momentum_base: (0.0, -45.0),
            momentum_rand: (50.0, 10.0),
            momentum_xpos: 50.0,
        })
    }
}
