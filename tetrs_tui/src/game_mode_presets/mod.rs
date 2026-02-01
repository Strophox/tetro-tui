use std::{num::NonZeroUsize, time::Duration};

use tetrs_engine::{Game, GameBuilder, Stat};

pub mod mods;

pub type GameModePreset = (String, (Stat, bool), Box<dyn Fn(&GameBuilder) -> Game>);

pub fn forty_lines() -> GameModePreset {
    (
        "40-Lines".to_owned(),
        (Stat::TimeElapsed(Duration::ZERO), true),
        Box::new(|builder: &GameBuilder| {
            builder
                .clone()
                .initial_gravity(3)
                .progressive_gravity(false)
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
                .initial_gravity(1)
                .progressive_gravity(true)
                .end_conditions(vec![(Stat::GravityReached(16), true)])
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
                .initial_gravity(3)
                .progressive_gravity(false)
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
                .initial_gravity(Game::INSTANT_GRAVITY)
                .progressive_gravity(true)
                .end_conditions(vec![(
                    Stat::GravityReached(Game::INSTANT_GRAVITY + 16),
                    true,
                )])
                .build()
        }),
    )
}

pub fn puzzle() -> GameModePreset {
    (
        "Puzzle".to_owned(),
        (Stat::TimeElapsed(Duration::ZERO), true),
        Box::new(mods::puzzle::build),
    )
}

pub fn n_cheese(linelimit: Option<NonZeroUsize>, gapsize: usize, gravity: u32) -> GameModePreset {
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
            move |builder: &GameBuilder| mods::cheese::build(builder, linelimit, gapsize, gravity)
        }),
    )
}

pub fn n_combo(linelimit: Option<NonZeroUsize>, startlayout: u16) -> GameModePreset {
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
                    .initial_gravity(1)
                    .progressive_gravity(false)
                    .end_conditions(match linelimit {
                        Some(c) => vec![(Stat::LinesCleared(c.get()), true)],
                        None => vec![],
                    })
                    .build_modded([mods::combo_board::modifier(startlayout)])
            }
        }),
    )
}

pub fn ascent() -> GameModePreset {
    (
        "*Ascent".to_owned(),
        (Stat::PointsScored(0), false),
        Box::new(mods::ascent::build),
    )
}
