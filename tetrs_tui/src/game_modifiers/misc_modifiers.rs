pub mod custom_start_board {
    use tetrs_engine::Modifier;

    pub const MOD_ID: &str = "custom_start_board";

    pub fn modifier(encoded_board: &str) -> Modifier {
        let board = crate::application::NewGameSettings::decode_board(encoded_board);
        let mut init = false;
        Modifier {
            descriptor: format!("{MOD_ID}\n{encoded_board}"),
            mod_function: Box::new(move |_config, _rules, state, _modpoint, _msgs| {
                if !init {
                    state.board.clone_from(&board);
                    init = true;
                }
            }),
        }
    }
}

// NOTE: Can be / was used for debugging.
#[allow(dead_code)]
pub mod show_recency_tetromino_likelihood {
    use tetrs_engine::{
        piece_generation::TetrominoSource, Feedback, GameEvent, ModificationPoint, Modifier,
        Tetromino,
    };

    pub const MOD_ID: &str = "show_recency_tetromino_likelihood";

    pub fn modifier() -> Modifier {
        Modifier {
            descriptor: MOD_ID.to_owned(),
            mod_function: Box::new(|config, _rules, state, modpoint, msgs| {
                if !matches!(modpoint, ModificationPoint::AfterEvent(GameEvent::Spawn)) {
                    return;
                }
                let TetrominoSource::Recency {
                    last_generated,
                    snap: _,
                } = config.tetromino_generation
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
}
