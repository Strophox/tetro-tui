mod core_game_engine;
mod fmt_helpers;
mod game_modding;
mod game_mode_blueprints;
mod game_renderers;
mod game_restoration;
mod savefile_logic;
mod settings;
mod terminal_buffers;
mod tui_menus;

use std::{io, path::PathBuf};

use std::{fmt::Debug, io::Write, time::Duration};

use crossterm::style::{PrintStyledContent, Stylize};
use crossterm::{
    ExecutableCommand,
    cursor::{self, MoveTo},
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    terminal::{self, Clear, ClearType},
};

use crate::core_game_engine::{
    ExtDuration, GameEndCause, InGameTime, Notification, NotificationFeed, Stat, Tetromino,
};

use crate::game_renderers::ShowStatsHud;
use crate::savefile_logic::SavefileResult;
use crate::{
    game_mode_blueprints::GameModeBlueprint,
    game_restoration::{
        EncodedInputHistory, GameRestorationData, InputHistoryEncoder, RawInputHistory,
    },
    savefile_logic::SavefileGranularity,
    settings::Settings,
    tui_menus::{Menu, MenuUpdate},
};

// Same as `clap::crate_version!()`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

const VERSION_MAJOR_MINOR: &str = {
    let full_semver_str_bytes = VERSION.as_bytes();
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use clap::Parser;
    #[derive(Parser, Debug)]
    #[command(version, about, long_about = None)]
    struct Args {
        /// Initial seed upon starting a custom game, given as a 64-bit integer.
        ///
        /// This influences e.g. the sequence of pieces used and\
        /// makes it possible to replay a run with the same pieces.
        ///
        /// Example usage:  ```tetro-tui --seed=42```
        ///              or ```tetro-tui -s 42```
        #[arg(short, long)]
        seed: Option<u64>,

        /// Initial board upon starting a custom game, encoded as character string.
        ///
        /// The string fills the board line-by-line:
        /// - Left->right; Bottom->top.
        /// - When end of the board width is reached, the next line is started.
        ///
        /// Every character corresponds to exactly *one filled board cell* except:
        /// - A space, underscore or dot (' ', '_', '.') indicates an *empty* cell.
        /// - A slash ('/') indicates an optional "skip to next line".
        /// - All newlines ('\n') are ignored completely.
        ///
        /// Example usage: |▄▄▀       |
        ///                becomes ```tetro-tui --board="###/   #"```
        ///                     or ```tetro-tui -b XYZ_/...W```
        #[arg(short, long)]
        board: Option<String>,
    }
    let args = Args::parse();

    // Initialize main application.
    let stdout = io::BufWriter::new(io::stdout());
    let mut app = Application::with_cmdlineflags(stdout, args.seed, args.board);

    // Set panic hook that catches actual panics and does a last effort to restore terminal state and print the error.
    set_custom_panic_hook();

    // Console prologue: Initialization.
    // FIXME: Handle io::Error? If not, why not?
    let _e = app.initialize_terminal_state();

    // Run main application.
    let app_result = app.run();

    // Console epilogue: Deinitialization.
    let term_deinit_result = app.deinitialize_terminal_state();
    if let Err(e) = term_deinit_result {
        eprintln!("An error occurred trying to set the terminal state back to its original: {e}");
    }

    // Print main `run` result.
    if let Err(e) = app_result {
        eprintln!("An error (likely I/O) closed the application: {e}");
    }

    // Try to write the savefile.
    // Note that this actively tries to delete the savefile if 'keep save file' is toggled off.
    let savefile_write_result = app.savefile_write();
    if let Err(e) = savefile_write_result {
        eprintln!("An error occurred trying to write the savefile: {e}");
    }

    Ok(())
}

