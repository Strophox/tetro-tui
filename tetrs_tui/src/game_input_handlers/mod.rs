pub mod combo_bot;
pub mod crossterm;

pub type InputOrInterrupt = Result<(std::time::Instant, tetrs_engine::Button, bool), Interrupt>;

pub enum Interrupt {
    WindowResize,
    Pause,
    ForfeitGame,
    ExitProgram,
}
