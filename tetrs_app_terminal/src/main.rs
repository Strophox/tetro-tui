pub mod terminal_tetrs;
mod input_handler;
mod game_renderer;

fn main() -> Result<(), std::io::Error> {
    println!("{}", terminal_tetrs::TerminalTetrs::new(std::io::stdout()).run()?);
    Ok(())
}
