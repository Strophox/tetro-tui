mod game_keybinds;
mod game_mode_preferences;
mod gameplay_settings;
mod graphics_settings;

pub use game_keybinds::GameKeybinds;
pub use game_mode_preferences::{CustomModeConfig, GameModePreferences};
pub use gameplay_settings::GameplaySettings;
pub use graphics_settings::{
    GraphicsSettings, TileTexture,
    hard_drop_effect::HardDropEffect,
    line_clear_effect::{LineClearEffect, LineClearInlineEffect, LineClearParticleEffect},
    lock_effect::LockEffect,
    mini_tetromino_symbols::MiniTetrominoSymbols,
    mino_symbols::MinoTextures,
    palette::Palette,
    small_tetromino_symbols::SmallTetrominoSymbols,
    tui_symbols::TuiSymbols,
};

use crate::{
    fmt_helpers::to_roman,
    tui_settings::{
        game_keybinds::game_keybinds_presets,
        gameplay_settings::gameplay_settings_presets,
        graphics_settings::{
            graphics_settings_presets, hard_drop_effect::hard_drop_effect_presets,
            line_clear_effect::line_clear_effect_presets, lock_effect::lock_effect_presets,
            mini_tetromino_symbols::mini_tetromino_symbols_presets,
            mino_symbols::mino_symbols_presets, palette::palette_presets,
            small_tetromino_symbols::small_tetromino_symbols_presets,
            tui_symbols::tui_symbols_presets,
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
    #[serde(rename = "TUI_SYMBOLS_SLOTS")]
    pub tui_style_slotmachine: SlotMachine<TuiSymbols>,
    #[serde(rename = "MINO_SYMBOLS_SLOTS")]
    pub mino_symbols_slotmachine: SlotMachine<MinoTextures>,
    #[serde(rename = "HARD_DROP_EFFECT_SLOTS")]
    pub hard_drop_effect_slotmachine: SlotMachine<HardDropEffect>,
    #[serde(rename = "LOCK_EFFECT_SLOTS")]
    pub lock_effect_slotmachine: SlotMachine<LockEffect>,
    #[serde(rename = "LINE_CLEAR_EFFECT_SLOTS")]
    pub line_clear_effect_slotmachine: SlotMachine<LineClearEffect>,
    #[serde(rename = "MINI_TETROMINO_SYMBOLS_SLOTS")]
    pub mini_tetromino_symbols_slotmachine: SlotMachine<MiniTetrominoSymbols>,
    #[serde(rename = "SMALL_TETROMINO_SYMBOLS_SLOTS")]
    pub small_tetromino_symbols_slotmachine: SlotMachine<SmallTetrominoSymbols>,

    pub keybinds_selected: usize,
    #[serde(rename = "GAME_KEYBINDS_SLOTS")]
    pub keybinds_slotmachine: SlotMachine<GameKeybinds>,

    pub gameplay_selected: usize,
    #[serde(rename = "GAMEPLAY_SETTINGS_SLOTS")]
    pub gameplay_slotmachine: SlotMachine<GameplaySettings>,

    #[serde(rename = "GAME_MODE_PREFERENCES")]
    pub game_mode_preferences: GameModePreferences,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            graphics_selected: 0,
            graphics_slotmachine: graphics_settings_presets(),
            palette_slotmachine: palette_presets(),
            tui_style_slotmachine: tui_symbols_presets(),
            mino_symbols_slotmachine: mino_symbols_presets(),
            hard_drop_effect_slotmachine: hard_drop_effect_presets(),
            lock_effect_slotmachine: lock_effect_presets(),
            line_clear_effect_slotmachine: line_clear_effect_presets(),
            mini_tetromino_symbols_slotmachine: mini_tetromino_symbols_presets(),
            small_tetromino_symbols_slotmachine: small_tetromino_symbols_presets(),

            keybinds_selected: 0,
            keybinds_slotmachine: game_keybinds_presets(),

            gameplay_selected: 0,
            gameplay_slotmachine: gameplay_settings_presets(),

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
    pub fn tui_symbols(&self) -> &TuiSymbols {
        &self
            .tui_style_slotmachine
            .grab(self.graphics().tui_symbols_selected)
            .1
    }
    pub fn mino_symbols(&self) -> &MinoTextures {
        &self
            .mino_symbols_slotmachine
            .grab(self.graphics().mino_symbols_selected)
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
    pub fn mini_tetromino_symbols(&self) -> &MiniTetrominoSymbols {
        &self
            .mini_tetromino_symbols_slotmachine
            .grab(self.graphics().mini_tetromino_symbols_selected)
            .1
    }
    pub fn small_tetromino_symbols(&self) -> &SmallTetrominoSymbols {
        &self
            .small_tetromino_symbols_slotmachine
            .grab(self.graphics().small_tetromino_symbols_selected)
            .1
    }
    pub fn lockedminopalette(&self) -> &Palette {
        &self
            .palette_slotmachine
            .grab(self.graphics().lockedminopalette_selected)
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
