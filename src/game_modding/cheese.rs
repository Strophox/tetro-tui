use std::num::{NonZeroU32, NonZeroUsize};

use falling_tetromino_engine::{
    Game, GameAccess, GameBuilder, GameLimits, GameModifier, GameRng, Line, NotificationFeed, Stat,
};

use rand::seq::SliceRandom;

use crate::settings::Palette;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Cheese {
    // Modifier configuration.
    holes_per_line: NonZeroUsize,
    ensure_distinct_holes: bool,
    cheese_limit: Option<NonZeroU32>,

    // Modifier state fields.
    cheese_eaten_up: u32,
    temp_last_clear_actual_cheese_lines: usize,
    cheese_generated: u32,
    last_hole_pattern_generated: Vec<usize>,
}

impl Cheese {
    pub const MOD_ID: &str = stringify!(Cheese); // lol.

    pub fn build(
        builder: &GameBuilder,
        holes_per_line: NonZeroUsize,
        ensure_distinct_holes: bool,
        cheese_limit: Option<NonZeroU32>,
    ) -> Game {
        let modifier = Box::new(Self {
            holes_per_line,
            ensure_distinct_holes,
            cheese_limit,

            cheese_eaten_up: 0,
            temp_last_clear_actual_cheese_lines: 0,
            cheese_generated: 0,
            last_hole_pattern_generated: Vec::new(),
        });

        builder
            .clone()
            .game_limits(match cheese_limit {
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

    fn args(&self) -> String {
        serde_json::to_string(&(self.holes_per_line, self.cheese_limit)).unwrap()
    }

    fn try_clone(&self) -> Result<Box<dyn GameModifier>, String> {
        Ok(Box::new(self.clone()))
    }

    fn on_game_built(&mut self, game: GameAccess) {
        let cheese_lines = Self::prng_cheese_lines(
            self.holes_per_line,
            self.ensure_distinct_holes,
            self.cheese_limit,
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
            self.holes_per_line,
            self.ensure_distinct_holes,
            self.cheese_limit,
            &mut self.last_hole_pattern_generated,
            &mut self.cheese_generated,
            &mut game.state.rng,
        );

        for cheese in cheese_lines.take(self.temp_last_clear_actual_cheese_lines) {
            game.state.board.rotate_right(1);
            game.state.board[0] = cheese;
        }

        game.state.points = self.cheese_eaten_up;
    }
}

impl Cheese {
    fn prng_cheese_lines<'a>(
        holes_per_line: NonZeroUsize,
        ensure_distinct_holes: bool,
        limit: Option<NonZeroU32>,
        last_hole_pattern_generated: &'a mut Vec<usize>,
        generated: &'a mut u32,
        rng: &'a mut GameRng,
    ) -> impl Iterator<Item = Line> + 'a {
        std::iter::from_fn(move || {
            limit.is_none_or(|l| *generated < l.get()).then(|| {
                *generated += 1;
                let mut line = Line::default();
                for tile in line
                    .iter_mut()
                    .take(Game::WIDTH.saturating_sub(holes_per_line.get()))
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
                    if !ensure_distinct_holes
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
