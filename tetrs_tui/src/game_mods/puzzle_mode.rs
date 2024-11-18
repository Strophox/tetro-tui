use std::{collections::VecDeque, num::NonZeroU8};

use tetrs_engine::{
    Feedback, FeedbackEvents, FnGameMod, Game, GameConfig, GameMode, GameOver, GameState,
    InternalEvent, Limits, ModifierPoint, Tetromino,
};

const MAX_STAGE_ATTEMPTS: usize = 5;
const PUZZLE_GRAVITY: u32 = 1;

pub fn new_game() -> Game {
    let puzzles = puzzle_list();
    let puzzles_len = puzzles.len();
    let load_puzzle = move |state: &mut GameState,
                            attempt: usize,
                            current_puzzle_idx: usize,
                            feedback_events: &mut FeedbackEvents|
          -> usize {
        let (puzzle_name, puzzle_lines, puzzle_pieces) = &puzzles[current_puzzle_idx];
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
                    "{} ATT. LEFT ({})",
                    MAX_STAGE_ATTEMPTS + 1 - attempt,
                    puzzle_name.to_ascii_uppercase()
                )
            }),
        ));
        state.next_pieces.clone_from(puzzle_pieces);
        for (load_line, board_line) in puzzle_lines
            .iter()
            .rev()
            .chain(std::iter::repeat(&&[b' '; 10]))
            .zip(state.board.iter_mut())
        {
            let grey_tile = Some(NonZeroU8::try_from(254).unwrap());
            *board_line = tetrs_engine::Line::default();
            if load_line.iter().any(|c| c != &b' ') {
                for (board_cell, puzzle_tile) in board_line
                    .iter_mut()
                    .zip(load_line.iter().chain(std::iter::repeat(&b'O')))
                {
                    if puzzle_tile != &b' ' {
                        *board_cell = grey_tile;
                    }
                }
            }
        }
        puzzle_pieces.len()
    };
    let mut init = false;
    let mut current_puzzle_idx = 0;
    let mut current_puzzle_attempt = 1;
    let mut current_puzzle_piececnt_limit = 0;
    let puzzle_mode: FnGameMod = Box::new(
        move |config: &mut GameConfig,
              _mode: &mut GameMode,
              state: &mut GameState,
              feedback_events: &mut FeedbackEvents,
              modifier_point: &ModifierPoint| {
            let game_piececnt = usize::try_from(state.pieces_played.iter().sum::<u32>()).unwrap();
            if !init {
                let piececnt = load_puzzle(
                    state,
                    current_puzzle_attempt,
                    current_puzzle_idx,
                    feedback_events,
                );
                current_puzzle_piececnt_limit = game_piececnt + piececnt;
                init = true;
            } else if matches!(
                modifier_point,
                ModifierPoint::BeforeEvent(InternalEvent::Spawn)
            ) && game_piececnt == current_puzzle_piececnt_limit
            {
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
                        let piececnt = load_puzzle(
                            state,
                            current_puzzle_attempt,
                            current_puzzle_idx,
                            feedback_events,
                        );
                        current_puzzle_piececnt_limit = game_piececnt + piececnt;
                    }
                }
            }
            // FIXME: handle displaying the level to the user better.
            // Keep custom game state that's also visible to player, but hide it from the game engine that handles gameplay.
            if matches!(
                modifier_point,
                ModifierPoint::BeforeEvent(_) | ModifierPoint::BeforeButtonChange(_, _)
            ) {
                config.preview_count = 0;
                state.gravity = PUZZLE_GRAVITY;
            } else {
                config.preview_count = state.next_pieces.len();
                state.gravity = u32::try_from(current_puzzle_idx + 1).unwrap();
                // Delete accolades.
                feedback_events.retain(|evt| !matches!(evt, (_, Feedback::Accolade { .. })));
            }
            // Remove spurious spawn.
            if matches!(
                modifier_point,
                ModifierPoint::AfterEvent(InternalEvent::Spawn)
            ) && state.end.is_some()
            {
                state.active_piece_data = None;
            }
            // Remove ability to hold.
            if matches!(modifier_point, ModifierPoint::AfterButtonChange) {
                state.events.remove(&InternalEvent::HoldPiece);
            }
        },
    );
    let mut game = Game::new(GameMode {
        name: "Puzzle".to_string(),
        initial_gravity: 2,
        increase_gravity: false,
        limits: Limits::default(),
    });
    game.config_mut().preview_count = 0;
    game.add_modifier(puzzle_mode);
    game
}

