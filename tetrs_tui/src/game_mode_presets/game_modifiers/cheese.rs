use std::num::{NonZeroU32, NonZeroU8, NonZeroUsize};

use rand::seq::SliceRandom;
use tetrs_engine::{
    DelayParameters, ExtDuration, Game, GameBuilder, GameModFn, GameRng, Line, Modifier, Stat,
    UpdatePoint,
};

pub const MOD_ID: &str = "cheese";

pub fn build(
    builder: &GameBuilder,
    linelimit: Option<NonZeroU32>,
    cheese_tiles_per_line: NonZeroUsize,
    fall_lock_delay: ExtDuration,
) -> Game {
    let mut temp_cheese_tally = 0;
    let mut temp_normal_tally = 0;
    let mut internal_remaining_lines = linelimit.unwrap_or(NonZeroU32::MAX).get();
    let mut init = false;
    let mod_function: Box<GameModFn> =
        Box::new(move |point, _config, _init_vals, state, _phase, _msgs| {
            if !init {
                let n_init_lines = usize::try_from(internal_remaining_lines.min(10)).unwrap();
                let mut line_source = random_gap_lines(
                    cheese_tiles_per_line,
                    &mut state.rng,
                    &mut internal_remaining_lines,
                );
                for (line, cheese) in state
                    .board
                    .iter_mut()
                    .take(n_init_lines)
                    .rev()
                    .zip(&mut line_source)
                {
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
                state.lineclears -= temp_normal_tally;
                let line_source = random_gap_lines(
                    cheese_tiles_per_line,
                    &mut state.rng,
                    &mut internal_remaining_lines,
                );
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
        .fall_delay_params(DelayParameters::constant(fall_lock_delay))
        .lock_delay_params(DelayParameters::constant(fall_lock_delay))
        .end_conditions(match linelimit {
            Some(c) => vec![(Stat::LinesCleared(c.get()), true)],
            None => vec![],
        })
        .build_modded([Modifier {
            descriptor: format!(
                "{MOD_ID}\n{}",
                serde_json::to_string(&(linelimit, cheese_tiles_per_line, fall_lock_delay))
                    .unwrap()
            ),
            mod_function,
        }])
}

fn random_gap_lines<'a>(
    cheese_tiles_per_line: NonZeroUsize,
    rng: &'a mut GameRng,
    remaining: &'a mut u32,
) -> impl Iterator<Item = Line> + 'a {
    let grey_tile = Some(NonZeroU8::try_from(254).unwrap());
    std::iter::from_fn(move || {
        if *remaining > 0 {
            *remaining -= 1;
            let mut line = Line::default();
            for tile in line.iter_mut().take(cheese_tiles_per_line.get()) {
                *tile = grey_tile;
            }
            line.shuffle(rng);
            Some(line)
        } else {
            None
        }
    })
}
