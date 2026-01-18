pub mod combo_bot_input_handler;
pub mod terminal_input_handler;

pub enum InputSignal {
    AbortProgram,
    ForfeitGame,
    Pause,
    WindowResize,
    StoreSavepoint,
    ButtonInput(tetrs_engine::Button, bool, std::time::Instant),
}
