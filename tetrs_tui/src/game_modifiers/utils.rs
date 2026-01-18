use tetrs_engine::{
    piece_generation::TetrominoSource, Feedback, GameEvent, ModificationPoint, Modifier, Tetromino,
};

pub fn custom_start_board(board_str: &str) -> Modifier {
    let grey_tile = Some(std::num::NonZeroU8::try_from(254).unwrap());
    let mut init = false;
    let board_str = board_str.to_owned();
    Modifier {
        identifier: "custom_start_board".to_owned(),
        mod_function: Box::new(move |_config, _rules, state, _modpoint, _msgs| {
            if !init {
                let mut chars = board_str.chars().rev();
                'init: for row in state.board.iter_mut() {
                    for cell in row.iter_mut().rev() {
                        let Some(char) = chars.next() else {
                            break 'init;
                        };
                        *cell = if char != ' ' { grey_tile } else { None };
                    }
                }
                init = true;
            }
        }),
    }
}

#[allow(dead_code)]
pub fn show_recency_tetromino_likelihood() -> Modifier {
    Modifier {
        identifier: "show_recency_tetromino_likelihood".to_owned(),
        mod_function: Box::new(|config, _rules, state, modpoint, msgs| {
            if !matches!(modpoint, ModificationPoint::AfterEvent(GameEvent::Spawn)) {
                return;
            }
            let TetrominoSource::Recency {
                last_generated,
                snap: _,
            } = config.tetromino_generator
            else {
                return;
            };
            let mut pieces_played_strs = [
                Tetromino::O,
                Tetromino::I,
                Tetromino::S,
                Tetromino::Z,
                Tetromino::T,
                Tetromino::L,
                Tetromino::J,
            ];
            pieces_played_strs.sort_by_key(|&t| last_generated[t]);
            msgs.push((
                state.time,
                Feedback::Text(
                    pieces_played_strs
                        .map(|tet| {
                            format!(
                                "{tet:?}{}{}{}",
                                last_generated[tet],
                                // "█".repeat(lg[t] as usize),
                                "█".repeat(
                                    (last_generated[tet] * last_generated[tet]) as usize / 8
                                ),
                                [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉"]
                                    [(last_generated[tet] * last_generated[tet]) as usize % 8]
                            )
                            .to_ascii_lowercase()
                        })
                        .join("")
                        .to_string(),
                ),
            ));
            // config.line_clear_delay = Duration::ZERO;
            // config.appearance_delay = Duration::ZERO;
            // state.board.remove(0);
            // state.board.push(Default::default());
            // state.board.remove(0);
            // state.board.push(Default::default());
        }),
    }
}
