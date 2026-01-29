pub mod combo_bot;
pub mod live_terminal;

pub enum InputSignal {
    AbortProgram,
    ForfeitGame,
    Pause,
    WindowResize,
    StoreSavepoint,
    StoreSeed,
    Blindfold,
    ButtonInput(tetrs_engine::ButtonChange, std::time::Instant),
}
