use std::time::Duration;

use falling_tetromino_engine::{Configuration, ExtNonNegF64, RotationSystem, TetrominoGenerator};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameplaySettings {
    pub rotation_system: RotationSystem,
    pub tetromino_generator: TetrominoGenerator,
    pub piece_preview_count: usize,
    pub delayed_auto_shift: Duration,
    pub auto_repeat_rate: Duration,
    pub soft_drop_factor: ExtNonNegF64,
    pub line_clear_duration: Duration,
    pub spawn_delay: Duration,
    pub allow_initial_actions: bool,
    pub double_tap_move_finesse: Option<Duration>,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        let c = Configuration::default();
        Self {
            rotation_system: c.rotation_system,
            tetromino_generator: TetrominoGenerator::default(),
            piece_preview_count: c.piece_preview_count,
            delayed_auto_shift: c.delayed_auto_shift,
            auto_repeat_rate: c.auto_repeat_rate,
            soft_drop_factor: c.soft_drop_factor,
            line_clear_duration: c.line_clear_duration,
            spawn_delay: c.spawn_delay,
            allow_initial_actions: c.allow_initial_actions,
            double_tap_move_finesse: None,
        }
    }
}

impl GameplaySettings {
    pub fn extra_finesse() -> GameplaySettings {
        GameplaySettings {
            delayed_auto_shift: Duration::from_millis(110),
            auto_repeat_rate: Duration::from_millis(0),
            piece_preview_count: 6,
            ..Self::default()
        }
    }

    pub fn guideline() -> GameplaySettings {
        GameplaySettings {
            rotation_system: RotationSystem::Super,
            tetromino_generator: TetrominoGenerator::bag(),
            piece_preview_count: 3,
            delayed_auto_shift: Duration::from_millis(167), // ≈ 0.3s
            auto_repeat_rate: Duration::from_millis(33),    // ≈ 0.5s / 8
            soft_drop_factor: ExtNonNegF64::new(20.0).unwrap(), // = 20
            line_clear_duration: Duration::from_millis(200), // (See spawn_delay.)
            spawn_delay: Duration::from_millis(50), // (Should be =0.2s but use that for line clear duration.)
            allow_initial_actions: true,
            double_tap_move_finesse: None,
        }
    }

    pub fn nes() -> GameplaySettings {
        GameplaySettings {
            rotation_system: RotationSystem::ClassicR,
            tetromino_generator: TetrominoGenerator::Uniform,
            delayed_auto_shift: Duration::from_millis(266), // ≈ 16 /60.0988
            piece_preview_count: 1,
            auto_repeat_rate: Duration::from_millis(100), // ≈ 6 /60.0988
            spawn_delay: Duration::from_millis(166),      // ≈ 10(~18) /60.0988
            line_clear_duration: Duration::from_millis(283), // ≈ 17(~20) /60.0988
            soft_drop_factor: ExtNonNegF64::new(20.0).unwrap(), // ≈ 60.0988 * (1/2 G)
            allow_initial_actions: false,
            double_tap_move_finesse: None,
        }
    }

    pub fn gameboy() -> GameplaySettings {
        GameplaySettings {
            rotation_system: RotationSystem::ClassicL,
            tetromino_generator: TetrominoGenerator::Uniform,
            delayed_auto_shift: Duration::from_millis(400), // ≈ 24 /59.73
            piece_preview_count: 1,
            auto_repeat_rate: Duration::from_millis(150), // ≈ 9 /59.73
            spawn_delay: Duration::from_millis(33),       // ≈ 2 /59.73
            line_clear_duration: Duration::from_millis(1500), // ≈ 91 /59.73
            soft_drop_factor: ExtNonNegF64::new(5.0).unwrap(), // !≈ 59.73 * (1/3 G)
            allow_initial_actions: false,
            double_tap_move_finesse: None,
        }
    }
}
