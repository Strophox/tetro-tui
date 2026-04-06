use falling_tetromino_engine::{Button, Game, GameBuilder, InGameTime, Input, NotificationLevel};

use crate::game_modifiers;

/// Raw, uncompressed representation of a partial or complete input history.
///
/// We normally presuppose this is sorted by timestamps.
pub type UncompressedInputHistory = Vec<(InGameTime, Input)>;

/// Compressed verson of an input history.
///
/// Currently done using deltatime and assumption that inputs have millisecond precision at most.
#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct CompressedInputHistory {
    inputbuf: Vec<u128>,
}

impl CompressedInputHistory {
    // How many bits it takes to encode a `ButtonChange`:
    // - 1 bit for Press/Release,
    // - At time of writing: 4 bits for the 11 `Button` variants.
    pub const BUTTON_CHANGE_BITSIZE: usize =
        1 + Button::VARIANTS.len().next_power_of_two().ilog2() as usize;

    pub fn new(game_input_history: &UncompressedInputHistory) -> Self {
        let mut inputbuf = Vec::new();

        let mut update_time_0 = InGameTime::ZERO;

        for (update_time_1, button_change) in game_input_history.iter() {
            let time_diff = update_time_1.saturating_sub(update_time_0);
            let i = Self::compress_input((time_diff, *button_change));

            // Add further input.
            inputbuf.push(i);

            update_time_0 = *update_time_1;
        }

        Self { inputbuf }
    }

    pub fn decompress(&self) -> UncompressedInputHistory {
        let mut decompressed_inputs = Vec::new();

        let mut update_time_0 = InGameTime::ZERO;
        for i in self.inputbuf.iter() {
            let (time_diff, button_change) = Self::decompress_input(*i);
            let update_time_1 = update_time_0.saturating_add(time_diff);

            // Add further input.
            decompressed_inputs.push((update_time_1, button_change));

            update_time_0 = update_time_1;
        }

        decompressed_inputs
    }

    // For serialization reasons, we encode a single user input as `u128` instead of
    // `(GameTime, ButtonChange)`, which would have a verbose direct string representation.
    fn compress_input((update_target_time, button_change): (InGameTime, Input)) -> u128 {
        // Encode `GameTime = std::time::Duration` using `std::time::Duration::as_millis`.
        // NOTE: We actually use `millis` not `nanos` as a convention which is upheld by `play_game.rs`!
        let millis: u128 = update_target_time.as_millis();
        // Encode `falling_tetromino_engine::ButtonChange` using `Self::encode_button_change`.
        let bc_bits: u8 = Self::compress_buttonchange(&button_change);
        (millis << Self::BUTTON_CHANGE_BITSIZE) | u128::from(bc_bits)
    }

    fn decompress_input(i: u128) -> (InGameTime, Input) {
        let mask = u128::MAX >> (128 - Self::BUTTON_CHANGE_BITSIZE);
        let bc_bits = u8::try_from(i & mask).unwrap();
        let millis = u64::try_from(i >> Self::BUTTON_CHANGE_BITSIZE).unwrap();
        (
            std::time::Duration::from_millis(millis),
            Self::decompress_buttonchange(bc_bits),
        )
    }

    fn compress_buttonchange(button_change: &Input) -> u8 {
        match button_change {
            Input::Deactivate(button) => (*button as u8) << 1,
            Input::Activate(button) => ((*button as u8) << 1) | 1,
        }
    }

    fn decompress_buttonchange(b: u8) -> Input {
        (if b.is_multiple_of(2) {
            Input::Deactivate
        } else {
            Input::Activate
        })(Button::VARIANTS[usize::from(b >> 1)])
    }
}

/// All the data required to functionally reconstruct gameplay.
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameRestorationData<T> {
    pub builder: GameBuilder,
    pub mod_ids_args: Vec<(String, String)>,
    pub input_history: T,
    pub forfeit: Option<InGameTime>,
}

impl<T> GameRestorationData<T> {
    pub fn new(
        game: &Game,
        input_history: T,
        forfeit: Option<InGameTime>,
    ) -> GameRestorationData<T> {
        let (builder, mod_ids_args) = game.blueprint();

        GameRestorationData {
            builder,
            mod_ids_args,
            input_history,
            forfeit,
        }
    }

    pub fn map<U>(self, f: impl Fn(T) -> U) -> GameRestorationData<U> {
        GameRestorationData::<U> {
            builder: self.builder,
            mod_ids_args: self.mod_ids_args,
            input_history: f(self.input_history),
            forfeit: self.forfeit,
        }
    }
}

impl GameRestorationData<UncompressedInputHistory> {
    pub fn restore(&self, input_index: usize) -> Game {
        // Step 1: Prepare builder.
        let builder = self.builder.clone();
        // Step 2: Build actual game by possibly reconstructing mods to finalize builder with.
        let mut game = if self.mod_ids_args.is_empty() {
            builder.build()
        } else {
            match game_modifiers::reconstruct_build_modded(&builder, &self.mod_ids_args) {
                Ok((mut modded_game, unrecognized_mod_ids)) => {
                    if !unrecognized_mod_ids.is_empty() {
                        // Add warning messages if certain mods could not be recognized.
                        // This should never happen in our application.
                        let warn_messages = unrecognized_mod_ids
                            .into_iter()
                            .map(|mod_desc| format!("WARNING: idk mod {mod_desc:?}"))
                            .collect();

                        let print_warn_msgs_mod =
                            game_modifiers::PrintMsgs::modifier(warn_messages);

                        modded_game.modifiers.push(print_warn_msgs_mod);
                    }

                    modded_game
                }
                Err(msg) => {
                    let error_messages = vec![format!("ERROR: {msg}")];

                    let print_error_msg_mod = game_modifiers::PrintMsgs::modifier(error_messages);

                    builder.build_modded(vec![print_error_msg_mod])
                }
            }
        };

        // Step 3: Reenact recorded game inputs.
        let restore_notification_level = game.config.notification_level;

        game.config.notification_level = NotificationLevel::Silent;
        for (update_time, button_change) in self.input_history.iter().take(input_index) {
            // FIXME: Handle UpdateGameError? If not, why not?
            let _v = game.update(*update_time, Some(*button_change));
        }

        game.config.notification_level = restore_notification_level;

        game
    }
}
