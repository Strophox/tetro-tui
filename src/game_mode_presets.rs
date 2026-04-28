use std::time::Duration;

use either::Either;
use falling_tetromino_engine::{
    DelayParameters, DelayTable, ExtDuration, Game, GameBuilder, GameLimits, Stat,
};

use crate::{
    game_modding::{self, CheeseConfig, ComboConfig, SurvivalConfig},
    game_renderers::ShowStats,
};

pub struct GameModePreset {
    pub title: String,
    pub description: String,
    pub show_stats: ShowStats,
    pub stat_and_is_order_desc: (Stat, bool),
    pub build: Box<dyn Fn(&GameBuilder) -> Game>,
}

impl GameModePreset {
    pub const TITLE_SWIFT: &str = "Swift";
    pub fn swift() -> Self {
        Self {
            title: Self::TITLE_SWIFT.to_owned(),
            description: "How fast can you clear 40 lines?".to_owned(),
            show_stats: ShowStats::TIME | ShowStats::LINES | ShowStats::PIECES,
            stat_and_is_order_desc: (Stat::TimeElapsed(Duration::ZERO), true),
            build: Box::new(|builder: &GameBuilder| {
                builder
                    .clone()
                    .fall_delay_curve(Either::Left(DelayParameters::constant(
                        Duration::from_millis(667).into(),
                    )))
                    .game_limits(GameLimits::single(Stat::LinesCleared(40), true))
                    .build()
            }),
        }
    }

    pub const TITLE_REGULAR: &str = "Regular";
    pub fn regular() -> Self {
        Self {
            title: Self::TITLE_REGULAR.to_owned(),
            description: "Clear 150 lines at increasing gravity.".to_owned(),
            show_stats: ShowStats::TIME | ShowStats::LINES | ShowStats::POINTS | ShowStats::GRAVITY,
            stat_and_is_order_desc: (Stat::PointsScored(0), false),
            build: Box::new(|builder: &GameBuilder| {
                builder
                    .clone()
                    .fall_delay_curve(Either::Left(DelayParameters::standard_fall()))
                    .lock_delay_curve(Some(Either::Left(DelayParameters::standard_lock())))
                    .game_limits(GameLimits::single(Stat::LinesCleared(150), true))
                    .build()
            }),
        }
    }

    pub const TITLE_CLASSIC: &str = "CLASSIC (A-TYPE)";
    pub fn classic() -> Self {
        Self {
            title: Self::TITLE_CLASSIC.to_owned(),
            description: "'NES' Graphics Settings recommended(<-TODO: Implement).".to_owned(),
            show_stats: ShowStats::TIME | ShowStats::LINES | ShowStats::PIECES_COUNTS,
            stat_and_is_order_desc: (Stat::PointsScored(0), false),
            build: Box::new(|builder: &GameBuilder| {
                // let nes = tui_settings::GameplaySettings::nes();
                builder
                    .clone()
                    .fall_delay_curve(Either::Right(DelayTable::classic_fall()))
                    .lock_delay_curve(None)
                    .game_limits(GameLimits::single(Stat::LinesCleared(2560), true))
                    // .rotation_system(nes.rotsys)
                    // .tetromino_generator(nes.tetgen)
                    // .generate_piece_preview(nes.preview)
                    // .delayed_auto_shift(nes.das)
                    // .auto_repeat_rate(nes.arr)
                    // .delayed_soft_drop(nes.dsd)
                    // .soft_drop_rate(nes.sdr)
                    // .line_clear_duration(nes.lcd)
                    // .spawn_delay(nes.are)
                    // .allow_spawn_manipulation(nes.initsys)
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
            show_stats: ShowStats::TIME
                | ShowStats::LINES
                | ShowStats::POINTS
                | ShowStats::GRAVITY
                | ShowStats::LOCKDELAY,
            stat_and_is_order_desc: (Stat::PointsScored(0), false),
            build: Box::new(|builder: &GameBuilder| {
                builder
                    .clone()
                    .fall_delay_curve(Either::Left(DelayParameters::constant(ExtDuration::ZERO)))
                    .lock_delay_curve(Some(Either::Left(DelayParameters::standard_lock())))
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
            show_stats: ShowStats::TIME,
            stat_and_is_order_desc: (Stat::TimeElapsed(Duration::ZERO), true),
            build: Box::new(game_modding::Puzzle::build),
        }
    }

    pub const TITLE_SURVIVAL: &str = "Survival";
    pub fn survival(config: SurvivalConfig) -> Self {
        Self {
            title: Self::TITLE_SURVIVAL.to_owned(),
            description: "Survive lines that regenerate with placed pieces.".to_owned(),
            show_stats: ShowStats::TIME | ShowStats::LINES | ShowStats::PIECES,
            stat_and_is_order_desc: (Stat::LinesCleared(0), true),
            build: Box::new({
                move |builder: &GameBuilder| {
                    let mut builder = builder.clone();
                    builder.fall_delay_curve(Either::Left(DelayParameters::constant(
                        Duration::from_secs_f64(1.0).into(),
                    )));
                    game_modding::Survival::build(&builder, config)
                }
            }),
        }
    }

    pub const TITLE_CHEESE: &str = "Cheese";
    pub fn cheese(config: CheeseConfig, fall_lock_delays: (ExtDuration, ExtDuration)) -> Self {
        Self {
            title: format!(
                "{}{}",
                Self::TITLE_CHEESE,
                if let Some(limit) = config.limit {
                    format!("-{limit}")
                } else {
                    "".to_owned()
                }
            ),
            description: format!(
                "Efficiently eat through lines like cheese. Limit={:?}",
                config.limit
            ),
            show_stats: ShowStats::TIME | ShowStats::LINES | ShowStats::PIECES,
            stat_and_is_order_desc: (Stat::PiecesLocked(0), true),
            build: Box::new({
                move |builder: &GameBuilder| {
                    let mut builder = builder.clone();
                    builder
                        .fall_delay_curve(Either::Left(DelayParameters::constant(
                            fall_lock_delays.0,
                        )))
                        .lock_delay_curve(Some(Either::Left(DelayParameters::constant(
                            fall_lock_delays.1,
                        ))));
                    game_modding::Cheese::build(&builder, config)
                }
            }),
        }
    }

    pub const TITLE_COMBO: &str = "Combo";
    pub fn combo(config: ComboConfig) -> Self {
        Self {
            title: format!(
                "{}{}",
                Self::TITLE_COMBO,
                if let Some(limit) = config.limit {
                    format!("-{limit}")
                } else {
                    "".to_owned()
                }
            ),
            description: format!(
                "The first rule of Combo is Do not break the combo. Limit={:?}{}",
                config.limit,
                if config.start_layout != game_modding::Combo::LAYOUTS[0] {
                    format!(", Layout={:b}", config.start_layout)
                } else {
                    "".to_owned()
                }
            ),
            show_stats: ShowStats::TIME,
            stat_and_is_order_desc: (Stat::TimeElapsed(Duration::ZERO), true),
            build: Box::new({
                move |builder: &GameBuilder| game_modding::Combo::build(builder, config)
            }),
        }
    }

    pub const TITLE_ASCENT: &str = "Ascent";
    pub fn ascent() -> Self {
        Self {
            title: format!("{}*", Self::TITLE_ASCENT),
            description: "(experimental, req. Ocular + 180° rot.)".to_owned(),
            show_stats: ShowStats::TIME | ShowStats::POINTS,
            stat_and_is_order_desc: (Stat::PointsScored(0), false),
            build: Box::new(game_modding::Ascent::build),
        }
    }
}
