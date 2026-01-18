mod game_input_handlers;
mod game_modifiers;
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
    /// Example: `./tetrs_tui --seed=42` or `./tetrs_tui -s 42`.
    #[arg(short, long)]
    seed: Option<u64>,
    /// Custom starting board when playing Custom mode (10-wide rows), encoded as string.
    /// Spaces indicate empty cells, anything else is a filled cell.
    /// The string just represents the row information, starting with the topmost row.
    /// Example: '█▀ ▄██▀ ▀█'
    ///          => `./tetrs_tui --board="XX  XXX XXO  OOO   O"` or `./tetrs_tui -b "XX  XXX XXO  OOO   O"`.
    #[arg(short, long)]
    board: Option<String>,
    /// Whether to enable the combo bot in Combo mode: `./tetrs_tui --enable-combo-bot` or `./tetrs_tui -e`
    #[arg(short, long)]
    enable_combo_bot: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let stdout = io::BufWriter::new(io::stdout());
    let mut app = terminal_user_interface::Application::new(
        stdout,
        args.seed,
        args.board,
        args.enable_combo_bot,
    );
    std::panic::set_hook(Box::new(|panic_info| {
        let crash_file_name = format!(
            "tetrs-tui_crash-msg_{}.txt",
            chrono::Utc::now().format("%Y-%m-%d_%Hh%Mm%Ss")
        );
        if let Ok(mut file) = std::fs::File::create(crash_file_name) {
            let _ = file.write(panic_info.to_string().as_bytes());
            // FIXME: remove: let _ = file.write(std::backtrace::Backtrace::force_capture().to_string().as_bytes());
        }
    }));
    let exit_msg = app.run()?;
    println!("{exit_msg}");
    Ok(())
}
