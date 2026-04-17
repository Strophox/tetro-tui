mod game_keybinds;
mod game_mode_preferences;
mod gameplay_settings;
mod graphics_settings;

pub use game_keybinds::GameKeybinds;
pub use game_mode_preferences::GameModePreferences;
pub use gameplay_settings::GameplaySettings;
pub use graphics_settings::{
    hard_drop_effect::HardDropEffect,
    line_clear_effect::{LineClearEffect, LineClearInlineEffect, LineClearParticleEffect},
    lock_effect::LockEffect,
    mini_tet_style::MiniTetStyle,
    mino_textures::MinoTextures,
    palette::Palette,
    small_tet_style::SmallTetStyle,
    tui_style::TuiStyle,
    GraphicsSettings, TileTexture,
};

use crate::{
    fmt_helpers::to_roman,
    tui_settings::{
        game_keybinds::default_keybinds_slots,
        gameplay_settings::default_gameplay_slots,
        graphics_settings::{
            default_graphics_slots, hard_drop_effect::default_hard_drop_effect_slots,
            line_clear_effect::default_line_clear_effect_slots,
            lock_effect::default_lock_effect_slots, mini_tet_style::default_mini_tet_style_slots,
            mino_textures::default_mino_textures_slots, palette::default_palette_slots,
            small_tet_style::default_small_tet_style_slots, tui_style::default_tui_style_slots,
        },
    },
};

// #[serde_with::serde_as]
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub graphics_selected: usize,
    #[serde(rename = "GRAPHICS_SLOTS")]
    pub graphics_slotmachine: SlotMachine<GraphicsSettings>,
    #[serde(rename = "PALETTE_SLOTS")]
    pub palette_slotmachine: SlotMachine<Palette>,
    #[serde(rename = "TUI_STYLE_SLOTS")]
    pub tui_style_slotmachine: SlotMachine<TuiStyle>,
    #[serde(rename = "MINO_TEXTURES_SLOTS")]
    pub mino_textures_slotmachine: SlotMachine<MinoTextures>,
    #[serde(rename = "HARD_DROP_EFFECT_SLOTS")]
    pub hard_drop_effect_slotmachine: SlotMachine<HardDropEffect>,
    #[serde(rename = "LOCK_EFFECT_SLOTS")]
    pub lock_effect_slotmachine: SlotMachine<LockEffect>,
    #[serde(rename = "LINE_CLEAR_EFFECT_SLOTS")]
    pub line_clear_effect_slotmachine: SlotMachine<LineClearEffect>,
    #[serde(rename = "MINI_TET_STYLE_SLOTS")]
    pub mini_tet_style_slotmachine: SlotMachine<MiniTetStyle>,
    #[serde(rename = "SMALL_TET_STYLE_SLOTS")]
    pub small_tet_style_slotmachine: SlotMachine<SmallTetStyle>,

    pub keybinds_selected: usize,
    #[serde(rename = "GAME_KEYBINDS_SLOTS")]
    pub keybinds_slotmachine: SlotMachine<GameKeybinds>,

    pub gameplay_selected: usize,
    #[serde(rename = "GAMEPLAY_CONFIG_SLOTS")]
    pub gameplay_slotmachine: SlotMachine<GameplaySettings>,

    #[serde(rename = "GAME_MODE_PREFERENCES")]
    pub game_mode_preferences: GameModePreferences,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            graphics_selected: 0,
            graphics_slotmachine: default_graphics_slots(),
            palette_slotmachine: default_palette_slots(),
            tui_style_slotmachine: default_tui_style_slots(),
            mino_textures_slotmachine: default_mino_textures_slots(),
            hard_drop_effect_slotmachine: default_hard_drop_effect_slots(),
            lock_effect_slotmachine: default_lock_effect_slots(),
            line_clear_effect_slotmachine: default_line_clear_effect_slots(),
            mini_tet_style_slotmachine: default_mini_tet_style_slots(),
            small_tet_style_slotmachine: default_small_tet_style_slots(),

            keybinds_selected: 0,
            keybinds_slotmachine: default_keybinds_slots(),

            gameplay_selected: 0,
            gameplay_slotmachine: default_gameplay_slots(),

            game_mode_preferences: GameModePreferences::default(),
        }
    }
}

