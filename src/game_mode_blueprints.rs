use std::time::Duration;

use crate::core_game_engine::{
    DelayParameters, DelayTable, ExtDuration, Game, GameBuilder, GameLimits, Stat,
};
use either::Either;

use crate::{
    game_modding::{self, CheeseConfig, ComboConfig, SurvivalConfig},
    game_renderers::ShowStatsHud,
};

pub struct GameModeBlueprint {
    pub title: String,
    pub description: String,
    pub show_stats_hud: ShowStatsHud,
    pub objective_orderdescending: (Stat, bool),
    pub build: Box<dyn Fn(&GameBuilder) -> Game>,
}

impl GameModeBlueprint {
    pub const TITLE_SWIFT: &str = "Swift";
    pub fn swift() -> Self {
        let build = Box::new(|builder: &GameBuilder| {
            builder
                .clone()
                .fall_delay_curve(Either::Left(DelayParameters::constant(
                    Duration::from_millis(667).into(),
                )))
                .game_limits(GameLimits::single(Stat::LinesCleared(40), true))
                .build()
        });

        Self {
            title: Self::TITLE_SWIFT.to_owned(),
            description: "How quickly can you clear 40 lines?".to_owned(),
            show_stats_hud: ShowStatsHud::TIME | ShowStatsHud::LINES | ShowStatsHud::PIECES,
            objective_orderdescending: (Stat::TimeElapsed(Duration::ZERO), true),
            build,
        }
    }

    pub const TITLE_REGULAR: &str = "Regular";
    pub fn regular() -> Self {
        let build = Box::new(|builder: &GameBuilder| {
            builder
                .clone()
                .fall_delay_curve(Either::Left(DelayParameters::standard_fall()))
                .lock_delay_curve(Some(Either::Left(DelayParameters::standard_lock())))
                .game_limits(GameLimits::single(Stat::LinesCleared(160), true))
                .build()
        });

        Self {
            title: Self::TITLE_REGULAR.to_owned(),
            description: "Clear 160 lines as gravity increases.".to_owned(),
            show_stats_hud: ShowStatsHud::TIME
                | ShowStatsHud::LINES
                | ShowStatsHud::POINTS
                | ShowStatsHud::GRAVITY,
            objective_orderdescending: (Stat::PointsScored(0), false),
            build,
        }
    }

    pub const TITLE_CLASSIC: &str = "Classic";
    pub fn classic(lvl_offset: u32, easier_lock_delay: bool) -> Self {
        let build = Box::new(move |builder: &GameBuilder| {
            let lvl_offset = lvl_offset.min(29);
            let fall_delay_curve = if lvl_offset > 0 {
                let mut delay_table_entries = DelayTable::classic_fall().entries().clone();
                let start_delay = delay_table_entries[lvl_offset as usize];
                delay_table_entries[..lvl_offset as usize].fill(start_delay);
                Either::Right(DelayTable::new(delay_table_entries).unwrap())
            } else {
                Either::Right(DelayTable::classic_fall())
            };
            let lock_delay_curve = if easier_lock_delay {
                // let lock_delay  = DelayTable::classic_fall().entries()[7];
                let lock_delay = DelayTable::classic_fall().entries()[4];
                let lock_delay_table = DelayTable::new(vec![lock_delay]).unwrap();
                Some(Either::Right(lock_delay_table))
            } else {
                None
            };
            let nes = crate::settings::GameplayPreferences::nes();
            builder
                .clone()
                .fall_delay_curve(fall_delay_curve)
                .lock_delay_curve(lock_delay_curve)
                .game_limits(GameLimits::single(Stat::LinesCleared(2560), true))
                .rotation_system(nes.rotsys)
                .tetromino_generator(nes.tetgen)
                .generate_piece_preview(nes.preview)
                .delayed_auto_shift(nes.das)
                .auto_repeat_rate(nes.arr)
                .delayed_soft_drop(nes.dsd)
                .soft_drop_rate(nes.sdr)
                .line_clear_duration(nes.lcd)
                .spawn_delay(nes.are)
                // .allow_spawn_manipulation(nes.initsys)
                .build()
        });

        Self {
            title: format!(
                "{}{} (lvl {}+)",
                Self::TITLE_CLASSIC,
                if easier_lock_delay { "*" } else { "" },
                lvl_offset
            ),
            description: "Oldschool - 'NES' Graphics recommended.".to_owned(),
            show_stats_hud: ShowStatsHud::TIME
                | ShowStatsHud::LINES
                | ShowStatsHud::POINTS
                | ShowStatsHud::PIECES_COUNTS,
            objective_orderdescending: (Stat::PointsScored(0), false),
            build,
        }
    }

