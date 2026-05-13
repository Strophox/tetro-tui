use std::time::Duration;

use crate::tetromino_engine::{
    Configuration, ExtDuration, ExtNonNegF64, MiscPceRots, MiscTetGens, SoftDropRate,
};
use either::Either;

use crate::settings::SlotMachine;

#[serde_with::serde_as]
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameplayPreferences {
    pub rotsys: MiscPceRots,
    pub tetgen: MiscTetGens,
    pub preview: usize,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub das: Duration,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub arr: Duration,
    #[serde_as(as = "Option<serde_with::DurationSecondsWithFrac<f64>>")]
    pub dsd: Option<Duration>,
    pub sdr: SoftDropRate,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub lcd: Duration,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub are: Duration,
    pub initsys: bool,
    #[serde_as(as = "Option<serde_with::DurationSecondsWithFrac<f64>>")]
    pub dtapfinesse: Option<Duration>,
}

pub fn gameplay_settings_presets() -> SlotMachine<GameplayPreferences> {
    let slots = vec![
        ("Default".to_owned(), GameplayPreferences::default()),
        ("Guideline".to_owned(), GameplayPreferences::guideline()),
        ("Finesse+".to_owned(), GameplayPreferences::extra_finesse()),
        ("NES".to_owned(), GameplayPreferences::nes()),
        ("Gameboy".to_owned(), GameplayPreferences::gameboy()),
        (
            "Elektronika 60".to_owned(),
            GameplayPreferences::elektronika_60(),
        ),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Gameplay".to_owned())
}

impl Default for GameplayPreferences {
    fn default() -> Self {
        let c = Configuration::default();
        Self {
            rotsys: c.rotation_system,
            tetgen: MiscTetGens::default(),
            preview: c.generate_piece_preview,
            das: c.delayed_auto_shift,
            arr: c.auto_repeat_rate,
            dsd: c.delayed_soft_drop,
            sdr: c.soft_drop_rate,
            lcd: c.line_clear_duration,
            are: c.spawn_delay,
            initsys: c.allow_spawn_manipulation,
            dtapfinesse: None,
        }
    }
}

impl GameplayPreferences {
    pub fn extra_finesse() -> GameplayPreferences {
        GameplayPreferences {
            das: Duration::from_millis(110),
            arr: Duration::from_millis(0),
            sdr: Either::Right(ExtDuration::ZERO),
            preview: 6,
            ..Self::default()
        }
    }

    pub fn guideline() -> GameplayPreferences {
        GameplayPreferences {
            rotsys: MiscPceRots::Super,
            tetgen: MiscTetGens::bag(),
            preview: 3,
            das: Duration::from_millis(167), // ≈ 0.3s
            arr: Duration::from_millis(33),  // ≈ 0.5s / 8
            dsd: None,
            sdr: Either::Left(ExtNonNegF64::new(20.0).unwrap()), // = 20
            lcd: Duration::from_millis(200),                     // (See spawn_delay.)
            are: Duration::from_millis(50), // (Should be =0.2s but use that for line clear duration.)
            initsys: true,
            dtapfinesse: None,
        }
    }

    pub fn nes() -> GameplayPreferences {
        GameplayPreferences {
            rotsys: MiscPceRots::ClassicR,
            tetgen: MiscTetGens::classic(),
            das: Duration::from_millis(266), // ≈ 16 /60.0988
            preview: 1,
            arr: Duration::from_millis(100), // ≈ 6 /60.0988
            are: Duration::from_millis(250), // ≈ [10~)15(~18] /60.0988
            lcd: Duration::from_millis(333), // ≈ [17~)20 /60.0988
            dsd: Some(Duration::from_millis(50)),
            sdr: Either::Right(Duration::from_millis(33).into()), // ≈ 60.0988 * (1/2 G)
            initsys: false,
            dtapfinesse: None,
        }
    }

    pub fn gameboy() -> GameplayPreferences {
        GameplayPreferences {
            rotsys: MiscPceRots::ClassicL,
            tetgen: MiscTetGens::uniform(),
            das: Duration::from_millis(400), // ≈ 24 /59.73
            preview: 1,
            arr: Duration::from_millis(150),  // ≈ 9 /59.73
            are: Duration::from_millis(33),   // ≈ 2 /59.73
            lcd: Duration::from_millis(1500), // ≈ 91 /59.73
            dsd: None,
            sdr: Either::Right(Duration::from_millis(50).into()), // ≈ 59.73 * (1/3 G)
            initsys: false,
            dtapfinesse: None,
        }
    }

    pub fn elektronika_60() -> GameplayPreferences {
        GameplayPreferences {
            rotsys: MiscPceRots::ClassicL,
            tetgen: MiscTetGens::uniform(),
            das: Duration::from_millis(350), // ≈ Originally no DAS, but we add an interesting one for better feel if needed.
            preview: 1,
            arr: Duration::from_millis(100), // ≈ Originally no ARR, but we add an interesting one for better feel if needed.
            are: Duration::from_millis(0),   // ≈ ?
            lcd: Duration::from_millis(400), // ≈ ?
            dsd: Some(Duration::from_millis(350)), // ≈ Originally no DSD, but we add an interesting one for better feel.
            sdr: Either::Right(Duration::from_millis(100).into()), // ≈ Originally no SDF, but we add an interesting one for better feel.
            initsys: false,
            dtapfinesse: None,
        }
    }
}
