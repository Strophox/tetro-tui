mod application;
mod fmt_helpers;
mod game_mode_presets;
mod game_renderers;
mod gameplay_settings;
mod graphics_settings;
mod keybinds;
mod live_input_handler;
mod palette;

use std::{io, path::PathBuf};

use clap::Parser;

// Inspired by `clap::crate_version!()`.
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

const CRATE_VERSION_MAJOR_MINOR: &str = {
    let full_semver_str_bytes = CRATE_VERSION.as_bytes();
    let mut dot_seen = false;
    let mut i = 0;
    loop {
        if full_semver_str_bytes[i] == b'.' {
            if dot_seen {
                break;
            } else {
                dot_seen = true;
            }
        }
        i += 1;
    }

    let Ok(the_str) = str::from_utf8(full_semver_str_bytes.split_at(i).0) else {
        unreachable!()
    };

    the_str
};

fn savefile_name() -> String {
    format!(".tetro-tui_v{CRATE_VERSION_MAJOR_MINOR}_savefile.json")
}

fn savefile_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(savefile_name())
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Custom starting seed when playing a custom game, given as a 64-bit integer.
    /// This influences e.g. the sequence of pieces used and makes it possible to replay
    /// a run with the same pieces if the same seed is entered.
    /// Example: `tetro-tui --seed=42` or `tetro-tui -s 42`.
    #[arg(short, long)]
    seed: Option<u64>,
    /// Custom starting board when playing a custom game (10-wide rows), encoded as string.
    /// Spaces indicate empty cells, any other character is a filled cell.
    /// The string just represents the row information, starting with the topmost row.
    /// Example: |█▀ ▄██▀ ▀█| => `tetro-tui --board="O  OOO   OXX  XXX XX"` or `tetro-tui -b "O  OOO   OXX  XXX XX"`.
    #[arg(short, long)]
    board: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read commandline arguments.
    let args = Args::parse();

    // Initialize application.
    let stdout = io::BufWriter::new(io::stdout());
    let mut app = application::Application::with_savefile_and_cmdlineoptions(
        stdout,
        savefile_path(),
        args.seed,
        args.board,
    );

    // Catch panics and write error to separate file, so it isn't lost due to app's terminal shenanigans.
    std::panic::set_hook(Box::new(|panic_info| {
        #[cfg(debug_assertions)]
        {
            let crash_file_name = format!(
                "tetro-tui_v{CRATE_VERSION}_panic-info_{}.txt",
                chrono::Utc::now().format("%Y-%m-%d_%Hh%Mm%Ss")
            );
            if let Ok(mut file) = std::fs::File::create(crash_file_name) {
                use std::io::Write;

                let _ = file.write(panic_info.to_string().as_bytes());
                let _ = file.write(b"\n\n\n");
                let _ = file.write(
                    std::backtrace::Backtrace::force_capture()
                        .to_string()
                        .as_bytes(),
                );
            }
        }
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
