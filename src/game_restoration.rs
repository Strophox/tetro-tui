use falling_tetromino_engine::{Button, Game, GameBuilder, InGameTime, Input, NotificationLevel};

use crate::game_modifiers;

/// Raw, uncompressed representation of a partial or complete input history.
///
/// We normally presuppose this is sorted by timestamps.
pub use raw::RawInputHistory;

pub use deltas_bitencoded_base64str::DeltasBitencodedBase64strInputHistory as EncodedInputHistory;

/// All the data required to functionally reconstruct gameplay.
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameRestorationData<IH: InputHistoryEncoder> {
    pub builder: GameBuilder,
    pub mod_ids_args: Vec<(String, String)>,
    pub input_history: IH,
    pub forfeit: Option<InGameTime>,
}

impl<IH: InputHistoryEncoder> GameRestorationData<IH> {
    pub fn new(
        game: &Game,
        input_history: IH,
        forfeit: Option<InGameTime>,
    ) -> GameRestorationData<IH> {
        let (builder, mod_ids_args) = game.blueprint();

        GameRestorationData {
            builder,
            mod_ids_args,
            input_history,
            forfeit,
        }
    }
}

impl GameRestorationData<RawInputHistory> {
    pub fn restore(&self, input_index: usize) -> Game {
        // Step 1: Prepare builder.
        let builder = self.builder.clone();
        // Step 2: Build actual game by possibly reconstructing mods to finalize builder with.
        let mut game = if self.mod_ids_args.is_empty() {
            builder.build()
        } else {
            match game_modifiers::reconstruct_modded(&builder, &self.mod_ids_args) {
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
        for (update_time, input) in self.input_history.inputs.iter().take(input_index) {
            // FIXME: Handle UpdateGameError? If not, why not?
            let _v = game.update(*update_time, Some(*input));
        }

        game.config.notification_level = restore_notification_level;

        game
    }
}

impl GameRestorationData<RawInputHistory> {
    pub fn encode<IH: InputHistoryEncoder>(self) -> GameRestorationData<IH> {
        GameRestorationData {
            builder: self.builder,
            input_history: IH::encode(&self.input_history),
            mod_ids_args: self.mod_ids_args,
            forfeit: self.forfeit,
        }
    }
}

impl<IH: InputHistoryEncoder> GameRestorationData<IH> {
    pub fn try_decode(self) -> Result<GameRestorationData<RawInputHistory>, String> {
        Ok(GameRestorationData {
            builder: self.builder,
            input_history: self.input_history.try_decode()?,
            mod_ids_args: self.mod_ids_args,
            forfeit: self.forfeit,
        })
    }
}

pub trait InputHistoryEncoder {
    fn encode(raw_input_history: &RawInputHistory) -> Self;
    fn try_decode(&self) -> Result<RawInputHistory, String>;
}

mod raw {
    use super::*;

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
    #[serde(transparent)]
    pub struct RawInputHistory {
        pub inputs: Vec<(InGameTime, Input)>,
    }
    impl From<Vec<(InGameTime, Input)>> for RawInputHistory {
        fn from(value: Vec<(InGameTime, Input)>) -> Self {
            Self { inputs: value }
        }
    }

    impl InputHistoryEncoder for RawInputHistory {
        fn encode(raw_input_history: &RawInputHistory) -> Self {
            raw_input_history.clone()
        }

        fn try_decode(&self) -> Result<RawInputHistory, String> {
            Ok(self.clone())
        }
    }
}

#[allow(unused)]
mod deltas_bitencoded_base64prefixstr {
    use super::*;
    use crate::fmt_helpers::{to_base64_charbyte, try_from_base64_charbyte};

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
    #[serde(transparent)]
    pub struct DeltasBitencodedBase64PrefixstrInputHistory {
        pub dti_base64prefixstr: String,
    }
    impl InputHistoryEncoder for DeltasBitencodedBase64PrefixstrInputHistory {
        fn encode(raw_input_history: &RawInputHistory) -> Self {
            let mut dti_base64prefixstr = Vec::new();
            let mut prev_update_time = InGameTime::ZERO;
            for (next_update_time, input) in raw_input_history.inputs.iter() {
                let time_delta = next_update_time.saturating_sub(prev_update_time);
                let mut dti_bitpattern = timed_input_to_u128((time_delta, *input));

                // Mask for 5 (of the max 6) bits we can use from a base64 digit.
                const MASK_U5: u128 = u128::MAX >> (128 - 5);
                // Get first 5 bits.
                let mut dti_bitpattern_chunk_u5 = (dti_bitpattern & MASK_U5) as u8;
                dti_bitpattern >>= 5;
                loop {
                    // If remaining bitpattern is empty, we end our prefixcode.
                    if dti_bitpattern == 0 {
                        let ti_bitpattern_chunk_u6 = dti_bitpattern_chunk_u5 << 1;
                        dti_base64prefixstr
                            .push(to_base64_charbyte(ti_bitpattern_chunk_u6).unwrap());
                        break;
                    } else {
                        let ti_bitpattern_chunk_u6 = (dti_bitpattern_chunk_u5 << 1) | 1;
                        dti_base64prefixstr
                            .push(to_base64_charbyte(ti_bitpattern_chunk_u6).unwrap());
                    }
                    // Get next 5 bits.
                    dti_bitpattern_chunk_u5 = (dti_bitpattern & MASK_U5) as u8;
                    dti_bitpattern >>= 5;
                }

                prev_update_time = *next_update_time;
            }
            let dti_base64prefixstr = String::from_utf8(dti_base64prefixstr).unwrap();
            Self {
                dti_base64prefixstr,
            }
        }

        fn try_decode(&self) -> Result<RawInputHistory, String> {
            let mut raw_input_history = Vec::new();
            let mut prev_update_time = InGameTime::ZERO;
            let mut dti_bitpattern = 0;
            for base64_charbyte in self.dti_base64prefixstr.bytes() {
                let dti_bitpattern_chunk_u6 = try_from_base64_charbyte(base64_charbyte)?;
                // If marked as such, prefixcode has ended.
                if dti_bitpattern_chunk_u6 & 1 == 0 {
                    // Push last 5 bits.
                    dti_bitpattern <<= 5;
                    dti_bitpattern |= (dti_bitpattern_chunk_u6 >> 1) as u128;
                    let (time_delta, input) = try_u128_to_timed_input(dti_bitpattern)?;
                    let next_update_time = prev_update_time.saturating_add(time_delta);
                    raw_input_history.push((next_update_time, input));

                    prev_update_time = next_update_time;
                    dti_bitpattern = 0;
                } else {
                    // Push next 5 bits.
                    dti_bitpattern <<= 5;
                    dti_bitpattern |= (dti_bitpattern_chunk_u6 >> 1) as u128;
                }
            }

            Ok(raw_input_history.into())
        }
    }
}

#[allow(unused)]
mod deltas_bitencoded_base64str {
    use super::*;
    use crate::fmt_helpers::{to_base64, try_from_base64};

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
    #[serde(transparent)]
    pub struct DeltasBitencodedBase64strInputHistory {
        pub dti_base64str: String,
    }
    impl InputHistoryEncoder for DeltasBitencodedBase64strInputHistory {
        fn encode(raw_input_history: &RawInputHistory) -> Self {
            let dti_base64str =
                deltas_bitencoded::DeltasBitencodedInputHistory::encode(raw_input_history)
                    .dti_bitpatterns
                    .into_iter()
                    .map(to_base64)
                    .collect::<Vec<_>>()
                    .join("~");

            Self { dti_base64str }
        }

        fn try_decode(&self) -> Result<RawInputHistory, String> {
            let dti_bitpatterns = self
                .dti_base64str
                .split("~")
                .map(try_from_base64)
                .collect::<Result<Vec<_>, _>>()?;

            deltas_bitencoded::DeltasBitencodedInputHistory { dti_bitpatterns }.try_decode()
        }
    }
}

#[allow(unused)]
mod deltas_bitencoded {
    use super::*;

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
    #[serde(transparent)]
    pub struct DeltasBitencodedInputHistory {
        pub dti_bitpatterns: Vec<u128>,
    }
    impl InputHistoryEncoder for DeltasBitencodedInputHistory {
        fn encode(raw_input_history: &RawInputHistory) -> Self {
            let mut dti_bitpatterns = Vec::new();
            let mut prev_update_time = InGameTime::ZERO;
            for (next_update_time, input) in raw_input_history.inputs.iter() {
                let time_delta = next_update_time.saturating_sub(prev_update_time);
                let dti_bitpattern = timed_input_to_u128((time_delta, *input));

                dti_bitpatterns.push(dti_bitpattern);
                prev_update_time = *next_update_time;
            }

            Self { dti_bitpatterns }
        }

        fn try_decode(&self) -> Result<RawInputHistory, String> {
            let mut raw_input_history = Vec::new();
            let mut prev_update_time = InGameTime::ZERO;
            for dti_bitpattern in self.dti_bitpatterns.iter().copied() {
                let (time_delta, input) = try_u128_to_timed_input(dti_bitpattern)?;
                let next_update_time = prev_update_time.saturating_add(time_delta);

                raw_input_history.push((next_update_time, input));
                prev_update_time = next_update_time;
            }

            Ok(raw_input_history.into())
        }
    }
}

#[allow(unused)]
mod simple_bitencoded {
    use super::*;

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
    #[serde(transparent)]
    pub struct SimpleBitencodedInputHistory {
        ti_bitpatterns: Vec<u128>,
    }
    impl InputHistoryEncoder for SimpleBitencodedInputHistory {
        fn encode(raw_input_history: &RawInputHistory) -> Self {
            let ti_bitpatterns = raw_input_history
                .inputs
                .iter()
                .copied()
                .map(timed_input_to_u128)
                .collect();

            Self { ti_bitpatterns }
        }

        fn try_decode(&self) -> Result<RawInputHistory, String> {
            let raw_input_history = self
                .ti_bitpatterns
                .iter()
                .copied()
                .map(try_u128_to_timed_input)
                .collect::<Result<Vec<_>, _>>()?;

            Ok(raw_input_history.into())
        }
    }
}

/// We round away 1_000_000 nanos -> We only keep milliseconds!
pub const NANOS_QUANTIZATION: u128 = 1_000_000;

pub trait QuantizeInGameTime {
    fn quantize(&self) -> InGameTime;
}

impl QuantizeInGameTime for InGameTime {
    fn quantize(&self) -> InGameTime {
        let truncated_nanos = self.as_nanos() - self.as_nanos() % NANOS_QUANTIZATION;
        let ceil_inc = if self.as_nanos().is_multiple_of(NANOS_QUANTIZATION) {
            0
        } else {
            NANOS_QUANTIZATION
        };

        InGameTime::from_nanos_u128(truncated_nanos + ceil_inc)
    }
}

// How many bits it takes to encode a `ButtonChange`:
// - 1 bit for Press/Release,
// - At time of writing: 4 bits for the 11 `Button` variants.
pub const INPUT_BITPATTERNSIZE: usize = {
    let i_bits = (Button::VARIANTS.len().next_power_of_two().ilog2() + 1) as usize;
    assert!(i_bits <= 8);
    i_bits
};

/// For serialization reasons, we encode a single user input as `u128` instead of
/// `(GameTime, ButtonChange)`, which would have a verbose direct string representation.
///
/// WARNING: THIS ASSUMES NANOS CAN BE TRUNCATED USING `NANOS_QUANTIZATION`.
fn timed_input_to_u128(timed_input: (InGameTime, Input)) -> u128 {
    let (time, input) = timed_input;
    let nanos: u128 = time.as_nanos();
    let truncated_nanos = nanos / NANOS_QUANTIZATION;
    let i_bitpattern: u8 = input_to_u8(&input);
    (truncated_nanos << INPUT_BITPATTERNSIZE) | u128::from(i_bitpattern)
}

/// WARNING: THIS ASSUMES NANOS WERE BE TRUNCATED USING `NANOS_QUANTIZATION`.
fn try_u128_to_timed_input(bitpattern: u128) -> Result<(InGameTime, Input), String> {
    // Generate mask for lower 'input information' bits.
    const MASK: u128 = u128::MAX >> (128 - INPUT_BITPATTERNSIZE);
    let i_bitpattern = (bitpattern & MASK) as u8;
    let truncated_nanos = bitpattern >> INPUT_BITPATTERNSIZE;
    Ok((
        std::time::Duration::from_nanos_u128(truncated_nanos * NANOS_QUANTIZATION),
        try_u8_to_input(i_bitpattern)?,
    ))
}

fn input_to_u8(input: &Input) -> u8 {
    match input {
        Input::Deactivate(button) => (*button as u8) << 1,
        Input::Activate(button) => ((*button as u8) << 1) | 1,
    }
}

fn try_u8_to_input(bitpattern: u8) -> Result<Input, String> {
    let inp = if bitpattern.is_multiple_of(2) {
        Input::Deactivate
    } else {
        Input::Activate
    };
    let Some(but) = Button::VARIANTS.get(usize::from(bitpattern >> 1)) else {
        return Err(format!(
            "cannot decode button variant from `{}`",
            bitpattern >> 1
        ));
    };
    Ok(inp(*but))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    #[test]
    fn input_u8_roundtrips() {
        for but in Button::VARIANTS {
            for inp in [Input::Activate, Input::Deactivate] {
                let input = inp(but);
                assert_eq!(try_u8_to_input(input_to_u8(&input)), Ok(input));
            }
        }
    }

    #[test]
    fn timed_input_u128_micros_roundtrips() {
        for e in 0..32 {
            for inp in [Input::Activate, Input::Deactivate] {
                for but in Button::VARIANTS {
                    let timed_input = (InGameTime::from_nanos(1 << e).quantize(), inp(but));
                    assert_eq!(
                        try_u128_to_timed_input(timed_input_to_u128(timed_input)),
                        Ok(timed_input)
                    );
                }
            }
        }
    }

    fn run_compressor_test<IH: InputHistoryEncoder>() {
        let update_times = (0..22).map(Duration::from_millis);
        let inputs = [Input::Activate, Input::Deactivate]
            .iter()
            .cycle()
            .zip(Button::VARIANTS.iter().cycle())
            .map(|(inp, but)| inp(*but));
        let raw_input_history = update_times.zip(inputs).collect::<Vec<_>>().into();

        assert_eq!(
            IH::encode(&raw_input_history).try_decode(),
            Ok(raw_input_history)
        );
    }

    #[test]
    fn raw_roundtrip() {
        run_compressor_test::<raw::RawInputHistory>();
    }

    #[test]
    fn simple_bitencoded_roundtrip() {
        run_compressor_test::<simple_bitencoded::SimpleBitencodedInputHistory>();
    }

    #[test]
    fn deltas_bitencoded_roundtrip() {
        run_compressor_test::<deltas_bitencoded::DeltasBitencodedInputHistory>();
    }

    #[test]
    fn deltas_bitencoded_base64str_roundtrip() {
        run_compressor_test::<deltas_bitencoded_base64str::DeltasBitencodedBase64strInputHistory>();
    }

    #[test]
    fn deltas_bitencoded_base64prefixstr_roundtrip() {
        // TODO: Fix test failure!
        run_compressor_test::<
            deltas_bitencoded_base64prefixstr::DeltasBitencodedBase64PrefixstrInputHistory,
        >();
    }
}
