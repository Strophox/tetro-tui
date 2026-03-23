use std::{
    num::{NonZeroU32, NonZeroUsize},
    time::Duration,
};

use falling_tetromino_engine::{DelayParameters, ExtDuration, Game, GameBuilder, GameLimits, Stat};

pub mod game_modifiers;

// Name, (Stat-to-sort-by, is-order-desc), game-builder-struct-finalizer).
pub struct GameMode {
    pub title: String,
    pub description: String,
    pub stat_and_order_desc: (Stat, bool),
    pub build: Box<dyn Fn(&GameBuilder) -> Game>,
}

impl GameMode {
    pub const TITLE_SWIFT: &str = "Swift";
    pub fn swift() -> Self {
        Self {
            title: Self::TITLE_SWIFT.to_owned(),
            description: "How fast can you clear 40 lines?".to_owned(),
            stat_and_order_desc: (Stat::TimeElapsed(Duration::ZERO), true),
            build: Box::new(|builder: &GameBuilder| {
                builder
                    .clone()
                    .fall_delay_params(DelayParameters::constant(Duration::from_millis(667).into()))
                    .game_limits(GameLimits::single(Stat::LinesCleared(40), true))
                    .build()
            }),
        }
    }

    pub const TITLE_CLASSIC: &str = "Classic";
    pub fn classic() -> Self {
        Self {
            title: Self::TITLE_CLASSIC.to_owned(),
            description: "Clear 150 lines at increasing gravity.".to_owned(),
            stat_and_order_desc: (Stat::PointsScored(0), false),
            build: Box::new(|builder: &GameBuilder| {
                builder
                    .clone()
                    .fall_delay_params(DelayParameters::standard_fall())
                    .lock_delay_params(DelayParameters::standard_lock())
                    .game_limits(GameLimits::single(Stat::LinesCleared(150), true))
                    .build()
            }),
        }
    }

    // pub fn time_trial() -> GameModePreset {// (
    //     game_mode_presets::time_trial(),
    //     "What highscore can you get in 3min.?".to_owned(),

    //     (
    //         "Time Trial".to_owned(),
    //         (Stat::PointsScored(0), false),
    //         Box::new(|builder: &GameBuilder| {
    //             builder
    //                 .clone()
    //                 .fall_delay_params(DelayParameters::constant(Duration::from_millis(667).into()))
    //                 .end_conditions(vec![(Stat::TimeElapsed(Duration::from_secs(3 * 60)), true)])
    //                 .build()
    //         }),
    //     )
    // }

    pub const TITLE_MASTER: &str = "Master";
    pub fn master() -> Self {
        Self {
            title: Self::TITLE_MASTER.to_owned(),
            description: "Clear 150 lines at instant gravity.".to_owned(),
            stat_and_order_desc: (Stat::PointsScored(0), false),
            build: Box::new(|builder: &GameBuilder| {
                builder
                    .clone()
                    .fall_delay_params(DelayParameters::constant(ExtDuration::ZERO))
                    .lock_delay_params(DelayParameters::standard_lock())
                    .game_limits(GameLimits::single(Stat::LinesCleared(150), true))
                    .build()
            }),
        }
    }

    pub const TITLE_PUZZLE: &str = "Puzzle";
    pub fn puzzle() -> Self {
        Self {
            title: Self::TITLE_PUZZLE.to_owned(),
            description: "Clear 24 hand-crafted puzzles.".to_owned(),
            stat_and_order_desc: (Stat::TimeElapsed(Duration::ZERO), true),
            build: Box::new(game_modifiers::Puzzle::build),
        }
    }

    pub const TITLE_CHEESE: &str = "Cheese";
    pub fn cheese(
        cheese_tiles_per_line: NonZeroUsize,
        cheese_limit: Option<NonZeroU32>,
        fall_lock_delays: (ExtDuration, ExtDuration),
    ) -> Self {
        Self {
            title: format!(
                "{}{}",
                Self::TITLE_CHEESE,
                if let Some(limit) = cheese_limit {
                    format!("-{limit}")
                } else {
                    "".to_owned()
                }
            ),
            description: format!(
                "Eat through lines like Swiss cheese. Limit={:?}",
                cheese_limit
            ),
            stat_and_order_desc: (Stat::PiecesLocked(0), true),
            build: Box::new({
                move |builder: &GameBuilder| {
                    let mut builder = builder.clone();
                    builder
                        .fall_delay_params(DelayParameters::constant(fall_lock_delays.0))
                        .lock_delay_params(DelayParameters::constant(fall_lock_delays.1));
                    game_modifiers::Cheese::build(&builder, cheese_tiles_per_line, cheese_limit)
                }
            }),
        }
    }

    pub const TITLE_COMBO: &str = "Combo";
    pub fn combo(initial_layout: u16, combo_limit: Option<NonZeroU32>) -> Self {
        Self {
            title: format!(
                "{}{}",
                Self::TITLE_COMBO,
                if let Some(limit) = combo_limit {
                    format!("-{limit}")
                } else {
                    "".to_owned()
                }
            ),
            description: format!(
                "Get consecutive line clears. Limit={:?}{}",
                combo_limit,
                if initial_layout != game_modifiers::Combo::LAYOUTS[0] {
                    format!(", Layout={:b}", initial_layout)
                } else {
                    "".to_owned()
                }
            ),
            stat_and_order_desc: (Stat::TimeElapsed(Duration::ZERO), true),
            build: Box::new({
                move |builder: &GameBuilder| {
                    game_modifiers::Combo::build(builder, initial_layout, combo_limit)
                }
            }),
        }
    }

    pub const TITLE_ASCENT: &str = "Ascent";
    pub fn ascent() -> Self {
        Self {
            title: format!("{}*", Self::TITLE_ASCENT),
            description: "(experimental, req. Ocular + 180° rot.)".to_owned(),
            stat_and_order_desc: (Stat::PointsScored(0), false),
            build: Box::new(game_modifiers::Ascent::build),
        }
    }
}
