mod game_input_handler;
mod game_mods;
mod game_renderers;
mod puzzle_mode;
pub mod terminal_tetrs;

use std::io::{self, Write};

use clap::Parser;

/// Terminal frontend for playing tetrs.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The framerate at which to run the main game.
    #[arg(short, long)]
    fps: Option<u32>,
}

fn main() -> Result<(), io::Error> {
    let args = Args::parse();
    let stdout = io::BufWriter::new(io::stdout());
    let mut app = terminal_tetrs::App::new(stdout, args.fps);
    std::panic::set_hook(Box::new(|panic_info| {
        if let Ok(mut file) = std::fs::File::create("tetrs_terminal_error_message.txt") {
            let _ = file.write(panic_info.to_string().as_bytes());
            // let _ = file.write(std::backtrace::Backtrace::force_capture().to_string().as_bytes());
        }
    }));
    let msg = app.run()?;
    println!("{msg}");
    Ok(())
}
