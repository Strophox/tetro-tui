use std::num::NonZeroUsize;

use falling_tetromino_engine::{
    Game, GameAccess, GameBuilder, GameEndCause, GameModifier, Line, NotificationFeed, Phase,
};

use crate::{
    game_modding::{Cheese, CheeseConfig},
    savefile_logic::to_savefile_string,
};

#[derive(PartialEq, PartialOrd, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Survival {
    // Modifier configuration.
    config: SurvivalConfig,

    // Modifier state fields.
    piece_budget: f64,

    // Fields needed to use Cheese:prng_cheese_lines
    cheese_config: CheeseConfig,
    cheese_generated: u32,
    last_hole_pattern_generated: Vec<usize>,

    cached_stats: [String; 1],
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct SurvivalConfig {
    pub holes_per_line: NonZeroUsize,
    pub ensure_distinct_holes: bool,
}

impl Default for SurvivalConfig {
    fn default() -> Self {
        Self {
            holes_per_line: NonZeroUsize::MIN,
            ensure_distinct_holes: false,
        }
    }
}

impl Survival {
    pub const MOD_ID: &str = stringify!(Survival);

    pub fn build(builder: &GameBuilder, config: SurvivalConfig) -> Game {
        let modifier = Box::new(Self {
            config,

            piece_budget: 0.0,

            cheese_generated: 0,
            last_hole_pattern_generated: Vec::new(),
            cheese_config: CheeseConfig {
                holes_per_line: config.holes_per_line,
                ensure_distinct_holes: config.ensure_distinct_holes,
                limit: None,
            },

            cached_stats: [Self::fmt_regen_period(0)],
        });

        builder.build_modded(vec![modifier])
    }
}

impl GameModifier for Survival {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn cfg(&self) -> String {
        to_savefile_string(&(self.config)).unwrap()
    }

    fn stats(&self) -> &[String] {
        &self.cached_stats
    }

    fn try_clone(&self) -> Result<Box<dyn GameModifier>, String> {
        Ok(Box::new(self.clone()))
    }

    fn on_game_built(&mut self, game: GameAccess) {
        // Initialize board with some lines, otherwise the empty board seems weird.

        let cheese_lines = Cheese::cheese_lines(
            &self.cheese_config,
            &mut self.last_hole_pattern_generated,
            &mut self.cheese_generated,
            &mut game.state.rng,
        );

        for (line, cheese) in game.state.board.iter_mut().take(1).zip(cheese_lines) {
            *line = cheese;
        }
    }

    fn on_lock_post(&mut self, mut game: GameAccess, _feed: &mut NotificationFeed) {
        self.piece_budget += 1.0;
        // Check entire board for a complete line:
        // If a line clear will occur, do not regenerate lines yet.
        for line in game.state.board.iter() {
            if line.iter().all(|mino| mino.is_some()) {
                return;
            }
        }

        // Possibly regenerate lines.
        self.try_regenerate_lines(&mut game);
        self.cached_stats[0] = Self::fmt_regen_period(game.state.lineclears);
    }

    fn on_lines_clear_post(&mut self, mut game: GameAccess, _feed: &mut NotificationFeed) {
        if f64::from(game.state.lineclears) >= Survival::LINECLEARS_LIMIT {
            *game.phase = Phase::GameEnd {
                cause: GameEndCause::Custom(format!(
                    "Survived {} lines",
                    Survival::LINECLEARS_LIMIT
                )),
                is_win: true,
            };
            return;
        }

        self.try_regenerate_lines(&mut game);
        self.cached_stats[0] = Self::fmt_regen_period(game.state.lineclears);
    }
}

impl Survival {
    const LINECLEARS_LIMIT: f64 = 300.0;
    fn fmt_regen_period(lineclears: u32) -> String {
        format!(
            "Regen. period: {:.01}",
            Self::calc_regeneration_period(lineclears)
        )
    }

    // The speed curve is vaguely inspired by tetr*s.js enhanced.
    fn calc_regeneration_period(lineclears: u32) -> f64 {
        // We want it to:
        // - Generate 1 cheese line / 8 pieces at the beginning.
        // - Gen. 1 cheese / 4 pieces at 150 lines.
        // - Gen. 1 cheese / 2 pieces at 300 lines.
        const ORIGIN_PERIOD: f64 = 8.0;
        const TARGET_PERIOD: f64 = 2.0;

        // Round to lowest multiple of 10 so it's not too erratic.
        let trunc10_lineclears = lineclears - lineclears % 10;

        // When lineclears == 0, then exponent == 0.0, and multiplier stays exactly `() * 1.0`
        // When lineclears == LIMIT, then exponent == 1.0 and multiplier becomes exactly `() * TARGET / ORIGIN`
        let raw_regen_period = ORIGIN_PERIOD
            * (TARGET_PERIOD / ORIGIN_PERIOD)
                .powf(f64::from(trunc10_lineclears) / Survival::LINECLEARS_LIMIT);

        // Round regen period to halves for neatness.
        (raw_regen_period * 2.0).round() / 2.0
    }

    fn try_regenerate_lines(&mut self, game: &mut GameAccess) {
        let regen_period = Self::calc_regeneration_period(game.state.lineclears);
        let mut cheese_lines = Cheese::cheese_lines(
            &self.cheese_config,
            &mut self.last_hole_pattern_generated,
            &mut self.cheese_generated,
            &mut game.state.rng,
        );

        while self.piece_budget >= regen_period {
            self.piece_budget -= regen_period;

            let Some(mut cheese_line) = cheese_lines.next() else {
                // Our cheese_config says limit = None, so this shouldn't happen.
                unreachable!()
            };

            // Cycle everything up.
            game.state.board.rotate_right(1);
            // Swap out lowest row.
            std::mem::swap(&mut game.state.board[0], &mut cheese_line);

            if cheese_line != Line::default() {
                *game.phase = Phase::GameEnd {
                    cause: GameEndCause::BufferOut {
                        overflowing_lines: vec![cheese_line],
                    },
                    is_win: false,
                };
                return;
            }
        }
    }
}
