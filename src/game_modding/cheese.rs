use std::num::{NonZeroU32, NonZeroUsize};

use falling_tetromino_engine::{
    Game, GameAccess, GameBuilder, GameLimits, GameModifier, GameRng, Line, NotificationFeed, Stat,
    WIDTH,
};

use rand::seq::SliceRandom;

use crate::{savefile_logic::to_savefile_string, tui_settings::Palette};

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Cheese {
    // Modifier configuration.
    config: CheeseConfig,
    // Modifier state fields.
    cheese_eaten_up: u32,
    temp_last_clear_actual_cheese_lines: usize,
    cheese_generated: u32,
    last_hole_pattern_generated: Vec<usize>,
    cached_stats: [String; 1],
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct CheeseConfig {
    pub holes_per_line: NonZeroUsize,
    pub ensure_distinct_holes: bool,
    pub limit: Option<NonZeroU32>,
}

impl Default for CheeseConfig {
    fn default() -> Self {
        Self {
            holes_per_line: NonZeroUsize::MIN,
            ensure_distinct_holes: true,
            limit: Some(NonZeroU32::try_from(20).unwrap()),
        }
    }
}

impl Cheese {
    pub const MOD_ID: &str = stringify!(Cheese); // lol.

    pub fn build(builder: &GameBuilder, config: CheeseConfig) -> Game {
        let modifier = Box::new(Self {
            config,
            cheese_eaten_up: 0,
            temp_last_clear_actual_cheese_lines: 0,
            cheese_generated: 0,
            last_hole_pattern_generated: Vec::new(),
            cached_stats: [format!("Cheese eaten: {}", 0)],
        });

        builder
            .clone()
            .game_limits(match config.limit {
                Some(c) => GameLimits::single(Stat::PointsScored(c.get()), true),
                None => GameLimits::new(),
            })
            .build_modded(vec![modifier])
    }
}

impl GameModifier for Cheese {
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
        let cheese_lines = Self::prng_cheese_lines(
            &self.config,
            &mut self.last_hole_pattern_generated,
            &mut self.cheese_generated,
            &mut game.state.rng,
        );

        for (line, cheese) in game.state.board.iter_mut().take(10).zip(cheese_lines) {
            *line = cheese;
        }
    }

    fn on_lock_post(&mut self, game: GameAccess, _feed: &mut NotificationFeed) {
        self.temp_last_clear_actual_cheese_lines = 0;

        // Check entire board.
        for line in game.state.board.iter() {
            // Check if line is complete.
            if line.iter().all(|mino| mino.is_some()) {
                // Check if line is a cheese one.
                if line.contains(&Some(Palette::GRAY)) {
                    // In theory would never underflow.
                    self.cheese_eaten_up += 1;
                    self.temp_last_clear_actual_cheese_lines += 1;
                }
            }
        }
    }

    fn on_lines_clear_post(&mut self, game: GameAccess, _feed: &mut NotificationFeed) {
        let cheese_lines = Self::prng_cheese_lines(
            &self.config,
            &mut self.last_hole_pattern_generated,
            &mut self.cheese_generated,
            &mut game.state.rng,
        );

        for cheese in cheese_lines.take(self.temp_last_clear_actual_cheese_lines) {
            game.state.board.rotate_right(1);
            game.state.board[0] = cheese;
        }

        self.cached_stats[0] = format!("Cheese eaten: {}", self.cheese_eaten_up);
        game.state.points = self.cheese_eaten_up;
    }
}

impl Cheese {
    fn prng_cheese_lines<'a>(
        config: &'a CheeseConfig,
        last_hole_pattern_generated: &'a mut Vec<usize>,
        generated: &'a mut u32,
        rng: &'a mut GameRng,
    ) -> impl Iterator<Item = Line> + 'a {
        std::iter::from_fn(move || {
            config.limit.is_none_or(|l| *generated < l.get()).then(|| {
                *generated += 1;
                let mut line = Line::default();
                for tile in line
                    .iter_mut()
                    .take(WIDTH.saturating_sub(config.holes_per_line.get()))
                {
                    *tile = Some(Palette::GRAY);
                }
                // Currently completely random.
                loop {
                    line.shuffle(rng);
                    let hole_pattern_generated: Vec<usize> = line
                        .iter()
                        .enumerate()
                        .filter_map(|(i, x)| x.is_some().then_some(i))
                        .collect();
                    if !config.ensure_distinct_holes
                        || hole_pattern_generated != *last_hole_pattern_generated
                        || hole_pattern_generated.len() == line.len()
                    // If the lines we generate are wholly empty (and cannot possibly be different), give up.
                    {
                        *last_hole_pattern_generated = hole_pattern_generated;
                        break;
                    }
                }

                line
            })
        })
    }
}