/// Catch panics and write error to separate file if in debug mode, so it isn't truncated/lost due to terminal shenanigans.
fn set_custom_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        // #[cfg(debug_assertions)]
        // {
        //     use crate::fmt_helpers::generate_timestamp;

        //     let crash_file_name = format!(
        //         "tetro-tui_v{VERSION}_panic-info_{}.txt",
        //         generate_timestamp()
        //     );
        //     if let Ok(mut file) = std::fs::File::create(crash_file_name) {
        //         use std::io::Write;

        //         let _ = file.write(panic_info.to_string().as_bytes());
        //         let _ = file.write(b"\n\n\n");
        //         let _ = file.write(
        //             std::backtrace::Backtrace::force_capture()
        //                 .to_string()
        //                 .as_bytes(),
        //         );
        //     }
        // }
        // Forcefully reset terminal state.
        // Although `Application` restores it, it appears to sometimes not do so before we can meaningfully print
        // an error visible to the user.
        let _ = crossterm::terminal::disable_raw_mode();
        let _ =
            crossterm::ExecutableCommand::execute(&mut io::stderr(), crossterm::style::ResetColor);
        let _ = crossterm::ExecutableCommand::execute(&mut io::stderr(), Clear(ClearType::All));
        let _ = crossterm::ExecutableCommand::execute(&mut io::stderr(), crossterm::cursor::Show);
        let _ = crossterm::ExecutableCommand::execute(
            &mut io::stderr(),
            crossterm::terminal::LeaveAlternateScreen,
        );
        // Print the actual panic info.
        // let _ = crossterm::ExecutableCommand::execute(
        //     &mut io::stderr(),
        //     crossterm::cursor::MoveToColumn(0),
        // );
        let nice_str = "If you are using an unmodified version, help in improving the project by making the maintainers aware of above error (e.g. on github.com/Strophox/tetro-tui) is appreciated!";
        eprintln!(
            "{panic_info}\n----Detailed error trace:----\n{}\nIf you see this, something accidentally went wrong inside {}. {nice_str}\n\n",
            std::backtrace::Backtrace::force_capture(),
            Application::<io::Stderr>::TERMINAL_TITLE,
        );
    }))
}

