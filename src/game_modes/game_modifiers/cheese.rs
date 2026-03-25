use std::num::{NonZeroU32, NonZeroU8, NonZeroUsize};

use falling_tetromino_engine::{
    Game, GameAccess, GameBuilder, GameLimits, GameModifier, GameRng, Line, NotificationFeed, Stat,
};

use rand::seq::SliceRandom;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Cheese {
    tiles_per_line: NonZeroUsize,
    cheese_limit: Option<NonZeroU32>,

    cheese_eaten: u32,
    cheese_last_eaten: usize,
    cheese_generated: u32,
}

impl Cheese {
    pub const MOD_ID: &str = stringify!(Cheese);

    pub fn build(
        builder: &GameBuilder,
        tiles_per_line: NonZeroUsize,
        cheese_limit: Option<NonZeroU32>,
    ) -> Game {
        let modifier = Box::new(Self {
            tiles_per_line,
            cheese_limit,
            cheese_eaten: 0,
            cheese_last_eaten: 0,
            cheese_generated: 0,
        });

        builder
            .clone()
            .game_limits(match cheese_limit {
                Some(l) => GameLimits::single(Stat::LinesCleared(l.get()), true),
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
        serde_json::to_string(&(self.tiles_per_line, self.cheese_limit)).unwrap()
    }

    fn try_clone(&self) -> Result<Box<dyn GameModifier>, String> {
        Ok(Box::new(self.clone()))
    }

    fn on_game_built(&mut self, game: GameAccess) {
        let cheese_lines = Self::prng_cheese_lines(
            self.tiles_per_line,
            &self.cheese_limit,
            &mut self.cheese_generated,
            &mut game.state.rng,
        );

        for (line, cheese) in game.state.board.iter_mut().take(10).zip(cheese_lines) {
            *line = cheese;
        }
    }

    fn on_lock_post(&mut self, game: GameAccess, _feed: &mut NotificationFeed) {
        self.cheese_last_eaten = 0;

        // Check entire board.
        for line in game.state.board.iter() {
            // Check if line is complete.
            if line.iter().all(|mino| mino.is_some()) {
                // Check if line is a cheese one.
                if line
                    .iter()
                    .any(|cell| *cell == Some(NonZeroU8::try_from(254).unwrap()))
                {
                    // In theory would never underflow.
                    self.cheese_eaten += 1;
                    self.cheese_last_eaten += 1;
                }
            }
        }
    }

    fn on_lines_clear_post(&mut self, game: GameAccess, _feed: &mut NotificationFeed) {
        let cheese_lines = Self::prng_cheese_lines(
            self.tiles_per_line,
            &self.cheese_limit,
            &mut self.cheese_generated,
            &mut game.state.rng,
        );

        for cheese in cheese_lines.take(self.cheese_last_eaten) {
            game.state.board.rotate_right(1);
            game.state.board[0] = cheese;
        }

        game.state.lineclears = self.cheese_eaten;
    }
}

impl Cheese {
    fn prng_cheese_lines<'a>(
        tiles_per_line: NonZeroUsize,
        limit: &'a Option<NonZeroU32>,
        generated: &'a mut u32,
        rng: &'a mut GameRng,
    ) -> impl Iterator<Item = Line> + 'a {
        let grey_tile = Some(NonZeroU8::try_from(254).unwrap());
        std::iter::from_fn(move || {
            limit.is_none_or(|l| *generated < l.get()).then(|| {
                *generated += 1;
                let mut line = Line::default();
                for tile in line.iter_mut().take(tiles_per_line.get()) {
                    *tile = grey_tile;
                }
                // Currently completely random.
                line.shuffle(rng);

                line
            })
        })
    }
}