#[allow(clippy::type_complexity)]
#[rustfmt::skip]
fn puzzle_list() -> [(&'static str, Vec<&'static [u8; 10]>, VecDeque<Tetromino>); 24] {
    [
        /* Puzzle template.
        ("puzzlename", vec![
            b"OOOOOOOOOO",
            b"OOOOOOOOOO",
            b"OOOOOOOOOO",
            b"OOOOOOOOOO",
        ], VecDeque::from([Tetromino::I,])),
        */
        /*("DEBUG L/J", vec![
            b" O O O O O",
            b"         O",
            b" O O O O O",
            b"         O",
            b" O O O O O",
            b"         O",
            b" O O O O O",
            b"         O",
        ], VecDeque::from([Tetromino::L,Tetromino::J])),*/
        // 4 I-spins.
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
        // 4 S/Z-spins.
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
        // 4 L/J-spins.
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
        // 4 L/J-turns.
        ("77", vec![
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
        ("L-turn", vec![
            b"OOOO  OOOO",
            b"OOOO  OOOO",
            b"OOOO   OOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::L,Tetromino::O,])),
        ("L-turn trial", vec![
            b"OOOO  OOOO",
            b"OOOO  OOOO",
            b"OO     OOO",
            b"OOO  OOOOO",
            b"OOO OOOOOO",
            ], VecDeque::from([Tetromino::L,Tetromino::L,Tetromino::O,])),
        // 7 T-spins.
        ("T-spin", vec![
            b"OOOO    OO",
            b"OOO   OOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::I])),
        ("T-spin pt.2", vec![
            b"OOOO    OO",
            b"OOO   OOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::L])),
        ("T-tuck", vec![
            b"OO   OOOOO",
            b"OOO  OOOOO",
            b"OOO   OOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::T])),
        ("T-insert", vec![
            b"OOOO  OOOO",
            b"OOOO  OOOO",
            b"OOOOO OOOO",
            b"OOOO   OOO",
            ], VecDeque::from([Tetromino::T,Tetromino::O])),
        ("T-go-round", vec![
            b"OOO  OOOOO",
            b"OOO   OOOO",
            b"OOOOO  OOO",
            b"OOOOO OOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::O])),
        ("T T-spin Setup", vec![
            b"OOOOO  OOO",
            b"OOOOO  OOO",
            b"OOO   OOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::O])),
        ("T T-spin Triple", vec![
            b"OOOO   OOO",
            b"OOOOO  OOO",
            b"OOO   OOOO",
            b"OOOO OOOOO",
            b"OOO  OOOOO",
            b"OOOO OOOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::J])),
        ("~ Finale ~", vec![ // v2.2.1
            b"OOOO  OOOO",
            b"O  O  OOOO",
            b"  OOO OOOO",
            b"OOO    OOO",
            b"OOOOOO   O",
            b"  O    OOO",
            b"OOOOO OOOO",
            b"O  O  OOOO",
            b"OOOOO OOOO",
            ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::O,Tetromino::S,Tetromino::I,Tetromino::J,Tetromino::Z])),
        // ("T-spin FINALE v2.3", vec![
        //     b"OOOO  OOOO",
        //     b"OOOO  O  O",
        //     b"OOOO OOO  ",
        //     b"OOO    OOO",
        //     b"O   OOOOOO",
        //     b"OOO    OOO",
        //     b"OOOO OOO  ",
        //     b"OOOO  O  O",
        //     b"OOOO OOOOO",
        //     ], VecDeque::from([Tetromino::T,Tetromino::J,Tetromino::O,Tetromino::Z,Tetromino::I,Tetromino::L,Tetromino::S])),
        // ("T-spin FINALE v2.2", vec![
        //     b"OOOO  OOOO",
        //     b"O  O  OOOO",
        //     b"  OOO OOOO",
        //     b"OOO    OOO",
        //     b"OOOOOO   O",
        //     b"OOO    OOO",
        //     b"  OOO OOOO",
        //     b"O  O  OOOO",
        //     b"OOOOO OOOO",
        //     ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::O,Tetromino::S,Tetromino::I,Tetromino::J,Tetromino::Z])),
        // ("T-spin FINALE v2.1", vec![
        //     b"OOOO  OOOO",
        //     b"OOOO  OOOO",
        //     b"OOOOO OOOO",
        //     b"OOO    OOO",
        //     b"OOOOOO   O",
        //     b"OOO    OOO",
        //     b"  OOO OO  ",
        //     b"O  O  OOOO",
        //     b"OOOOO O  O",
        //     ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::O,Tetromino::I,Tetromino::J,Tetromino::Z,Tetromino::S])),
        // ("T-spin FINALE v3", vec![
        //     b"OOOO  OOOO",
        //     b"OOOO  OOOO",
        //     b"OOOOO OOOO",
        //     b"OOO    OOO",
        //     b"OOOOOO   O",
        //     b"OOO    OOO",
        //     b"OOOOO OOOO",
        //     b"O  O  O  O",
        //     b"O  OO OO  ",
        //     ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::S,Tetromino::I,Tetromino::J,Tetromino::O,Tetromino::Z])),
        // ("T-spin FINALE v2", vec![
        //     b"OOOO  OOOO",
        //     b"OOOO  OOOO",
        //     b"OOOOO OOOO",
        //     b"OOO    OOO",
        //     b"OOOOOO   O",
        //     b"OOO    OOO",
        //     b"OOOOO OOOO",
        //     b"O  O  O  O",
        //     b"  OOO OO  ",
        //     ], VecDeque::from([Tetromino::T,Tetromino::L,Tetromino::O,Tetromino::I,Tetromino::J,Tetromino::Z,Tetromino::S])),
        // ("T-spin FINALE v1", vec![
        //     b"OOOO  OOOO",
        //     b"OOOO  OOOO",
        //     b"OOOOO OOOO",
        //     b"OOO     OO",
        //     b"OOOOOO   O",
        //     b"OO     O  ",
        //     b"OOOOO OOOO",
        //     b"O  O  OOOO",
        //     b"  OOO OOOO",
        //     ], VecDeque::from([Tetromino::T,Tetromino::O,Tetromino::L,Tetromino::I,Tetromino::J,Tetromino::Z,Tetromino::S])),
    ]
}
