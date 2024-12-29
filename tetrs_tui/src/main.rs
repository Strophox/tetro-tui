mod game_input_handlers;
mod game_mods;
mod game_renderers;
mod terminal_app;

use std::io::{self, Write};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
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
    let mut app = terminal_app::TerminalApp::new(
        stdout,
        args.combo_start,
        args.custom_start,
        args.enable_combo_bot,
    );
    std::panic::set_hook(Box::new(|panic_info| {
        if let Ok(mut file) = std::fs::File::create("tetrs_tui_error_message.txt") {
            let _ = file.write(panic_info.to_string().as_bytes());
            // let _ = file.write(std::backtrace::Backtrace::force_capture().to_string().as_bytes());
        }
    }));
    let msg = app.run()?;
    println!("{msg}");
    Ok(())
}
