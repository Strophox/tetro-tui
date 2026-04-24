use std::time::Duration;

use either::Either;
use falling_tetromino_engine::{Configuration, ExtDuration, ExtNonNegF64, StdPceRot, StdTetGen};

use crate::tui_settings::SlotMachine;

#[serde_with::serde_as]
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameplaySettings {
    pub rotsys: StdPceRot,
    pub tetgen: StdTetGen,
    pub preview: usize,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub das: Duration,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub arr: Duration,
    pub sdf: Either<ExtNonNegF64, ExtDuration>,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub lcd: Duration,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub are: Duration,
    pub initsys: bool,
    #[serde_as(as = "Option<serde_with::DurationSecondsWithFrac<f64>>")]
    pub dtapfinesse: Option<Duration>,
}

pub fn gameplay_settings_presets() -> SlotMachine<GameplaySettings> {
    let slots = vec![
        ("Default".to_owned(), GameplaySettings::default()),
        ("Guideline".to_owned(), GameplaySettings::guideline()),
        ("Finesse+".to_owned(), GameplaySettings::extra_finesse()),
        ("NES".to_owned(), GameplaySettings::nes()),
        ("Gameboy".to_owned(), GameplaySettings::gameboy()),
        (
            "Elektronika 60".to_owned(),
            GameplaySettings::elektronika_60(),
        ),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Gameplay".to_owned())
}

impl Default for GameplaySettings {
    fn default() -> Self {
        let c = Configuration::default();
        Self {
            rotsys: c.rotation_system,
            tetgen: StdTetGen::default(),
            preview: c.generate_piece_preview,
            das: c.delayed_auto_shift,
            arr: c.auto_repeat_rate,
            sdf: c.soft_drop_speedup,
            lcd: c.line_clear_duration,
            are: c.spawn_delay,
            initsys: c.allow_spawn_manipulation,
            dtapfinesse: None,
        }
    }
}

impl GameplaySettings {
    pub fn extra_finesse() -> GameplaySettings {
        GameplaySettings {
            das: Duration::from_millis(110),
            arr: Duration::from_millis(0),
            preview: 6,
            ..Self::default()
        }
    }

    pub fn guideline() -> GameplaySettings {
        GameplaySettings {
            rotsys: StdPceRot::Super,
            tetgen: StdTetGen::bag(),
            preview: 3,
            das: Duration::from_millis(167), // ≈ 0.3s
            arr: Duration::from_millis(33),  // ≈ 0.5s / 8
            sdf: Either::Left(ExtNonNegF64::new(20.0).unwrap()), // = 20
            lcd: Duration::from_millis(200), // (See spawn_delay.)
            are: Duration::from_millis(50), // (Should be =0.2s but use that for line clear duration.)
            initsys: true,
            dtapfinesse: None,
        }
    }

    pub fn nes() -> GameplaySettings {
        GameplaySettings {
            rotsys: StdPceRot::ClassicR,
            tetgen: StdTetGen::classic(),
            das: Duration::from_millis(266), // ≈ 16 /60.0988
            preview: 1,
            arr: Duration::from_millis(100), // ≈ 6 /60.0988
            are: Duration::from_millis(250), // ≈ [10~)15(~18] /60.0988
            lcd: Duration::from_millis(333), // ≈ [17~)20 /60.0988
            sdf: Either::Left(ExtNonNegF64::new(20.0).unwrap()), // ≈ 60.0988 * (1/2 G) TODO
            initsys: false,
            dtapfinesse: None,
        }
    }

    pub fn gameboy() -> GameplaySettings {
        GameplaySettings {
            rotsys: StdPceRot::ClassicL,
            tetgen: StdTetGen::uniform(),
            das: Duration::from_millis(400), // ≈ 24 /59.73
            preview: 1,
            arr: Duration::from_millis(150),  // ≈ 9 /59.73
            are: Duration::from_millis(33),   // ≈ 2 /59.73
            lcd: Duration::from_millis(1500), // ≈ 91 /59.73
            sdf: Either::Left(ExtNonNegF64::new(5.0).unwrap()), // !≈ 59.73 * (1/3 G) TODO
            initsys: false,
            dtapfinesse: None,
        }
    }

    pub fn elektronika_60() -> GameplaySettings {
        GameplaySettings {
            rotsys: StdPceRot::ClassicL,
            tetgen: StdTetGen::uniform(),
            das: Duration::from_secs(60 * 60 * 24 * 356), // ≈ No DAS/ARR
            preview: 1,
            arr: Duration::from_secs(60 * 60 * 24 * 356), // ≈ No DAS/ARR
            are: Duration::from_millis(0),                // ≈ ?
            lcd: Duration::from_millis(400),              // ≈ ?
            sdf: Either::Left(ExtNonNegF64::new(1.0).unwrap()), // ≈ No Soft Drop TODO
            initsys: false,
            dtapfinesse: None,
        }
    }
}
