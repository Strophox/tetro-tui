use falling_tetromino_engine::{Configuration, DelayCurve, ExtDuration, Stat};

use crate::game_modding::{CheeseConfig, ComboConfig, SurvivalConfig};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameModePreferences {
    pub custom_config: CustomModeConfig,
    pub survival_config: SurvivalConfig,
    pub cheese_config: CheeseConfig,
    pub cheese_fall_and_lock_delays: (ExtDuration, ExtDuration),
    pub combo_config: ComboConfig,
    pub unlock_master_mode: bool,
    pub unlock_classic_mode: bool,
    pub unlock_experimental_mode: bool,
}

impl Default for GameModePreferences {
    fn default() -> Self {
        Self {
            custom_config: CustomModeConfig::default(),
            survival_config: SurvivalConfig::default(),
            cheese_config: CheeseConfig::default(),
            cheese_fall_and_lock_delays: (ExtDuration::Infinite, ExtDuration::Infinite),
            combo_config: ComboConfig::default(),
            unlock_master_mode: false,
            unlock_classic_mode: false,
            unlock_experimental_mode: false,
        }
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct CustomModeConfig {
    pub seed: Option<u64>,
    pub start_board: Option<String>, // For more compact serialization of NewGameSettings, we store an encoded `Board` (see `StartBoard` mod.)
    pub fall_curve: DelayCurve,
    pub lock_curve: Option<DelayCurve>,
    pub win_condition: Option<Stat>,
}

impl Default for CustomModeConfig {
    fn default() -> Self {
        let config = Configuration::default();
        Self {
            fall_curve: config.fall_delay_curve,
            lock_curve: config.lock_delay_curve,
            win_condition: None,
            seed: None,
            start_board: None,
        }
    }
}
