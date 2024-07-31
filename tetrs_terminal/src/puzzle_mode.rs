use std::{collections::VecDeque, num::NonZeroU32};

use tetrs_engine::{
    Feedback, FeedbackEvents, Game, GameConfig, GameMode, GameOver, GameState, InternalEvent, Limits, ModifierPoint, Tetromino
};

const MAX_STAGE_ATTEMPTS: usize = 3;
const SPEED_LEVEL: u32 = 3;

pub fn make_game() -> Game {
    #[rustfmt::skip]
    let puzzles = [
        /* Puzzle template.
        ("puzzlename", vec![
            b"OOOOOOOOOO",
            b"OOOOOOOOOO",
            b"OOOOOOOOOO",
            b"OOOOOOOOOO",
        ], VecDeque::from([Tetromino::I,])),
        */
        // I-spins.
        ("I-spin", vec![
            b"OOOOO OOOO",
            b"OOOOO OOOO",
            b"OOOOO OOOO",
            b"OOOOO OOOO",
            b"OOOO    OO",
            ], VecDeque::from([Tetromino::I,Tetromino::I])),
        ("I-spin", vec![
            b"OOOOO  OOO",
            b"OOOOO OOOO",
            b"OOOOO OOOO",
            b"OO    OOOO",
            ], VecDeque::from([Tetromino::I,Tetromino::J])),
        ("I-spin Triple", vec![
            b"OO  O   OO",
            b"OO    OOOO",
            b"OOOO OOOOO",
            b"OOOO OOOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::I,Tetromino::L,Tetromino::O,])),
        ("I-spin trial", vec![
            b"OOOOO  OOO",
            b"OOO OO OOO",
            b"OOO OO OOO",
            b"OOO     OO",
            b"OOO OOOOOO",
            ], VecDeque::from([Tetromino::I,Tetromino::I,Tetromino::L,])),
        // S/Z-spins.
        ("S-spin", vec![
            b"OOOO  OOOO",
            b"OOO  OOOOO",
            ], VecDeque::from([Tetromino::S,])),
        ("S-spins", vec![
            b"OOOO    OO",
            b"OOO    OOO",
            b"OOOOO  OOO",
            b"OOOO  OOOO",
            ], VecDeque::from([Tetromino::S,Tetromino::S,Tetromino::S,])),
        ("Z-spin galore", vec![
            b"O  OOOOOOO",
            b"OO  OOOOOO",
            b"OOO  OOOOO",
            b"OOOO  OOOO",
            b"OOOOO  OOO",
            b"OOOOOO  OO",
            b"OOOOOOO  O",
            b"OOOOOOOO  ",
            ], VecDeque::from([Tetromino::Z,Tetromino::Z,Tetromino::Z,Tetromino::Z,])),
        ("SuZ-spins", vec![
            b"OOOO  OOOO",
            b"OOO  OOOOO",
            b"OO    OOOO",
            b"OO    OOOO",
            b"OOO    OOO",
            b"OO  OO  OO",
            ], VecDeque::from([Tetromino::S,Tetromino::S,Tetromino::I,Tetromino::I,Tetromino::Z,])),
        // L/J-spins.
        ("J-spin", vec![
            b"OO     OOO",
            b"OOOOOO OOO",
            b"OOOOO  OOO",
            ], VecDeque::from([Tetromino::J,Tetromino::I,])),
        ("L_J-spin", vec![
            b"OO      OO",
            b"OO OOOO OO",
            b"OO  OO  OO",
            ], VecDeque::from([Tetromino::J,Tetromino::L,Tetromino::I])),
        ("L-spin", vec![
            b"OOOOO OOOO",
            b"OOO   OOOO",
            ], VecDeque::from([Tetromino::L,])),
        ("L/J-spins", vec![
            b"O   OO   O",
            b"O O OO O O",
            b"O   OO   O",
            ], VecDeque::from([Tetromino::J,Tetromino::L,Tetromino::J,Tetromino::L,])),
        // L/J-turns.
        ("L-turn", vec![
            b"OOOO  OOOO",
            b"OOOO  OOOO",
            b"OOOO   OOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::L,Tetromino::O,])),
        ("77-turn", vec![
            b"OOOO  OOOO",
            b"OOOOO OOOO",
            b"OOO   OOOO",
            b"OOOO OOOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::L,Tetromino::L,])),
        ("7-turn", vec![
            b"OOOOO  OOO",
            b"OOO    OOO",
            b"OOOO OOOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::L,Tetromino::O,])),
        ("L-turn trial", vec![
            b"OOOO  OOOO",
            b"OOOO  OOOO",
            b"OO     OOO",
            b"OOO  OOOOO",
            b"OOO OOOOOO",
            ], VecDeque::from([Tetromino::L,Tetromino::L,Tetromino::O,])),
        // T-spins.
        ("T-spin Single", vec![
            b"OOOO    OO",
            b"OOO   OOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::I])),
        ("T-spin Double", vec![
            b"OOOO    OO",
            b"OOO   OOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::L])),
        ("T-tuck", vec![
            b"OOO   OOOO",
            b"OOOO  OOOO",
            b"OOOO   OOO",
            ], VecDeque::from([Tetromino::T,Tetromino::T])),
        ("Tetrs T-spin", vec![
            b"OOO  OOOOO",
            b"OOO  OOOOO",
            b"OOOO   OOO",
            b"OOOOO OOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::O])),
        ("Tetrs T-spin Triple", vec![
            b"OOO   OOOO",
            b"OOO  OOOOO",
            b"OOOO   OOO",
            b"OOOOO OOOO",
            b"OOOOO  OOO",
            b"OOOOO OOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::J,Tetromino::L])),
    ];
    let puzzles_len = puzzles.len();
    let load_puzzle = move |state: &mut GameState, attempt: usize, current_puzzle_idx: usize, feedback_events: &mut FeedbackEvents| -> usize {
        let (puzzle_name, puzzle_lines, puzzle_pieces) =
            &puzzles[current_puzzle_idx];
        // Game message.
        feedback_events.push((
            state.time,
            Feedback::Message(if attempt == 1 {
                format!(
                    "Stage {}: {}",
                    current_puzzle_idx + 1,
                    puzzle_name.to_ascii_uppercase()
                )
            } else {
                format!(
                    "{}. TRY ({})",
                    attempt,
                    puzzle_name.to_ascii_uppercase()
                )
            }),
        ));
        // Queue pieces and lines.
        state.next_pieces.clone_from(puzzle_pieces);
        // Load in pieces.
        for (puzzle_line, board_line) in puzzle_lines
            .iter()
            .rev()
            .map(|line| {
                line.map(|b| {
                    if b == b' ' {
                        None
                    } else {
                        Some(unsafe { NonZeroU32::new_unchecked(254) })
                    }
                })
            })
            .chain(std::iter::repeat(Default::default()))
            .zip(state.board.iter_mut())
        {
            *board_line = puzzle_line;
        }
        puzzle_pieces.len()
    };
    let mut init = true;
    let mut current_puzzle_idx = 0;
    let mut current_puzzle_attempt = 1;
    let mut current_puzzle_piececnt_limit = 0;
    let puzzle_modifier =
        move |config: &mut GameConfig,
              _mode: &mut GameMode,
              state: &mut GameState,
              feedback_events: &mut FeedbackEvents,
              modifier_point: &ModifierPoint| {
            let game_piececnt = usize::try_from(state.pieces_played.iter().sum::<u32>()).unwrap();
            if init {
                let piececnt = load_puzzle(state, current_puzzle_attempt, current_puzzle_idx, feedback_events);
                current_puzzle_piececnt_limit = game_piececnt + piececnt;
                init = false;
            } else if matches!(modifier_point, ModifierPoint::BeforeEvent(InternalEvent::Spawn)) && game_piececnt == current_puzzle_piececnt_limit {
                let puzzle_done = state
                    .board
                    .iter()
                    .all(|line| line.iter().all(|cell| cell.is_none()));
                // Run out of attempts, game over.
                if !puzzle_done && current_puzzle_attempt == MAX_STAGE_ATTEMPTS {
                    state.end = Some(Err(GameOver::ModeLimit));
                } else {
                    if puzzle_done {
                        current_puzzle_idx += 1;
                        current_puzzle_attempt = 1;
                    } else {
                        current_puzzle_attempt += 1;
                    }
                    if current_puzzle_idx == puzzles_len {
                        // Done with all puzzles, game completed.
                        state.end = Some(Ok(()));
                    } else {
                        // Load in new puzzle.
                        let piececnt = load_puzzle(state, current_puzzle_attempt, current_puzzle_idx, feedback_events);
                        current_puzzle_piececnt_limit = game_piececnt + piececnt;
                    }
                }
            }
            if matches!(modifier_point, ModifierPoint::BeforeEvent(_) | ModifierPoint::BeforeButtonChange(_, _)) {
                config.preview_count = 0;
                state.level = NonZeroU32::try_from(SPEED_LEVEL).unwrap();
            } else {
                config.preview_count = state.next_pieces.len();
                state.level =
                    NonZeroU32::try_from(u32::try_from(current_puzzle_idx + 1).unwrap()).unwrap();
                // Delete accolades.
                feedback_events.retain(|evt| !matches!(evt, (_, Feedback::Accolade { .. })));
            }
            // Remove spurious spawn.
            if matches!(modifier_point, ModifierPoint::AfterEvent(InternalEvent::Spawn)) && state.end.is_some() {
                state.active_piece_data = None;
            }
        };
    let mut game = Game::new(GameMode {
        name: "Puzzle".to_string(),
        start_level: NonZeroU32::MIN.saturating_add(1),
        increment_level: false,
        limits: Limits {
            level: Some((true, NonZeroU32::try_from(u32::try_from(puzzles_len).unwrap()).unwrap())),
            ..Default::default()
        },
    });
    game.config_mut().preview_count = 0;
    unsafe { game.add_modifier(Box::new(puzzle_modifier)) };
    game
}
