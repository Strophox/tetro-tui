mod application;
mod fmt_helpers;
mod game_mode_presets;
mod game_renderers;
mod keybinds_presets;
mod live_input_handler;
mod palette_presets;

use std::io;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Custom starting seed when playing a custom game, given as a 64-bit integer.
    /// This influences e.g. the sequence of pieces used and makes it possible to replay
    /// a run with the same pieces if the same seed is entered.
    /// Example: `./tetrs_tui --seed=42` or `./tetrs_tui -s 42`.
    #[arg(short, long)]
    seed: Option<u64>,
    /// Custom starting board when playing a custom game (10-wide rows), encoded as string.
    /// Spaces indicate empty cells, any other character is a filled cell.
    /// The string just represents the row information, starting with the topmost row.
    /// Example: |█▀ ▄██▀ ▀█| => `./tetrs_tui --board="O  OOO   OXX  XXX XX"` or `./tetrs_tui -b "O  OOO   OXX  XXX XX"`.
    #[arg(short, long)]
    board: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read commandline arguments.
    let args = Args::parse();

    // Initialize application.
    let stdout = io::BufWriter::new(io::stdout());
    let mut app = application::Application::new(stdout, args.seed, args.board);

    // Catch panics and write error to separate file, so it isn't lost due to app's terminal shenanigans.
    std::panic::set_hook(Box::new(|panic_info| {
        // Forcefully reset terminal state.
        // Although `Application` restores it, it appears to sometimes not do so before we can meaningfully print
        // an error visible to the user.
        let _ = crossterm::terminal::disable_raw_mode();
        let _ =
            crossterm::ExecutableCommand::execute(&mut io::stderr(), crossterm::style::ResetColor);
        let _ = crossterm::ExecutableCommand::execute(&mut io::stderr(), crossterm::cursor::Show);
        let _ = crossterm::ExecutableCommand::execute(
            &mut io::stderr(),
            crossterm::terminal::LeaveAlternateScreen,
        );

        // Print the actual panic info.
        eprint!("{panic_info}\n\n");
    }));

    // Run main application.
    app.run()?;

    Ok(())
}
