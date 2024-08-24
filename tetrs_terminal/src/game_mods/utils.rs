use tetrs_engine::{
    piece_generation::TetrominoSource, Feedback, FeedbackEvents, GameConfig, GameMode, GameState,
    InternalEvent, ModifierPoint, Tetromino,
};

#[allow(dead_code)]
pub fn display_tetromino_likelihood(
    config: &mut GameConfig,
    _mode: &mut GameMode,
    state: &mut GameState,
    feedback_events: &mut FeedbackEvents,
    event: &ModifierPoint,
) {
    if !matches!(event, ModifierPoint::AfterEvent(InternalEvent::Spawn)) {
        return;
    }
    let TetrominoSource::Recency { last_generated } = config.tetromino_generator else {
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
    feedback_events.push((
        state.time,
        Feedback::Message(
            pieces_played_strs
                .map(|tet| {
                    format!(
                        "{tet:?}{}{}{}",
                        last_generated[tet],
                        // "█".repeat(lg[t] as usize),
                        "█".repeat((last_generated[tet] * last_generated[tet]) as usize / 8),
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
}
