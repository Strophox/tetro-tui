mod game_keybinds;
mod game_mode_preferences;
mod gameplay_settings;
mod graphics_settings;
mod palette;

pub use game_keybinds::GameKeybinds;
pub use game_mode_preferences::GameModePreferences;
pub use gameplay_settings::GameplaySettings;
pub use graphics_settings::{Glyphset, GraphicsSettings};
pub use palette::Palette;

use crate::{
    fmt_helpers::arabic_to_roman,
    settings::{
        game_keybinds::default_keybinds_slots, gameplay_settings::default_gameplay_slots,
        graphics_settings::default_graphics_slots, palette::default_palette_slots,
    },
};

/// This struct allows storing 'slots' (elements of some kind), where a certain
/// number of elements is considere as 'unmodifiable' (should not be modified)
/// but can be automatically cloned to a new slot and then modified for ease of use.
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SlotMachine<T> {
    /// The number of slots considered unmodifiable.
    pub unmodifiable_slots: usize,
    /// The string that is used as base to generate a name for duplicate slots.
    pub name_templating: String,
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
            name_templating: cloned_slot_name_template,
        }
    }

    // FIXME: Remove unused code or reconsider. Not ergonomic enough for current usecase: `self.settings.gameplay_picked = self.settings.gameplay_slotmachine.increment_cyclic(self.settings.gameplay_picked);`
    // pub fn increment_cyclic(&self, slot_idx: usize) -> usize {
    //     (slot_idx + 1) % self.slots.len()
    // }
    // pub fn decrement_cyclic(&self, slot_idx: usize) -> usize {
    //     (slot_idx + self.slots.len() - 1) % self.slots.len()
    // }

    /// Given a valid index, clones and appends to itself of the corresponding slot if it is considered unmodifiable,
    /// and returns the index of the new slot.
    /// Otherwise return `None` and do nothing (i.e. slot is 'modifiable' or index invalid).
    pub fn clone_slot_if_unmodifiable(&mut self, slot_idx: usize) -> Option<usize> {
        slot_idx.lt(&self.unmodifiable_slots).then(|| {
            let cloned_slot_content = self.slots[slot_idx].1.clone();

            let mut n = 1;
            let cloned_slot_name = loop {
                let name = format!("{} {}", self.name_templating, arabic_to_roman(n));
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

// #[serde_with::serde_as]
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub graphics_picked: usize,
    pub keybinds_picked: usize,
    pub gameplay_picked: usize,

    #[serde(rename = "PALETTE_SLOTS")]
    pub palette_slotmachine: SlotMachine<Palette>,
    #[serde(rename = "GRAPHICS_SLOTS")]
    pub graphics_slotmachine: SlotMachine<GraphicsSettings>,
    #[serde(rename = "GAME_KEYBINDS_SLOTS")]
    pub keybinds_slotmachine: SlotMachine<GameKeybinds>,
    #[serde(rename = "GAMEPLAY_CONFIG_SLOTS")]
    pub gameplay_slotmachine: SlotMachine<GameplaySettings>,

    #[serde(rename = "GAMEMODE_PREFERENCES")]
    pub gamemode_preferences: GameModePreferences,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            graphics_picked: 0,
            keybinds_picked: 0,
            gameplay_picked: 0,

            palette_slotmachine: default_palette_slots(),
            graphics_slotmachine: default_graphics_slots(),
            keybinds_slotmachine: default_keybinds_slots(),
            gameplay_slotmachine: default_gameplay_slots(),

            gamemode_preferences: GameModePreferences::default(),
        }
    }
}

impl Settings {
    // NOTE: The common pattern for making use of SlotMachines is currently:
    // 1. Have a SlotMachine<T>.
    // 2. Store an index into the slots somewhere.
    // 3. Implementing 'getter' on the place that owns the slots (not where the index is stored.)
    pub fn palette(&self) -> &Palette {
        &self.palette_slotmachine.slots[self.graphics().palette_picked].1
    }
    pub fn palette_lockedtiles(&self) -> &Palette {
        &self.palette_slotmachine.slots[self.graphics().boardpalette_picked].1
    }

    pub fn graphics(&self) -> &GraphicsSettings {
        &self.graphics_slotmachine.slots[self.graphics_picked].1
    }
    pub fn keybinds(&self) -> &GameKeybinds {
        &self.keybinds_slotmachine.slots[self.keybinds_picked].1
    }
    pub fn gameplay(&self) -> &GameplaySettings {
        &self.gameplay_slotmachine.slots[self.gameplay_picked].1
    }
    pub fn graphics_mut(&mut self) -> &mut GraphicsSettings {
        &mut self.graphics_slotmachine.slots[self.graphics_picked].1
    }
    pub fn keybinds_mut(&mut self) -> &mut GameKeybinds {
        &mut self.keybinds_slotmachine.slots[self.keybinds_picked].1
    }
    pub fn gameplay_mut(&mut self) -> &mut GameplaySettings {
        &mut self.gameplay_slotmachine.slots[self.gameplay_picked].1
    }
}
