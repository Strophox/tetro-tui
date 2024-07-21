mod game_input_handler;
mod game_screen_renderers;
pub mod terminal_tetrs;

use std::io;

use clap::Parser;

/// Terminal frontend for playing tetrs.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The framerate at which to run the main game.
    #[arg(short, long, default_value_t = 30)]
    fps: u32,
}

fn main() -> Result<(), io::Error> {
    let args = Args::parse();
    let stdout = io::BufWriter::new(io::stdout());
    let mut app = terminal_tetrs::TerminalTetrs::new(stdout, args.fps);
    let msg = app.run()?;
    println!("{msg}");
    Ok(())
}
