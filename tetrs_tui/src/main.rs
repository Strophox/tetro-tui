mod game_input_handlers;
mod game_mods;
mod game_renderers;
mod terminal_user_interface;

use std::io::{self, Write};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Custom starting seed when playing Custom mode, given as a 64-bit integer.
    /// This influences the sequence of pieces used and makes it possible to replay
    /// a run with the same pieces if the same seed is entered.
    /// Example: `./tetrs_tui --custom-seed=42` or `./tetrs_tui -c 42`.
    #[arg(short, long)]
    custom_seed: Option<u64>,
    /// Custom starting board when playing Custom mode (10-wide rows), encoded as string.
    /// Spaces indicate empty cells, anything else is a filled cell.
    /// The string just represents the row information, starting with the topmost row.
    /// Example: '█▀ ▄██▀ ▀█'
    ///          => `./tetrs_tui --custom-start="XX  XXX XXO  OOO   O"`.
    #[arg(long)]
    custom_start: Option<String>,
    /// Custom starting layout when playing Combo mode (4-wide rows), encoded as binary.
    /// Example: '▀▄▄▀' => 0b_1001_0110 = 150
    ///          => `./tetrs_tui --combo-start=150`.
    #[arg(long)]
    combo_start: Option<u16>,
    /// Whether to enable the combo bot in Combo mode: `./tetrs_tui --enable-combo-bot` or `./tetrs_tui -e`
    #[arg(short, long)]
    enable_combo_bot: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let stdout = io::BufWriter::new(io::stdout());
    let mut app = terminal_user_interface::Application::new(
        stdout,
        args.custom_seed,
        args.custom_start,
        args.combo_start,
        args.enable_combo_bot,
    );
    std::panic::set_hook(Box::new(|panic_info| {
        if let Ok(mut file) = std::fs::File::create("tetrs_tui_crash_message.txt") {
            let _ = file.write(panic_info.to_string().as_bytes());
            // let _ = file.write(std::backtrace::Backtrace::force_capture().to_string().as_bytes());
        }
    }));
    let msg = app.run()?;
    println!("{msg}");
    Ok(())
}
