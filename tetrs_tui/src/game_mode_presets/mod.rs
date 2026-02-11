use std::{
    num::{NonZeroU32, NonZeroUsize},
    time::Duration,
};

use tetrs_engine::{DelayParameters, ExtDuration, Game, GameBuilder, Stat};

pub mod game_modifiers;

pub type GameModePreset = (String, (Stat, bool), Box<dyn Fn(&GameBuilder) -> Game>);

pub fn forty_lines() -> GameModePreset {
    (
        "40-Lines".to_owned(),
        (Stat::TimeElapsed(Duration::ZERO), true),
        Box::new(|builder: &GameBuilder| {
            builder
                .clone()
                .fall_delay_params(DelayParameters::constant(Duration::from_millis(667).into()))
                .end_conditions(vec![(Stat::LinesCleared(40), true)])
                .build()
        }),
    )
}

pub fn marathon() -> GameModePreset {
    (
        "Marathon".to_owned(),
        (Stat::PointsScored(0), false),
        Box::new(|builder: &GameBuilder| {
            builder
                .clone()
                .fall_delay_params(DelayParameters::standard_fall())
                .lock_delay_params(DelayParameters::standard_lock())
                .end_conditions(vec![(Stat::LinesCleared(150), true)])
                .build()
        }),
    )
}

pub fn time_trial() -> GameModePreset {
    (
        "Time Trial".to_owned(),
        (Stat::PointsScored(0), false),
        Box::new(|builder: &GameBuilder| {
            builder
                .clone()
                .fall_delay_params(DelayParameters::constant(Duration::from_millis(667).into()))
                .end_conditions(vec![(Stat::TimeElapsed(Duration::from_secs(3 * 60)), true)])
                .build()
        }),
    )
}

pub fn master() -> GameModePreset {
    (
        "Master".to_owned(),
        (Stat::PointsScored(0), false),
        Box::new(|builder: &GameBuilder| {
            builder
                .clone()
                .fall_delay_params(DelayParameters::constant(ExtDuration::ZERO))
                .lock_delay_params(DelayParameters::standard_lock())
                .end_conditions(vec![(Stat::LinesCleared(150), true)])
                .build()
        }),
    )
}

pub fn puzzle() -> GameModePreset {
    (
        "Puzzle".to_owned(),
        (Stat::TimeElapsed(Duration::ZERO), true),
        Box::new(game_modifiers::puzzle::build),
    )
}

pub fn n_cheese(
    linelimit: Option<NonZeroU32>,
    cheese_tiles_per_line: NonZeroUsize,
    fall_delay: ExtDuration,
) -> GameModePreset {
    (
        format!(
            "{}Cheese",
            if let Some(limit) = linelimit {
                format!("{limit}-")
            } else {
                "".to_owned()
            }
        ),
        (Stat::PiecesLocked(0), true),
        Box::new({
            move |builder: &GameBuilder| {
                game_modifiers::cheese::build(builder, linelimit, cheese_tiles_per_line, fall_delay)
            }
        }),
    )
}

pub fn n_combo(linelimit: Option<NonZeroU32>, startlayout: u16) -> GameModePreset {
    (
        format!(
            "{}Combo",
            if let Some(limit) = linelimit {
                format!("{limit}-")
            } else {
                "".to_owned()
            }
        ),
        (Stat::TimeElapsed(Duration::ZERO), true),
        Box::new({
            move |builder: &GameBuilder| {
                builder
                    .clone()
                    .fall_delay_params(DelayParameters::constant(
                        Duration::from_millis(1000).into(),
                    ))
                    .end_conditions(match linelimit {
                        Some(l) => vec![(Stat::LinesCleared(l.get()), true)],
                        None => vec![],
                    })
                    .build_modded([game_modifiers::combo_board::modifier(startlayout)])
            }
        }),
    )
}

pub fn ascent() -> GameModePreset {
    (
        "*Ascent".to_owned(),
        (Stat::PointsScored(0), false),
        Box::new(game_modifiers::ascent::build),
    )
}
