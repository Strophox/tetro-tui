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

    cheese_remaining: u32,
    full_lines_pre_lineclear: usize,
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
            cheese_remaining: cheese_limit.unwrap_or(NonZeroU32::MAX).get(),
            full_lines_pre_lineclear: 0,
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
        let n_init_lines = usize::try_from(self.cheese_remaining.min(10)).unwrap();

        let mut cheese_lines = Self::prng_cheese_lines(
            self.tiles_per_line,
            &mut self.cheese_remaining,
            &mut game.state.rng,
        );

        for (line, cheese) in game
            .state
            .board
            .iter_mut()
            .take(n_init_lines)
            .rev()
            .zip(&mut cheese_lines)
        {
            *line = cheese;
        }
    }

    fn on_lock_post(&mut self, game: GameAccess, _feed: &mut NotificationFeed) {
        self.full_lines_pre_lineclear = 0;

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
                    self.cheese_remaining = self.cheese_remaining.saturating_sub(1);
                }

                self.full_lines_pre_lineclear += 1;
            }
        }
    }

    fn on_lines_clear_post(&mut self, game: GameAccess, _feed: &mut NotificationFeed) {
        let cheese_lines = Self::prng_cheese_lines(
            self.tiles_per_line,
            &mut self.cheese_remaining,
            &mut game.state.rng,
        );

        for cheese in cheese_lines.take(self.full_lines_pre_lineclear) {
            game.state.board.rotate_right(1);
            game.state.board[0] = cheese;
        }
    }
}

impl Cheese {
    fn prng_cheese_lines<'a>(
        cheese_tiles_per_line: NonZeroUsize,
        remaining: &'a mut u32,
        rng: &'a mut GameRng,
    ) -> impl Iterator<Item = Line> + 'a {
        let grey_tile = Some(NonZeroU8::try_from(254).unwrap());
        std::iter::from_fn(move || {
            if *remaining > 0 {
                *remaining -= 1;
                let mut line = Line::default();
                for tile in line.iter_mut().take(cheese_tiles_per_line.get()) {
                    *tile = grey_tile;
                }
                // Currently completely random.
                line.shuffle(rng);
                Some(line)
            } else {
                None
            }
        })
    }
}