    // pub fn time_trial() -> GameModePreset {// (
    //     game_mode_blueprints::time_trial(),
    //     "How many points can you score in 3min.?".to_owned(),

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
            description: "Clear 320 lines at instant gravity.".to_owned(),
            show_stats_hud: ShowStatsHud::TIME
                | ShowStatsHud::LINES
                | ShowStatsHud::POINTS
                | ShowStatsHud::GRAVITY
                | ShowStatsHud::LOCKDELAY,
            objective_orderdescending: (Stat::PointsScored(0), false),
            build: Box::new(|builder: &GameBuilder| {
                builder
                    .clone()
                    .fall_delay_curve(Either::Left(DelayParameters::constant(ExtDuration::ZERO)))
                    .lock_delay_curve(Some(Either::Left(DelayParameters::standard_lock())))
                    .game_limits(GameLimits::single(Stat::LinesCleared(320), true))
                    .build()
            }),
        }
    }

    pub const TITLE_PUZZLE: &str = "Puzzle";
    pub fn puzzle() -> Self {
        let build = Box::new(|builder: &GameBuilder| {
            builder
                .clone()
                .fall_delay_curve(Either::Left(DelayParameters::constant(
                    Duration::from_millis(1000).into(),
                )))
                .generate_piece_preview(0)
                .rotation_system(crate::core_game_engine::MiscPceRots::Ocular)
                .build_modded(vec![Box::new(game_modding::Puzzle::new())])
        });
        Self {
            title: Self::TITLE_PUZZLE.to_owned(),
            description: "Clear 24 hand-crafted puzzles (feat.Ocular rot.)".to_owned(),
            show_stats_hud: ShowStatsHud::TIME,
            objective_orderdescending: (Stat::TimeElapsed(Duration::ZERO), true),
            build,
        }
    }

    pub const TITLE_PLACEMENT_PRACTICE: &str = "Placement";
    pub fn placement_practice() -> Self {
        let build = Box::new(|builder: &GameBuilder| {
            builder
                .clone()
                .fall_delay_curve(Either::Left(DelayParameters::constant(
                    Duration::from_millis(1000).into(),
                ))) /*.game_limits(GameLimits::single(Stat, is_win))*/
                .build_modded(vec![
                    Box::new(game_modding::PlacementPractice::new()),
                    Box::new(game_modding::DisplayFinesse::new()),
                ])
        });

        Self {
            title: Self::TITLE_PLACEMENT_PRACTICE.to_owned(),
            description: "Practice placing pieces.".to_owned(),
            show_stats_hud: ShowStatsHud::TIME
                | ShowStatsHud::LINES
                | ShowStatsHud::PIECES
                | ShowStatsHud::PIECES_COUNTS,
            objective_orderdescending: (Stat::LinesCleared(0), true),
            build,
        }
    }

    pub const TITLE_SURVIVAL: &str = "Survival";
    pub fn survival(config: SurvivalConfig) -> Self {
        let build = Box::new({
            move |builder: &GameBuilder| {
                builder
                    .clone()
                    .fall_delay_curve(Either::Left(DelayParameters::constant(
                        Duration::from_secs_f64(1.0).into(),
                    )))
                    .build_modded(vec![Box::new(game_modding::Survival::with_cfg(config))])
            }
        });

        Self {
            title: Self::TITLE_SURVIVAL.to_owned(),
            description: "Lines regenerate as you place more pieces.".to_owned(),
            show_stats_hud: ShowStatsHud::TIME | ShowStatsHud::LINES | ShowStatsHud::PIECES,
            objective_orderdescending: (Stat::LinesCleared(0), true),
            build,
        }
    }

    pub const TITLE_CHEESE: &str = "Cheese";
    pub fn cheese(config: CheeseConfig, fall_lock_delays: (ExtDuration, ExtDuration)) -> Self {
        let build = Box::new({
            move |builder: &GameBuilder| {
                builder
                    .clone()
                    .fall_delay_curve(Either::Left(DelayParameters::constant(fall_lock_delays.0)))
                    .lock_delay_curve(Some(Either::Left(DelayParameters::constant(
                        fall_lock_delays.1,
                    ))))
                    .build_modded(vec![Box::new(game_modding::Cheese::with_cfg(config))])
            }
        });

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
                "Efficiently eat through some lines. Limit={:?}",
                config.limit
            ),
            show_stats_hud: ShowStatsHud::TIME | ShowStatsHud::LINES | ShowStatsHud::PIECES,
            objective_orderdescending: (Stat::PiecesLocked(0), true),
            build,
        }
    }

    pub const TITLE_COMBO: &str = "Combo";
    pub fn combo(config: ComboConfig) -> Self {
        let build = Box::new({
            move |builder: &GameBuilder| {
                builder.build_modded(vec![Box::new(game_modding::Combo::with_cfg(config))])
            }
        });

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
                "Rule #1 of Combo is don't break the combo. Limit={:?}{}",
                config.limit,
                if config.start_layout != game_modding::Combo::LAYOUTS[0] {
                    format!(", Layout={:b}", config.start_layout)
                } else {
                    "".to_owned()
                }
            ),
            show_stats_hud: ShowStatsHud::TIME,
            objective_orderdescending: (Stat::TimeElapsed(Duration::ZERO), true),
            build,
        }
    }

    pub const TITLE_ASCENT: &str = "Ascent";
    pub fn ascent() -> Self {
        let build = Box::new(|builder: &GameBuilder| {
            builder
                .clone()
                .rotation_system(crate::core_game_engine::MiscPceRots::Ocular)
                .lock_delay_curve(Some(Either::Left(DelayParameters::constant(
                    ExtDuration::Infinite,
                ))))
                .game_limits(GameLimits::single(
                    Stat::TimeElapsed(Duration::from_secs(2 * 60)),
                    true,
                ))
                .build_modded(vec![Box::new(game_modding::Ascent::new())])
        });

        Self {
            title: Self::TITLE_ASCENT.to_owned(),
            description: "Experimental gamemode (requires 180° rot.)".to_owned(),
            show_stats_hud: ShowStatsHud::TIME | ShowStatsHud::POINTS,
            objective_orderdescending: (Stat::PointsScored(0), false),
            build,
        }
    }
}
