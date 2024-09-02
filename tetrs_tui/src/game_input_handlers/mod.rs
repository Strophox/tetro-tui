pub mod crossterm_handler;

pub type InputOrInterrupt = Result<(std::time::Instant, tetrs_engine::Button, bool), Interrupt>;

pub enum Interrupt {
    WindowResize,
    Pause,
    ForfeitGame,
    ExitProgram,
}