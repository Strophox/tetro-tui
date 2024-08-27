mod game_input_handler;
mod game_mods;
mod game_renderers;
pub mod terminal_app;

use std::io::{self, Write};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Whether to show Descent mode in the New Game menu.
    #[arg(short, long)]
    descent_mode: bool,
    /// A custom starting layout for Combo mode, encoded in binary, by 4-wide rows.
    /// Example: "▀▄▄▀" -> "10010110"
    #[arg(short, long)]
    combo_layout: Option<u16>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let stdout = io::BufWriter::new(io::stdout());
    let mut app = terminal_app::TerminalApp::new(stdout, args.descent_mode, args.combo_layout);
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
