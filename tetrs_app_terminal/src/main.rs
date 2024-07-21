mod game_screen_renderers;
mod input_handler;
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
    let msg = terminal_tetrs::TerminalTetrs::new(stdout, args.fps).run()?;
    println!("{msg}");
    Ok(())
}
