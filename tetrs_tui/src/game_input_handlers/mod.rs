pub mod combo_bot;
pub mod crossterm;

pub enum InputSignal {
    AbortProgram,
    ForfeitGame,
    Pause,
    WindowResize,
    TakeSnapshot,
    ButtonInput(tetrs_engine::Button, bool, std::time::Instant),
}
