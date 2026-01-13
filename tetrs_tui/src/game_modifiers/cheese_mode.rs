use std::num::{NonZeroU8, NonZeroUsize};

use rand::Rng;

use tetrs_engine::{FnGameMod, Game, GameEvent, GameMode, Limits, Line, ModifierPoint};

fn random_gap_lines(gap_size: usize) -> impl Iterator<Item = Line> {
    let gap_size = gap_size.min(Game::WIDTH);
    let grey_tile = Some(NonZeroU8::try_from(254).unwrap());
    let mut rng = rand::rng();
    std::iter::from_fn(move || {
        let mut line = [grey_tile; Game::WIDTH];
        let gap_idx = rng.random_range(0..=line.len() - gap_size);
        for i in 0..gap_size {
            line[gap_idx + i] = None;
        }
        Some(line)
    })
}

fn is_cheese_line(line: &Line) -> bool {
    line.iter()
        .any(|cell| *cell == Some(NonZeroU8::try_from(254).unwrap()))
}

pub fn new_game(cheese_limit: Option<NonZeroUsize>, gap_size: usize, gravity: u32) -> Game {
    let mut line_source =
        random_gap_lines(gap_size).take(cheese_limit.unwrap_or(NonZeroUsize::MAX).get());
    let mut temp_cheese_tally = 0;
    let mut temp_normal_tally = 0;
    let mut init = false;
    let cheese_mode: FnGameMod = Box::new(
        move |_config, _mode, state, _rng, _feedback_events, modifier_point| {
            if !init {
                for (line, cheese) in state.board.iter_mut().take(10).rev().zip(&mut line_source) {
                    *line = cheese;
                }
                init = true;
            } else if matches!(
                modifier_point,
                ModifierPoint::BeforeEvent(GameEvent::LineClear)
            ) {
                for line in state.board.iter() {
                    if line.iter().all(|mino| mino.is_some()) {
                        if is_cheese_line(line) {
                            temp_cheese_tally += 1;
                        } else {
                            temp_normal_tally += 1;
                        }
                    }
                }
            }
            if matches!(
                modifier_point,
                ModifierPoint::AfterEvent(GameEvent::LineClear)
            ) {
                state.lines_cleared -= temp_normal_tally;
                for cheese in line_source.by_ref().take(temp_cheese_tally) {
                    state.board.insert(0, cheese);
                }
                temp_cheese_tally = 0;
                temp_normal_tally = 0;
            }
        },
    );
    Game::builder(GameMode {
        name: Some("Cheese".to_string()),
        initial_gravity: gravity,
        increase_gravity: false,
        limits: Limits {
            lines: cheese_limit.map(|line_count| (true, line_count.get())),
            ..Limits::default()
        },
    })
    .build_modified([cheese_mode])
}