/// Data associated with a Tetro TUI game.
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameMetaData {
    pub timestamp: String,
    pub title: String,
    pub show_stats: ShowStatsHud,
    pub objective_sort_descending: (Stat, bool),
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameSave<IH: InputHistoryEncoder> {
    game_meta_data: GameMetaData,
    game_restoration_data: GameRestorationData<IH>,
    inputs_to_load: usize,
}

impl GameSave<RawInputHistory> {
    fn compress<IH: InputHistoryEncoder>(self) -> GameSave<IH> {
        GameSave {
            game_restoration_data: self.game_restoration_data.encode(),
            game_meta_data: self.game_meta_data,
            inputs_to_load: self.inputs_to_load,
        }
    }
}

impl<IH: InputHistoryEncoder> GameSave<IH> {
    fn try_decode(self) -> Result<GameSave<RawInputHistory>, String> {
        Ok(GameSave {
            game_restoration_data: self.game_restoration_data.try_decode()?,
            game_meta_data: self.game_meta_data,
            inputs_to_load: self.inputs_to_load,
        })
    }
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct GameSaves<IH: InputHistoryEncoder> {
    selected: usize,
    slots: Vec<GameSave<IH>>,
}

impl<IH: InputHistoryEncoder> GameSaves<IH> {
    pub fn get(&self) -> Option<&GameSave<IH>> {
        self.slots.get(self.selected)
    }

    pub fn get_mut(&mut self) -> Option<&mut GameSave<IH>> {
        self.slots.get_mut(self.selected)
    }
}

/// An entry for the scoreboard. Store all the basic, cheap stats required for proper scoreboard entry display and sorting.
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct ScoreSummary {
    game_meta_data: GameMetaData,
    end_cause: GameEndCause,
    is_win: bool,
    time: InGameTime,
    lineclears: u32,
    points: u32,
    pieces: [u32; Tetromino::VARIANTS.len()],
    fall_delay_reached: ExtDuration,
    lock_delay_reached: ExtDuration,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum ScoreboardSorting {
    ModeDependent,
    Chronological,
    GameStat(Stat),
}

impl std::fmt::Display for ScoreboardSorting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ScoreboardSorting::ModeDependent => "Mode-dependent",
            ScoreboardSorting::Chronological => "Chronological",
            ScoreboardSorting::GameStat(stat) => match stat {
                Stat::TimeElapsed(_) => "Time elapsed",
                Stat::PiecesLocked(_) => "Pieces locked",
                Stat::LinesCleared(_) => "Lines cleared",
                Stat::PointsScored(_) => "Points scored",
            },
        };
        write!(f, "{name}")
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Scoreboard {
    sorting: ScoreboardSorting,
    entries: Vec<(
        ScoreSummary,
        Option<GameRestorationData<EncodedInputHistory>>,
    )>,
}

impl Default for Scoreboard {
    fn default() -> Self {
        Self {
            sorting: ScoreboardSorting::ModeDependent,
            entries: Vec::new(),
        }
    }
}

impl Scoreboard {
    fn sort(&mut self) {
        match self.sorting {
            ScoreboardSorting::Chronological => self.sort_chronologically(),
            ScoreboardSorting::ModeDependent => self.sort_semantically(),
            ScoreboardSorting::GameStat(stat) => self.sort_by_stat(stat),
        }
    }

    fn sort_chronologically(&mut self) {
        self.entries.sort_by(|(pg1, _), (pg2, _)| {
            pg1.game_meta_data
                .timestamp
                .cmp(&pg2.game_meta_data.timestamp)
                .reverse()
        });
    }

    #[rustfmt::skip]
    fn sort_semantically(&mut self) {
        self.entries.sort_by(|(pg1, _), (pg2, _)|
            // Sort by game mode (name).
            pg1.game_meta_data.title.cmp(&pg2.game_meta_data.title).then_with(||
            // Sort by if game mode was finished successfully.
            pg1.is_win.cmp(&pg2.is_win).reverse().then_with(|| {
                // Sort by comparison stat...
                let o = match pg1.game_meta_data.objective_sort_descending.0 {
                    Stat::TimeElapsed(_)    => pg1.time.cmp(&pg2.time),
                    Stat::PiecesLocked(_)   => pg1.pieces.cmp(&pg2.pieces),
                    Stat::LinesCleared(_)   => pg1.lineclears.cmp(&pg2.lineclears),
                    Stat::PointsScored(_)   => pg1.points.cmp(&pg2.points),
                };
                // Comparison stat is used positively/negatively (minimize or maximize) depending on
                // how comparison stat compares to 'most important'(??) (often sole) end condition.
                // This is shady, but the special order we subtly chose and never publicly document
                // makes this make sense...
                if pg1.game_meta_data.objective_sort_descending.1
                    { o } else { o.reverse() }
            })
            )
        );
    }

    fn sort_by_stat(&mut self, stat: Stat) {
        self.entries.sort_by(|(pg1, _), (pg2, _)| match stat {
            Stat::TimeElapsed(_) => pg1.time.cmp(&pg2.time),
            Stat::PiecesLocked(_) => pg1.pieces.cmp(&pg2.pieces),
            Stat::LinesCleared(_) => pg1.lineclears.cmp(&pg2.lineclears),
            Stat::PointsScored(_) => pg1.points.cmp(&pg2.points),
        });
    }
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Statistics {
    new_games_started: u32,
    games_ended: u32,
    play_time: Duration,
    pieces_locked: u32,
    points_scored: u32,
    lines_cleared: u32,
    monos: u32,
    duos: u32,
    tris: u32,
    tetras: u32,
    spins: u32,
    perfect_clears: u32,
    combo_clears: u32,
}

impl Statistics {
    // This simple blacklist is used to prevent certain game modes from being counted toward stats (e.g. Puzzle's perfect clears).
    const GAME_MODE_TITLE_PREFIX_BLACKLIST: &[&str] = &[
        GameModeBlueprint::TITLE_PUZZLE,
        GameModeBlueprint::TITLE_COMBO,
    ];

    fn accumulate_from_feed(&mut self, feed: &NotificationFeed) {
        for (notification, _notif_time) in feed {
            match notification {
                Notification::PieceLocked { .. } => {
                    self.pieces_locked += 1;
                }

                Notification::Accolade {
                    point_bonus,
                    lineclears,
                    combo,
                    is_spin,
                    is_perfect,
                    tetromino: _,
                } => {
                    self.points_scored += point_bonus;
                    self.lines_cleared += lineclears;
                    match lineclears {
                        1 => self.monos += 1,
                        2 => self.duos += 1,
                        3 => self.tris += 1,
                        4 => self.tetras += 1,
                        _ => {}
                    }
                    self.spins += if *is_spin { 1 } else { 0 };
                    self.perfect_clears += if *is_perfect { 1 } else { 0 };
                    self.combo_clears += if *combo > 1 { 1 } else { 0 };
                }

                _ => {}
            }
        }
    }

    fn accumulate(&mut self, other: &Statistics) {
        let Statistics {
            new_games_started: total_new_games_started,
            games_ended: total_games_ended,
            play_time: total_play_time,
            pieces_locked: total_pieces_locked,
            points_scored: total_points_scored,
            lines_cleared: total_lines_cleared,
            monos: total_mono,
            duos: total_duo,
            tris: total_tri,
            tetras: total_tetra,
            spins: total_spin,
            perfect_clears: total_perfect_clear,
            combo_clears: total_combo,
        } = self;

        *total_new_games_started += other.new_games_started;
        *total_games_ended += other.games_ended;
        *total_play_time += other.play_time;
        *total_pieces_locked += other.pieces_locked;
        *total_points_scored += other.points_scored;
        *total_lines_cleared += other.lines_cleared;
        *total_mono += other.monos;
        *total_duo += other.duos;
        *total_tri += other.tris;
        *total_tetra += other.tetras;
        *total_spin += other.spins;
        *total_perfect_clear += other.perfect_clears;
        *total_combo += other.combo_clears;
    }
}

#[derive(Debug)]
pub struct TemporaryAppData {
    pub custom_terminal_state_initialized: bool,
    pub kitty_detected: bool,
    pub kitty_assumed: bool,
    pub blindfold_game: bool,
    pub pause_on_focus_lost: bool,
    pub renderer_used: usize,
    pub save_on_exit: SavefileGranularity,
    pub savefile_path: PathBuf, // This should technically be the same for a given compiled binary, but we compute it at runtime.
    pub load_savefile_result: SavefileResult<()>,
    pub store_savefile_result: SavefileResult<()>,
}

#[derive(Debug)]
pub struct Application<W: Write> {
    term: W,

    temp_data: TemporaryAppData,

    settings: Settings,

    scores_and_replays: Scoreboard,

    statistics: Statistics,

    // FIXME: Currently one can only access one without resorting to manually editing the savefile.
    // The exact mechanism to ergonomically store and access several of these is subject to study.
    game_saves: GameSaves<RawInputHistory>,
}

impl<W: Write> Application<W> {
    pub const W_MAIN: u16 = 62;
    pub const H_MAIN: u16 = 24;

    pub const TERMINAL_TITLE: &str = "Tetro TUI";

    // FIXME: Undesirable results from pushing all() enhancement flags?
    pub const GAME_KEYBOARD_ENHANCEMENT_FLAGS: KeyboardEnhancementFlags =
        KeyboardEnhancementFlags::all();

    pub fn viewport_offset() -> (u16, u16) {
        let (w_console, h_console) = terminal::size().unwrap_or((0, 0));
        (
            w_console.saturating_sub(Self::W_MAIN) / 2,
            h_console.saturating_sub(Self::H_MAIN) / 2,
        )
    }

    fn initialize_terminal_state(&mut self) -> io::Result<()> {
        if !self.temp_data.custom_terminal_state_initialized {
            // Save the fact we started doing this.
            self.temp_data.custom_terminal_state_initialized = true;

            // 1. Enter alternate screen. This allows us not to trash the terminal's contents from before the app is run.
            self.term.execute(terminal::EnterAlternateScreen)?;

            // 2. Enable raw input mode (no enter required to read keyboard input).
            terminal::enable_raw_mode()?;

            // 3. Hide cursor.
            self.term.execute(cursor::Hide)?;

            // 4. Set title.
            self.term
                .execute(terminal::SetTitle(Self::TERMINAL_TITLE))?;

            // 5. For technical reasons we do not want default keyboard enhancement in the TUI's menus.
            // - Default enhancement trigger screen refreshes, discarding text selection and preventing Ctrl+Shift+C (copy, e.g. of savefile path in Advanced Settings menu).
            // - Enhancement-sensitive menus (e.g. game, replay, keybind settings) should set their own custom enhancement flags if applicable, so this should really only affect menus which rely on the "default" terminal enhancement state.
            // NOTE: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm minds if we do this on Windows.
            let _r = self.term.execute(PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::empty(),
            ));
        }
        Ok(())
    }

    fn deinitialize_terminal_state(&mut self) -> io::Result<()> {
        if self.temp_data.custom_terminal_state_initialized {
            // (Try to) undo terminal setup.

            // Bonus Step. Clean up terminal and reposition cursor.
            // This ensures we don't leave stuff in the middle of the terminal in case the terminal did not support switching between alternate screens and just reuses the same buffer.
            self.term.execute(Clear(ClearType::All))?;
            self.term.execute(MoveTo(0, 0))?;

            // 5.
            // NOTE: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm minds if we do this on Windows.
            let _r = self.term.execute(PopKeyboardEnhancementFlags);

            // FIXME: 4. Unset title?

            // 3.
            self.term.execute(cursor::Show)?;

            // 2.
            terminal::disable_raw_mode()?;

            // 1.
            self.term.execute(terminal::LeaveAlternateScreen)?;

            // Now save the fact we don't need to do this again.
            self.temp_data.custom_terminal_state_initialized = false;
        }

        Ok(())
    }

    pub fn with_cmdlineflags(
        term: W,
        custom_start_seed: Option<u64>,
        custom_start_board: Option<String>,
    ) -> Self {
        // Now that the settings are loaded, we handle separate flags set for this session.
        let kitty_detected = terminal::supports_keyboard_enhancement().unwrap_or(false);

        let temp_data = TemporaryAppData {
            custom_terminal_state_initialized: false,
            kitty_detected,
            kitty_assumed: kitty_detected,
            blindfold_game: false,
            pause_on_focus_lost: false,
            renderer_used: 0,
            save_on_exit: SavefileGranularity::default(),
            savefile_path: savefile_logic::savefile_path(),
            load_savefile_result: Ok(()),
            store_savefile_result: Ok(()),
        };

        let mut new = Self {
            term,
            temp_data,
            settings: Settings::default(),
            scores_and_replays: Scoreboard::default(),
            game_saves: GameSaves::default(),
            statistics: Statistics::default(),
        };

        // Load in actual settings.
        new.temp_data.load_savefile_result = new.savefile_read();

        if new.temp_data.load_savefile_result.is_err() {
            /*FIXME: Add replay demo?
            let demo = from_savefile_str(r#"..."#).unwrap();
            new.scores_and_replays.entries.push(demo);*/
        }

        // Special: Overwrite specifically requested cmdline flags.

        if custom_start_board.is_some() {
            new.settings.game_mode_settings.custom_config.start_board = custom_start_board;
        }

        if custom_start_seed.is_some() {
            new.settings.game_mode_settings.custom_config.seed = custom_start_seed;
        }

        new
    }

    pub fn run(&mut self) -> io::Result<()> {
        let mut menu_stack = vec![Menu::Title];
        // Retrieve active menu, and stop application if stack is empty.
        while let Some(menu) = menu_stack.last_mut() {
            // Open new menu screen, then store what it returns.
            let menu_update = match menu {
                Menu::Title => self.run_menu_title(),
                Menu::KeybindsOverview {
                    client_menu_name: is_help_for_menu_with_name,
                    legend: keybinds,
                } => self.run_menu_keybinds_help(is_help_for_menu_with_name, keybinds),
                Menu::NewGame => self.run_menu_new_game(),
                Menu::PlayGame {
                    game,
                    raw_input_history,
                    game_meta_data,
                    game_renderer,
                    selection_id_for_game_retry,
                } => self.run_menu_play_game(
                    game,
                    raw_input_history,
                    game_meta_data,
                    game_renderer,
                    *selection_id_for_game_retry,
                ),
                Menu::Pause => self.run_menu_pause(),
                Menu::Settings => self.run_menu_settings(),
                Menu::AdjustGraphics => self.run_menu_adjust_graphics(),
                Menu::AdjustKeybinds => self.run_menu_adjust_keybinds(),
                Menu::AdjustGameplay => self.run_menu_adjust_gameplay(),
                Menu::AdvancedSettings => self.run_menu_advanced_settings(),
                Menu::GameOver { game_scoring } => self.run_menu_game_ended(game_scoring),
                Menu::GameComplete { game_scoring } => self.run_menu_game_ended(game_scoring),
                Menu::ScoresAndReplays {
                    cursor_pos,
                    camera_pos,
                } => self.run_menu_scores_and_replays(cursor_pos, camera_pos),
                Menu::ReplayGame {
                    game_restoration_data,
                    game_meta_data,
                    replay_length,
                    game_renderer,
                    cached_game_and_replay_anchors,
                } => self.run_menu_replay_game(
                    game_restoration_data,
                    game_meta_data,
                    *replay_length,
                    game_renderer.as_mut(),
                    cached_game_and_replay_anchors,
                ),
                Menu::Statistics => self.run_menu_statistics(),
                Menu::About => self.run_menu_about(),
                Menu::Quit => break,
            }?;

            // Change screen session depending on what response screen gave.
            match menu_update {
                MenuUpdate::Pop => {
                    if menu_stack.len() > 1 {
                        menu_stack.pop();
                    }
                }
                MenuUpdate::Push(menu) => {
                    if matches!(menu, Menu::Quit) {
                        break;
                    }

                    if matches!(
                        menu,
                        Menu::Title
                            | Menu::PlayGame { .. }
                            | Menu::GameOver { .. }
                            | Menu::GameComplete { .. }
                    ) {
                        menu_stack.clear();
                    }

                    if matches!(menu, Menu::GameOver { .. }) {
                        let (w, h) = terminal::size()?;
                        for y in (0..h).rev() {
                            self.term
                                .execute(MoveTo(0, y))?
                                .execute(PrintStyledContent(
                                    " ".repeat(w as usize)
                                        .on(self.settings.tui_coloring().bg_tui),
                                ))?;
                            std::thread::sleep(Duration::from_secs_f32(1. / 120.0));
                        }
                    } else if matches!(menu, Menu::GameComplete { .. }) {
                        let (w, h) = terminal::size()?;
                        for y in 0..h {
                            self.term
                                .execute(MoveTo(0, y))?
                                .execute(PrintStyledContent(
                                    " ".repeat(w as usize)
                                        .on(self.settings.tui_coloring().bg_tui),
                                ))?;
                            std::thread::sleep(Duration::from_secs_f32(1. / 120.0));
                        }
                    }

                    menu_stack.push(menu);
                }
            }
        }

        Ok(())
    }
}
