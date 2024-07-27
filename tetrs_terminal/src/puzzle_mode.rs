use std::{collections::VecDeque, num::NonZeroU32, time::Duration};

use tetrs_engine::{
    Feedback, FeedbackEvents, Game, GameConfig, GameOver, GameState, Gamemode, InternalEvent, Stat,
    Tetromino,
};

pub fn make_game() -> Game {
    const SPEED_LEVEL: NonZeroU32 = NonZeroU32::MIN.saturating_add(1);
    const MAX_PUZZLE_ATTEMPTS: usize = 3;
    let mut init = false;
    #[allow(non_snake_case)]
    // SAFETY: 255 > 0.
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
        ("1.1 I-spin", vec![
            b"OOOOO OOOO",
            b"OOOOO OOOO",
            b"OOOOO OOOO",
            b"OOOOO OOOO",
            b"OOOO    OO",
            ], VecDeque::from([Tetromino::I,Tetromino::I])),
        ("1.2 I-spin", vec![
            b"OOOOO  OOO",
            b"OOOOO OOOO",
            b"OOOOO OOOO",
            b"OO    OOOO",
            ], VecDeque::from([Tetromino::I,Tetromino::J])),
        ("1.3 I-spin Triple", vec![
            b"OO  O   OO",
            b"OO    OOOO",
            b"OOOO OOOOO",
            b"OOOO OOOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::I,Tetromino::L,Tetromino::O,])),
        ("1.4 I-spin trial", vec![
            b"OOOOO  OOO",
            b"OOO OO OOO",
            b"OOO OO OOO",
            b"OOO     OO",
            b"OOO OOOOOO",
            ], VecDeque::from([Tetromino::I,Tetromino::I,Tetromino::L,])),
        // S/Z-spins.
        ("2.1 S-spin", vec![
            b"OOOO  OOOO",
            b"OOO  OOOOO",
            ], VecDeque::from([Tetromino::S,])),
        ("2.2 S-spins", vec![
            b"OOOO    OO",
            b"OOO    OOO",
            b"OOOOO  OOO",
            b"OOOO  OOOO",
            ], VecDeque::from([Tetromino::S,Tetromino::S,Tetromino::S,])),
        ("2.3 Z-spin galore", vec![
            b"O  OOOOOOO",
            b"OO  OOOOOO",
            b"OOO  OOOOO",
            b"OOOO  OOOO",
            b"OOOOO  OOO",
            b"OOOOOO  OO",
            b"OOOOOOO  O",
            b"OOOOOOOO  ",
            ], VecDeque::from([Tetromino::Z,Tetromino::Z,Tetromino::Z,Tetromino::Z,])),
        ("2.4 SuZ-spin trial", vec![
            b"OOOO  OOOO",
            b"OOO  OOOOO",
            b"OO    OOOO",
            b"OO    OOOO",
            b"OOO    OOO",
            b"OO  OO  OO",
            ], VecDeque::from([Tetromino::S,Tetromino::S,Tetromino::I,Tetromino::I,Tetromino::Z,])),
        // L/J-spins.
        ("3.1 J-spin", vec![
            b"OO     OOO",
            b"OOOOOO OOO",
            b"OOOOO  OOO",
            ], VecDeque::from([Tetromino::J,Tetromino::I,])),
        ("3.2 L/J-spin", vec![
            b"OO      OO",
            b"OO OOOO OO",
            b"OO  OO  OO",
            ], VecDeque::from([Tetromino::J,Tetromino::L,Tetromino::I])),
        ("3.3 L-spin", vec![
            b"OOOOO OOOO",
            b"OOO   OOOO",
            ], VecDeque::from([Tetromino::L,])),
        ("3.4 L/J-spin trial", vec![
            b"O   OO   O",
            b"O O OO O O",
            b"O   OO   O",
            ], VecDeque::from([Tetromino::J,Tetromino::L,Tetromino::J,Tetromino::L,])),
        // L/J-turns.
        ("4.1 L-turn", vec![
            b"OOOO  OOOO",
            b"OOOO  OOOO",
            b"OOOO   OOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::L,Tetromino::O,])),
        ("4.3 77-turn", vec![
            b"OOOO  OOOO",
            b"OOOOO OOOO",
            b"OOO   OOOO",
            b"OOOO OOOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::L,Tetromino::L,])),
        ("4.2 L-turn revisited", vec![
            b"OOOOO  OOO",
            b"OOO    OOO",
            b"OOOO OOOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::L,Tetromino::O,])),
        ("4.4 L-turn trial", vec![
            b"OOOO  OOOO",
            b"OOOO  OOOO",
            b"OO     OOO",
            b"OOO  OOOOO",
            b"OOO OOOOOO",
            ], VecDeque::from([Tetromino::L,Tetromino::L,Tetromino::O,])),
        // T-spins.
        ("5.1 T-spin", vec![
            b"OOOO    OO",
            b"OOO   OOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::I])),
        ("5.2 T-spin", vec![
            b"OOOO    OO",
            b"OOO   OOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::L])),
        ("5.3 T-turn", vec![
            b"OOO   OOOO",
            b"OOOO  OOOO",
            b"OOOO   OOO",
            ], VecDeque::from([Tetromino::T,Tetromino::T])),
        ("5.4 Tetrs T-spin", vec![
            b"OOO  OOOOO",
            b"OOO  OOOOO",
            b"OOOO   OOO",
            b"OOOOO OOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::O])),
        ("5.5 Tetrs T-spin Triple", vec![
            b"OOO   OOOO",
            b"OOO  OOOOO",
            b"OOOO   OOO",
            b"OOOOO OOOO",
            b"OOOOO  OOO",
            b"OOOOO OOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::J,Tetromino::L])),
    ];
    let mut current_puzzle = 0;
    let mut current_puzzle_attempt = 0;
    let mut current_puzzle_ptimelimit = 0;
    let puzzles_len = puzzles.len();
    let game_modifier = move |upcoming_event: Option<InternalEvent>,
                              config: &mut GameConfig,
                              state: &mut GameState,
                              feedback_events: &mut FeedbackEvents| {
        // Initialize internal game state.
        if !init {
            config.preview_count = 1;
            init = true;
        }
        // Puzzle may have failed.
        let game_ptimestamp = usize::try_from(state.pieces_played.iter().sum::<u32>()).unwrap();
        // All pieces for this puzzle were used up, handle puzzle outcome.
        if upcoming_event == Some(InternalEvent::Spawn)
            && game_ptimestamp == current_puzzle_ptimelimit
        {
            let puzzle_done = state
                .board
                .iter()
                .all(|line| line.iter().all(|cell| cell.is_none()));
            if !puzzle_done && current_puzzle_attempt >= MAX_PUZZLE_ATTEMPTS {
                // Run out of attempts, game over.
                state.level = NonZeroU32::try_from(u32::try_from(current_puzzle).unwrap()).unwrap();
                state.finished = Some(Err(GameOver::Fail));
            } else {
                // Change puzzle number or repeat attempt.
                if puzzle_done {
                    current_puzzle += 1;
                    current_puzzle_attempt = 1;
                } else {
                    current_puzzle_attempt += 1;
                }
                if current_puzzle == puzzles.len() {
                    // Done with all puzzles, game completed.
                    state.level =
                        NonZeroU32::try_from(u32::try_from(puzzles_len).unwrap()).unwrap();
                    state.finished = Some(Ok(()));
                } else {
                    // Load in new puzzle.
                    let (puzzle_name, puzzle_lines, puzzle_pieces) = &puzzles[current_puzzle];
                    current_puzzle_ptimelimit = game_ptimestamp + puzzle_pieces.len();
                    state.consecutive_line_clears = 0;
                    // Game message.
                    feedback_events.push((
                        state.game_time,
                        Feedback::Message(if current_puzzle_attempt == 1 {
                            format!("Stage: {}", puzzle_name.to_ascii_uppercase())
                        } else {
                            format!(
                                "{}.RETRY ({})",
                                current_puzzle_attempt - 1,
                                puzzle_name.to_ascii_uppercase()
                            )
                        }),
                    ));
                    // Queue pieces and lines.
                    state.next_pieces.clone_from(puzzle_pieces);
                    // Additional piece for consistent end preview.
                    state.next_pieces.push_back(Tetromino::I);
                    // Load in pieces.
                    for (puzzle_line, board_line) in puzzle_lines
                        .iter()
                        .rev()
                        .map(|line| {
                            line.map(|b| {
                                if b == b' ' {
                                    None
                                } else {
                                    Some(unsafe { NonZeroU32::new_unchecked(255) })
                                }
                            })
                        })
                        .chain(std::iter::repeat(Default::default()))
                        .zip(state.board.iter_mut())
                    {
                        *board_line = puzzle_line;
                    }
                }
            }
        } else if upcoming_event.is_none() {
            feedback_events.retain(|evt| !matches!(evt, (_, Feedback::Accolade { .. })));
        }
    };
    let mut game = Game::with_gamemode(Gamemode::custom(
        "Puzzle".to_string(),
        SPEED_LEVEL,
        false,
        Some(Stat::Level(
            NonZeroU32::try_from(u32::try_from(puzzles_len).unwrap()).unwrap(),
        )),
        Stat::Time(Duration::ZERO),
    ));
    game.set_modifier(Some(Box::new(game_modifier)));
    game
}
