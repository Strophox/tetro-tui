use std::num::{NonZeroU32, NonZeroUsize};

use falling_tetromino_engine::{DelayParameters, ExtDuration, Stat};

use crate::game_modding::Combo;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameModePreferences {
    pub custom_fall_params: DelayParameters,
    pub custom_lock_params: DelayParameters,
    pub custom_win_condition: Option<Stat>,
    pub custom_seed: Option<u64>,
    pub custom_start_board: Option<String>, // For more compact serialization of NewGameSettings, we store an encoded `Board` (see `encode_board`).

    pub cheese_fall_and_lock_delays: (ExtDuration, ExtDuration),
    pub cheese_holes_per_line: NonZeroUsize,
    pub cheese_ensure_distinct_holes: bool,
    pub cheese_limit: Option<NonZeroU32>,

    /// Custom starting layout when playing Combo mode (4-wide rows), encoded as binary.
    /// Example: '▀▄▄▀' => 0b_1001_0110 = 150
    pub combo_start_layout: u16,
    pub combo_limit: Option<NonZeroU32>,

    pub master_mode_unlocked: bool,
    pub experimental_mode_unlocked: bool,
}

impl Default for GameModePreferences {
    fn default() -> Self {
        Self {
            custom_fall_params: DelayParameters::standard_fall(),
            custom_lock_params: DelayParameters::standard_lock(),
            custom_win_condition: None,
            custom_seed: None,
            custom_start_board: None,

            cheese_fall_and_lock_delays: (ExtDuration::Infinite, ExtDuration::Infinite),
            cheese_holes_per_line: NonZeroUsize::MIN,
            cheese_ensure_distinct_holes: true,
            cheese_limit: Some(NonZeroU32::try_from(20).unwrap()),

            combo_limit: Some(NonZeroU32::try_from(30).unwrap()),
            combo_start_layout: Combo::LAYOUTS[0],

            master_mode_unlocked: false,
            experimental_mode_unlocked: false,
        }
    }
}
