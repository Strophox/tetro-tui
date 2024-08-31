use std::num::{NonZeroU32, NonZeroU8, NonZeroUsize};

use rand::{self, prelude::SliceRandom};

use tetrs_engine::{
    FeedbackEvents, FnGameMod, Game, GameConfig, GameMode, GameState, InternalEvent, Limits, Line,
    ModifierPoint,
};

pub fn random_hole_lines() -> impl Iterator<Item = Line> {
    let grey_tile = Some(NonZeroU8::try_from(254).unwrap());
    let mut rng = rand::thread_rng();
    std::iter::from_fn(move || {
        let mut line = [grey_tile; 10];
        line[4] = None;
        line.shuffle(&mut rng);
        Some(line)
    })
}

fn is_cheese_line(line: &Line) -> bool {
    line.iter()
        .any(|cell| *cell == Some(NonZeroU8::try_from(254).unwrap()))
}

pub fn new_game(cheese_limit: Option<NonZeroUsize>) -> Game {
    let mut line_source = random_hole_lines().take(cheese_limit.unwrap_or(NonZeroUsize::MAX).get());
    let mut temp_cheese_tally = 0;
    let mut temp_normal_tally = 0;
    let mut init = false;
    let cheese_mode: FnGameMod = Box::new(
        move |_config: &mut GameConfig,
              _mode: &mut GameMode,
              state: &mut GameState,
              _feedback_events: &mut FeedbackEvents,
              modifier_point: &ModifierPoint| {
            if !init {
                for (line, cheese) in state.board.iter_mut().take(8).rev().zip(&mut line_source) {
                    *line = cheese;
                }
                init = true;
            } else if matches!(
                modifier_point,
                ModifierPoint::BeforeEvent(InternalEvent::LineClear)
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
                ModifierPoint::AfterEvent(InternalEvent::LineClear)
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
    let mut game = Game::new(GameMode {
        name: "Cheese".to_string(),
        start_level: NonZeroU32::MIN,
        increment_level: false,
        limits: Limits {
            lines: cheese_limit.map(|line_count| (true, line_count.get())),
            ..Default::default()
        },
    });
    unsafe { game.add_modifier(cheese_mode) };
    game
}