impl Settings {
    // NOTE: The common pattern for making use of SlotMachines is currently:
    // 1. Have a SlotMachine<T>.
    // 2. Store an index into the slots somewhere.
    // 3. Implementing 'getter' on the place that owns the slots (not where the index is stored.)
    pub fn palette(&self) -> &Palette {
        &self
            .palette_slotmachine
            .grab(self.graphics().palette_selected)
            .1
    }
    pub fn tui_style(&self) -> &TuiStyle {
        &self
            .tui_style_slotmachine
            .grab(self.graphics().tui_style_selected)
            .1
    }
    pub fn mino_textures(&self) -> &MinoTextures {
        &self
            .mino_textures_slotmachine
            .grab(self.graphics().mino_textures_selected)
            .1
    }
    pub fn hard_drop_effect(&self) -> &HardDropEffect {
        &self
            .hard_drop_effect_slotmachine
            .grab(self.graphics().hard_drop_selected)
            .1
    }
    pub fn lock_effect(&self) -> &LockEffect {
        &self
            .lock_effect_slotmachine
            .grab(self.graphics().lock_effect_selected)
            .1
    }
    pub fn line_clear_effect(&self) -> &LineClearEffect {
        &self
            .line_clear_effect_slotmachine
            .grab(self.graphics().line_clear_selected)
            .1
    }
    pub fn mini_tet_style(&self) -> &MiniTetStyle {
        &self
            .mini_tet_style_slotmachine
            .grab(self.graphics().mini_tet_selected)
            .1
    }
    pub fn small_tet_style(&self) -> &SmallTetStyle {
        &self
            .small_tet_style_slotmachine
            .grab(self.graphics().small_tet_selected)
            .1
    }
    pub fn boardpalette(&self) -> &Palette {
        &self
            .palette_slotmachine
            .grab(self.graphics().boardpalette_selected)
            .1
    }

    pub fn graphics(&self) -> &GraphicsSettings {
        &self.graphics_slotmachine.grab(self.graphics_selected).1
    }
    pub fn keybinds(&self) -> &GameKeybinds {
        &self.keybinds_slotmachine.grab(self.keybinds_selected).1
    }
    pub fn gameplay(&self) -> &GameplaySettings {
        &self.gameplay_slotmachine.grab(self.gameplay_selected).1
    }
    pub fn graphics_mut(&mut self) -> &mut GraphicsSettings {
        &mut self.graphics_slotmachine.grab_mut(self.graphics_selected).1
    }
    pub fn keybinds_mut(&mut self) -> &mut GameKeybinds {
        &mut self.keybinds_slotmachine.grab_mut(self.keybinds_selected).1
    }
    pub fn gameplay_mut(&mut self) -> &mut GameplaySettings {
        &mut self.gameplay_slotmachine.grab_mut(self.gameplay_selected).1
    }
}

/// This struct allows storing 'slots' (elements of some kind), where a certain
/// number of elements is considere as 'unmodifiable' (should not be modified)
/// but can be automatically cloned to a new slot and then modified for ease of use.
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SlotMachine<T> {
    /// The number of slots considered unmodifiable.
    pub unmodifiable_slots: usize,
    /// The string that is used as base to generate a name for duplicate slots.
    pub name_template: String,
    /// The actual contents of the slot machine: the slots (usually 'profiles').
    pub slots: Vec<(String, T)>,
}

impl<T: Clone> SlotMachine<T> {
    pub fn with_unmodifiable_slots(
        slots: Vec<(String, T)>,
        cloned_slot_name_template: String,
    ) -> Self {
        let num_unmodifiable_slots = slots.len();
        Self {
            slots,
            unmodifiable_slots: num_unmodifiable_slots,
            name_template: cloned_slot_name_template,
        }
    }

    pub fn grab(&self, mut idx: usize) -> &(String, T) {
        if idx >= self.slots.len() {
            idx = 0;
        }
        &self.slots[idx]
    }

    pub fn grab_mut(&mut self, mut idx: usize) -> &mut (String, T) {
        if idx >= self.slots.len() {
            idx = 0;
        }
        &mut self.slots[idx]
    }

    /// Given a valid index, clones and appends to itself of the corresponding slot if it is considered unmodifiable,
    /// and returns the index of the new slot.
    /// Otherwise return `None` and do nothing (i.e. slot is 'modifiable' or index invalid).
    pub fn clone_slot_if_unmodifiable(&mut self, slot_idx: usize) -> Option<usize> {
        slot_idx.lt(&self.unmodifiable_slots).then(|| {
            let cloned_slot_content = self.slots[slot_idx].1.clone();

            let mut n = 1;
            let cloned_slot_name = loop {
                let name = format!("{} {}", self.name_template, to_roman(n));
                if self.slots.iter().all(|s| s.0 != name) {
                    break name;
                }
                n += 1;
            };

            self.slots.push((cloned_slot_name, cloned_slot_content));

            self.slots.len() - 1
        })
    }
}
