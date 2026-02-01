use std::num::{NonZeroU8, NonZeroUsize};

use rand::Rng;
use tetrs_engine::{
    Game, GameBuilder, GameModFn, GameRng, Line, Modifier, Stat, UpdatePoint
};

pub const MOD_ID: &str = "cheese";

pub fn build(
    builder: &GameBuilder,
    linelimit: Option<NonZeroUsize>,
    gapsize: usize,
    gravity: u32,
) -> Game {
    let mut temp_cheese_tally = 0;
    let mut temp_normal_tally = 0;
    let mut remaining_lines = linelimit.unwrap_or(NonZeroUsize::MAX).get();
    let mut init = false;
    let mod_function: Box<GameModFn> = Box::new(move |point, _config, _init_vals, state, _phase, _msgs| {
        if !init {
            let mut line_source = random_gap_lines(gapsize, &mut state.rng, &mut remaining_lines);
            for (line, cheese) in state.board.iter_mut().take(10).rev().zip(&mut line_source) {
                *line = cheese;
            }
            init = true;
        } else if matches!(point, UpdatePoint::PieceLocked) {
            for line in state.board.iter() {
                if line.iter().all(|mino| mino.is_some()) {
                    let is_cheese_line = line
                        .iter()
                        .any(|cell| *cell == Some(NonZeroU8::try_from(254).unwrap()));
                    if is_cheese_line {
                        temp_cheese_tally += 1;
                    } else {
                        temp_normal_tally += 1;
                    }
                }
            }
        }
        if matches!(point, UpdatePoint::LinesCleared) {
            state.lines_cleared -= temp_normal_tally;
            let line_source = random_gap_lines(gapsize, &mut state.rng, &mut remaining_lines);
            for cheese in line_source.take(temp_cheese_tally) {
                state.board.rotate_right(1);
                state.board[0] = cheese;
            }
            temp_cheese_tally = 0;
            temp_normal_tally = 0;
        }
    });
    builder
        .clone()
        .initial_gravity(gravity)
        .progressive_gravity(false)
        .end_conditions(match linelimit {
            Some(c) => vec![(Stat::LinesCleared(c.get()), true)],
            None => vec![],
        })
        .build_modded([Modifier {
            descriptor: format!(
                "{MOD_ID}\n{}",
                serde_json::to_string(&(linelimit, gapsize, gravity)).unwrap()
            ),
            mod_function,
        }])
}

fn random_gap_lines<'a>(
    gapsize: usize,
    rng: &'a mut GameRng,
    remaining: &'a mut usize,
) -> impl Iterator<Item = Line> + 'a {
    let gap_size = gapsize.min(Game::WIDTH);
    let grey_tile = Some(NonZeroU8::try_from(254).unwrap());
    std::iter::from_fn(move || {
        if *remaining > 0 {
            *remaining -= 1;
            let mut line = [grey_tile; Game::WIDTH];
            let gap_idx = rng.random_range(0..=line.len() - gap_size);
            for i in 0..gap_size {
                line[gap_idx + i] = None;
            }
            Some(line)
        } else {
            None
        }
    })
}
