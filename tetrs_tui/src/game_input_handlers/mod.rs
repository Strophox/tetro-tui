pub mod combo_bot;
pub mod terminal;

pub enum InputSignal {
    AbortProgram,
    ForfeitGame,
    Pause,
    WindowResize,
    StoreSavepoint,
    StoreSeed,
    StoreBoard,
    ButtonInput(tetrs_engine::Button, bool, std::time::Instant),
}
