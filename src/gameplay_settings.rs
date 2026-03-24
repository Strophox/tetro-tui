use std::time::Duration;

use falling_tetromino_engine::{Configuration, ExtNonNegF64, RotationSystem, TetrominoGenerator};

use crate::application::SlotMachine;

#[serde_with::serde_as]
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameplaySettings {
    pub rotsys: RotationSystem,
    pub randomizer: TetrominoGenerator,
    pub preview: usize,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub das: Duration,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub arr: Duration,
    pub sdf: ExtNonNegF64,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub lcd: Duration,
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    pub are: Duration,
    pub initsys: bool,
    #[serde_as(as = "Option<serde_with::DurationSecondsWithFrac<f64>>")]
    pub dtapfinesse: Option<Duration>,
}

pub fn default_gameplay_slots() -> SlotMachine<GameplaySettings> {
    let slots = vec![
        ("Default".to_owned(), GameplaySettings::default()),
        ("Finesse+".to_owned(), GameplaySettings::extra_finesse()),
        ("Guideline".to_owned(), GameplaySettings::guideline()),
        ("NES".to_owned(), GameplaySettings::nes()),
        ("Gameboy".to_owned(), GameplaySettings::gameboy()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Gameplay".to_owned())
}

impl Default for GameplaySettings {
    fn default() -> Self {
        let c = Configuration::default();
        Self {
            rotsys: c.rotation_system,
            randomizer: TetrominoGenerator::default(),
            preview: c.piece_preview_count,
            das: c.delayed_auto_shift,
            arr: c.auto_repeat_rate,
            sdf: c.soft_drop_factor,
            lcd: c.line_clear_duration,
            are: c.spawn_delay,
            initsys: c.allow_initial_actions,
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
            rotsys: RotationSystem::Super,
            randomizer: TetrominoGenerator::bag(),
            preview: 3,
            das: Duration::from_millis(167),       // ≈ 0.3s
            arr: Duration::from_millis(33),        // ≈ 0.5s / 8
            sdf: ExtNonNegF64::new(20.0).unwrap(), // = 20
            lcd: Duration::from_millis(200),       // (See spawn_delay.)
            are: Duration::from_millis(50), // (Should be =0.2s but use that for line clear duration.)
            initsys: true,
            dtapfinesse: None,
        }
    }

    pub fn nes() -> GameplaySettings {
        GameplaySettings {
            rotsys: RotationSystem::ClassicR,
            randomizer: TetrominoGenerator::Uniform,
            das: Duration::from_millis(266), // ≈ 16 /60.0988
            preview: 1,
            arr: Duration::from_millis(100),       // ≈ 6 /60.0988
            are: Duration::from_millis(166),       // ≈ 10(~18) /60.0988
            lcd: Duration::from_millis(283),       // ≈ 17(~20) /60.0988
            sdf: ExtNonNegF64::new(20.0).unwrap(), // ≈ 60.0988 * (1/2 G)
            initsys: false,
            dtapfinesse: None,
        }
    }

    pub fn gameboy() -> GameplaySettings {
        GameplaySettings {
            rotsys: RotationSystem::ClassicL,
            randomizer: TetrominoGenerator::Uniform,
            das: Duration::from_millis(400), // ≈ 24 /59.73
            preview: 1,
            arr: Duration::from_millis(150),      // ≈ 9 /59.73
            are: Duration::from_millis(33),       // ≈ 2 /59.73
            lcd: Duration::from_millis(1500),     // ≈ 91 /59.73
            sdf: ExtNonNegF64::new(5.0).unwrap(), // !≈ 59.73 * (1/3 G)
            initsys: false,
            dtapfinesse: None,
        }
    }
}
