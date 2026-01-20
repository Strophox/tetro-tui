mod menus;

use std::{
    fmt::Debug,
    fs::File,
    io::{self, Read, Write},
    num::NonZeroUsize,
    path::PathBuf,
    time::{Duration, Instant},
};

use crossterm::{cursor, style, terminal, ExecutableCommand};

use tetrs_engine::{
    Config, Feedback, FeedbackVerbosity, Game, GameBuilder, GameOver, GameResult, GameTime,
    Modifier, Rules, Stat, Tetromino,
};

use crate::{
    game_input_handlers::terminal::{
        guideline_keybinds, tetrs_default_keybinds, vim_keybinds, Keybinds,
    },
    game_modifiers,
    game_renderers::{
        self, color16_palette, empty_palette, fullcolor_palette, gruvbox_light_palette,
        gruvbox_palette, oklch2_palette, Palette,
    },
    utils::decode_buttons,
};

pub type Slots<T> = Vec<(String, T)>;

pub type RecordedUserInput = Vec<(
    u64, // For serialization reasons, we use an encoded version of `GameTime` (see `std::time::Duration::as_nanos`).
    u16, // For serialization reasons, we use an encoded version of `tetrs_engine::PressedButtons` (see `crate::utils::encode_buttons`).
)>;

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GameRestorationData {
    builder: GameBuilder,
    mod_descriptors: Vec<String>,
    recorded_user_input: RecordedUserInput,
}

impl GameRestorationData {
    fn new(game: &Game, recorded_user_input: &RecordedUserInput) -> GameRestorationData {
        let (builder, mod_descriptors) = game.blueprint();
        GameRestorationData {
            builder,
            mod_descriptors: mod_descriptors.map(str::to_owned).collect(),
            recorded_user_input: recorded_user_input.clone(),
        }
    }

    fn restore(&self, input_index: usize) -> Game {
        // Step 1: Prepare builder.
        let builder = self.builder.clone();
        // Step 2: Build actual game by possibly reconstructing mods to finalize builder with.
        let mut game = if self.mod_descriptors.is_empty() {
            builder.build()
        } else {
            match game_modifiers::reconstruct_modified(
                &builder,
                self.mod_descriptors.iter().map(String::as_str),
            ) {
                Ok(modified_game) => modified_game,
                Err(msg) => {
                    #[rustfmt::skip]
                    let print_error_msg_mod = Modifier {
                        descriptor: "print_error_msg_mod".to_owned(),
                        mod_function: Box::new({ let mut init = false;
                            move |_config, _rules, state, _modpoint, msgs| {
                                if init { return; } init = true;
                                msgs.push((state.time, Feedback::Text(format!("ERROR: {msg:?}"))));
                            }
                        }),
                    };
                    builder.build_modified([print_error_msg_mod])
                }
            }
        };

        // Step 3: Reenact recorded game inputs.
        let restore_feedback_verbosity = game.config().feedback_verbosity;

        game.config_mut().feedback_verbosity = FeedbackVerbosity::Quiet;
        for (input_time, int) in self.recorded_user_input.iter().take(input_index) {
            let button_state = decode_buttons(*int);
            // FIXME: Error handling?
            let _ = game.update(Some(button_state), GameTime::from_nanos(*input_time));
        }

        game.config_mut().feedback_verbosity = restore_feedback_verbosity;

        game
    }
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameMetaData {
    pub datetime: String,
    pub title: String,
    pub comparison_stat: (Stat, bool),
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct ScoreboardEntry {
    meta_data: GameMetaData,
    result: GameResult,
    time_elapsed: GameTime,
    pieces_locked: [u32; Tetromino::VARIANTS.len()],
    lines_cleared: usize,
    gravity_reached: u32,
    points_scored: u64,
}

impl ScoreboardEntry {
    fn new(game: &Game, meta_data: &GameMetaData) -> ScoreboardEntry {
        ScoreboardEntry {
            meta_data: meta_data.clone(),
            time_elapsed: game.state().time,
            pieces_locked: game.state().pieces_locked,
            lines_cleared: game.state().lines_cleared,
            gravity_reached: game.state().gravity,
            points_scored: game.state().score,
            result: game.state().result.unwrap_or(Err(GameOver::Forfeit)),
        }
    }
}

#[derive(
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ScoreboardSorting {
    #[default]
    Chronological,
    Semantic,
}

#[derive(PartialEq, PartialOrd, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Scoreboard {
    sorting: ScoreboardSorting,
    entries: Vec<(ScoreboardEntry, Option<GameRestorationData>)>,
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct NewGameSettings {
    custom_rules: Rules,
    custom_seed: Option<u64>,
    custom_board: Option<String>, // For more compact serialization of NewGameSettings, we store an encoded `Board` (see `encode_board`).
    /// Custom starting layout when playing Combo mode (4-wide rows), encoded as binary.
    /// Example: '▀▄▄▀' => 0b_1001_0110 = 150
    combo_startlayout: u16,
    combo_linelimit: Option<NonZeroUsize>,
    cheese_gapsize: usize,
    cheese_gravity: u32,
    cheese_linelimit: Option<NonZeroUsize>,
    experimental_mode_unlocked: bool,
}

impl Default for NewGameSettings {
    fn default() -> Self {
        Self {
            custom_rules: Rules::default(),
            custom_seed: None,
            custom_board: None,
            cheese_linelimit: Some(NonZeroUsize::try_from(50).unwrap()),
            cheese_gravity: 0,
            cheese_gapsize: 1,
            combo_linelimit: Some(NonZeroUsize::try_from(25).unwrap()),
            combo_startlayout: game_modifiers::combo_board::LAYOUTS[0],
            experimental_mode_unlocked: false,
        }
    }
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum Glyphset {
    Electronika60,
    #[allow(clippy::upper_case_acronyms)]
    ASCII,
    Unicode,
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphicsSettings {
    pub glyphset: Glyphset,
    palette_active: usize,
    palette_active_lockedtiles: usize,
    pub render_effects: bool,
    pub show_ghost_piece: bool,
    game_fps: f64,
    show_fps: bool,
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum SavefileGranularity {
    NoSavefile,
    RememberSettings,
    RememberSettingsScoreboard,
    RememberSettingsScoreboardGamereplays,
}

#[serde_with::serde_as]
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    graphics_slot_active: usize,
    keybinds_slot_active: usize,
    config_slot_active: usize,
    graphics_slots_that_should_not_be_changed: usize,
    palette_slots_that_should_not_be_changed: usize,
    keybinds_slots_that_should_not_be_changed: usize,
    config_slots_that_should_not_be_changed: usize,
    graphics_slots: Slots<GraphicsSettings>,
    palette_slots: Slots<Palette>,
    config_slots: Slots<Config>,
    // NOTE: Reconsider #[serde_as(as = "Vec<(_, std::collections::HashMap<serde_with::json::JsonString, _>)>")]
    #[serde_as(as = "Vec<(_, Vec<(_, _)>)>")]
    keybinds_slots: Slots<Keybinds>,
}

impl Default for Settings {
    fn default() -> Self {
        let graphics_slots = vec![
            (
                "default".to_owned(),
                GraphicsSettings {
                    glyphset: Glyphset::Unicode,
                    palette_active: 3,
                    palette_active_lockedtiles: 3,
                    render_effects: true,
                    show_ghost_piece: true,
                    game_fps: 30.0,
                    show_fps: false,
                },
            ),
            (
                "high focus".to_owned(),
                GraphicsSettings {
                    glyphset: Glyphset::Unicode,
                    palette_active: 2,
                    palette_active_lockedtiles: 0,
                    render_effects: false,
                    show_ghost_piece: true,
                    game_fps: 60.0,
                    show_fps: false,
                },
            ),
        ];
        let palette_slots = vec![
            ("Monochrome".to_owned(), empty_palette()), // NOTE: The slot at index 0 is the special 'monochrome'/no palette slot.
            ("16-color".to_owned(), color16_palette()),
            ("Fullcolor".to_owned(), fullcolor_palette()),
            ("Okpalette".to_owned(), oklch2_palette()),
            ("Gruvbox".to_owned(), gruvbox_palette()),
            ("Gruvbox (light)".to_owned(), gruvbox_light_palette()),
        ];
        let keybinds_slots = vec![
            ("tetrs default".to_owned(), tetrs_default_keybinds()),
            ("Vim-like".to_owned(), vim_keybinds()),
            ("TTC default".to_owned(), guideline_keybinds()),
        ];
        let config_slots = vec![
            ("default".to_owned(), Config::default()),
            (
                "high finesse".to_owned(),
                Config {
                    preview_count: 9,
                    delayed_auto_shift: Duration::from_millis(110),
                    auto_repeat_rate: Duration::from_millis(0),
                    ..Config::default()
                },
            ),
        ];
        Self {
            graphics_slot_active: 0,
            keybinds_slot_active: 0,
            config_slot_active: 0,
            graphics_slots_that_should_not_be_changed: graphics_slots.len(),
            palette_slots_that_should_not_be_changed: palette_slots.len(),
            keybinds_slots_that_should_not_be_changed: keybinds_slots.len(),
            config_slots_that_should_not_be_changed: config_slots.len(),
            graphics_slots,
            palette_slots,
            keybinds_slots,
            config_slots,
        }
    }
}

impl Settings {
    pub fn graphics(&self) -> &GraphicsSettings {
        &self.graphics_slots[self.graphics_slot_active].1
    }
    pub fn keybinds(&self) -> &Keybinds {
        &self.keybinds_slots[self.keybinds_slot_active].1
    }
    pub fn config(&self) -> &Config {
        &self.config_slots[self.config_slot_active].1
    }
    fn graphics_mut(&mut self) -> &mut GraphicsSettings {
        &mut self.graphics_slots[self.graphics_slot_active].1
    }
    fn keybinds_mut(&mut self) -> &mut Keybinds {
        &mut self.keybinds_slots[self.keybinds_slot_active].1
    }
    fn config_mut(&mut self) -> &mut Config {
        &mut self.config_slots[self.config_slot_active].1
    }

    pub fn palette(&self) -> &Palette {
        &self.palette_slots[self.graphics().palette_active].1
    }
    pub fn palette_lockedtiles(&self) -> &Palette {
        &self.palette_slots[self.graphics().palette_active_lockedtiles].1
    }
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct RuntimeData {
    kitty_detected: bool,
    kitty_assumed: bool,
    combo_bot_enabled: bool,
}

#[derive(Debug)]
enum Menu {
    Title,
    NewGame,
    Game {
        game: Box<Game>,
        meta_data: GameMetaData,
        time_started: Instant,
        last_paused: Instant,
        total_pause_duration: Duration,
        recorded_user_input: RecordedUserInput,
        game_renderer: Box<game_renderers::diff::DiffRenderer>,
    },
    GameOver(Box<ScoreboardEntry>),
    GameComplete(Box<ScoreboardEntry>),
    Pause,
    Settings,
    AdjustKeybinds,
    AdjustGameplay,
    AdjustGraphics,
    Scores,
    About,
    Quit(String),
}

impl std::fmt::Display for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Menu::Title => "Title Screen",
            Menu::NewGame => "New Game",
            Menu::Game { meta_data, .. } => &format!("Game ({})", meta_data.title),
            Menu::GameOver(_) => "Game Over",
            Menu::GameComplete(_) => "Game Completed",
            Menu::Pause => "Pause",
            Menu::Settings => "Settings",
            Menu::AdjustKeybinds => "Adjust Keybinds",
            Menu::AdjustGameplay => "Adjust Gameplay",
            Menu::AdjustGraphics => "Adjust Graphics",
            Menu::Scores => "Scoreboard",
            Menu::About => "About",
            Menu::Quit(_) => "Quit",
        };
        write!(f, "{name}")
    }
}

#[derive(Debug)]
enum MenuUpdate {
    Pop,
    Push(Menu),
}

#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Application<T: Write> {
    runtime_data: RuntimeData,
    pub term: T,
    save_on_exit: SavefileGranularity,
    settings: Settings,
    new_game_settings: NewGameSettings,
    savepoint: Option<(GameMetaData, usize, GameRestorationData)>,
    scoreboard: Scoreboard,
}

impl<T: Write> Drop for Application<T> {
    fn drop(&mut self) {
        let savefile_path = Self::savefile_path();
        // If the user wants any of their data stored, try to do so.
        if self.save_on_exit != SavefileGranularity::NoSavefile {
            // FIXME: Handle error?
            if let Err(_e) = self.store_savefile(savefile_path) {
                //eprintln!("Could not save settings this time: {e} ");
                //std::thread::sleep(Duration::from_secs(4));
            }
        // Otherwise explicitly check for savefile to make sure it's removed.
        } else if savefile_path.try_exists().is_ok_and(|exists| exists) {
            let _ = std::fs::remove_file(savefile_path);
        }
        // FIXME: Handle error?
        let _ = terminal::disable_raw_mode();
        let _ = self.term.execute(style::ResetColor);
        let _ = self.term.execute(cursor::Show);
        let _ = self.term.execute(terminal::LeaveAlternateScreen);
    }
}

impl<T: Write> Application<T> {
    pub const W_MAIN: u16 = 80;
    pub const H_MAIN: u16 = 24;

    pub const SAVEFILE_NAME: &'static str = ".tetrs_tui_savefile.json";

    pub fn new(
        mut term: T,
        custom_start_seed: Option<u64>,
        custom_start_board: Option<String>,
        combo_bot_enabled: bool,
    ) -> Self {
        // Console prologue: Initialization.
        // FIXME: Handle error?
        let _ = term.execute(terminal::EnterAlternateScreen);
        let _ = term.execute(terminal::SetTitle("tetrs - Terminal User Interface"));
        let _ = term.execute(cursor::Hide);
        let _ = terminal::enable_raw_mode();
        let mut app = Self {
            runtime_data: RuntimeData {
                kitty_detected: false,
                kitty_assumed: false,
                combo_bot_enabled: false,
            },
            term,
            settings: Settings::default(),
            scoreboard: Scoreboard::default(),
            new_game_settings: NewGameSettings::default(),
            savepoint: None,
            save_on_exit: SavefileGranularity::NoSavefile,
        };

        // Actually load in settings.
        // FIXME: Handle error?
        if app.load_savefile(Self::savefile_path()).is_err() {
            //eprintln!("Could not loading settings: {e}");
            //std::thread::sleep(Duration::from_secs(5));
        }

        // Now that the settings are loaded, we handle separate flags set for this session.
        let kitty_detected = terminal::supports_keyboard_enhancement().unwrap_or(false);
        app.runtime_data = RuntimeData {
            combo_bot_enabled,
            kitty_detected,
            kitty_assumed: kitty_detected,
        };
        if custom_start_board.is_some() {
            app.new_game_settings.custom_board = custom_start_board;
        }
        if custom_start_seed.is_some() {
            app.new_game_settings.custom_seed = custom_start_seed;
        }
        app
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub(crate) fn fetch_main_xy() -> (u16, u16) {
        let (w_console, h_console) = terminal::size().unwrap_or((0, 0));
        (
            w_console.saturating_sub(Self::W_MAIN) / 2,
            h_console.saturating_sub(Self::H_MAIN) / 2,
        )
    }

    fn savefile_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(Self::SAVEFILE_NAME)
    }

    fn store_savefile(&mut self, path: PathBuf) -> io::Result<()> {
        if self.save_on_exit < SavefileGranularity::RememberSettingsScoreboard {
            // Clear scoreboard if no game data is wished to be stored.
            self.scoreboard.entries.clear();
        } else if self.save_on_exit < SavefileGranularity::RememberSettingsScoreboardGamereplays {
            // Clear past game inputs if no game input data is wished to be stored.
            for (_entry, restoration_data) in &mut self.scoreboard.entries {
                restoration_data.take();
            }
        }

        let save_state = (
            &self.save_on_exit,
            &self.settings,
            &self.new_game_settings,
            &self.scoreboard,
            &self.savepoint,
        );
        let save_str = serde_json::to_string(&save_state)?;
        let mut file = File::create(path)?;
        // FIXME: Handle error?
        let _ = file.write(save_str.as_bytes())?;
        Ok(())
    }

    fn load_savefile(&mut self, path: PathBuf) -> io::Result<()> {
        let mut file = File::open(path)?;
        let mut save_str = String::new();
        file.read_to_string(&mut save_str)?;
        let save_state = serde_json::from_str(&save_str)?;
        (
            self.save_on_exit,
            self.settings,
            self.new_game_settings,
            self.scoreboard,
            self.savepoint,
        ) = save_state;
        Ok(())
    }

    fn sort_past_games_chronologically(&mut self) {
        self.scoreboard.entries.sort_by(|(pg1, _), (pg2, _)| {
            pg1.meta_data
                .datetime
                .cmp(&pg2.meta_data.datetime)
                .reverse()
        });
    }

    #[rustfmt::skip]
    fn sort_past_games_semantically(&mut self) {
        self.scoreboard.entries.sort_by(|(pg1, _), (pg2, _)|
            // Sort by gamemode (name).
            pg1.meta_data.title.cmp(&pg2.meta_data.title).then_with(||
            // Sort by if gamemode was finished successfully.
            pg1.result.is_ok().cmp(&pg2.result.is_ok()).then_with(|| {
                // Sort by comparison stat...
                let o = match pg1.meta_data.comparison_stat.0 {
                    Stat::TimeElapsed(_)    => pg1.time_elapsed.cmp(&pg2.time_elapsed),
                    Stat::PiecesLocked(_)   => pg1.pieces_locked.cmp(&pg2.pieces_locked),
                    Stat::LinesCleared(_)   => pg1.lines_cleared.cmp(&pg2.lines_cleared),
                    Stat::GravityReached(_) => pg1.gravity_reached.cmp(&pg2.gravity_reached),
                    Stat::PointsScored(_)   => pg1.points_scored.cmp(&pg2.points_scored),
                };
                // Comparison stat is used positively/negatively (minimize or maximize) depending on
                // how comparison stat compares to 'most important'(??) (often sole) end condition.
                // This is shady, but the special order we subtly chose and never publicly document
                // makes this make sense...
                if pg1.meta_data.comparison_stat.1
                    { o } else { o.reverse() }
            })
            )
            .reverse()
        );
    }

    pub fn run(&mut self) -> io::Result<String> {
        let mut menu_stack = vec![Menu::Title];
        let msg = loop {
            // Retrieve active menu, stop application if stack is empty.
            let Some(screen) = menu_stack.last_mut() else {
                break String::from("all menus exited");
            };
            // Open new menu screen, then store what it returns.
            let menu_update = match screen {
                Menu::Title => self.menu_title(),
                Menu::NewGame => self.menu_new_game(),
                Menu::Game {
                    game,
                    meta_data,
                    time_started,
                    total_pause_duration,
                    last_paused,
                    recorded_user_input,
                    game_renderer,
                } => self.menu_game(
                    game,
                    meta_data,
                    time_started,
                    last_paused,
                    total_pause_duration,
                    recorded_user_input,
                    game_renderer.as_mut(),
                ),
                Menu::Pause => self.menu_pause(),
                Menu::GameOver(past_game) => self.menu_game_ended(past_game),
                Menu::GameComplete(past_game) => self.menu_game_ended(past_game),
                Menu::Scores => self.menu_scoreboard(),
                Menu::About => self.menu_about(),
                Menu::Settings => self.menu_settings(),
                Menu::AdjustKeybinds => self.menu_adjust_keybinds(),
                Menu::AdjustGameplay => self.menu_adjust_gameplay(),
                Menu::AdjustGraphics => self.menu_adjust_graphics(),
                Menu::Quit(string) => break string.clone(),
            }?;
            // Change screen session depending on what response screen gave.
            match menu_update {
                MenuUpdate::Pop => {
                    if menu_stack.len() > 1 {
                        menu_stack.pop();
                    }
                }
                MenuUpdate::Push(menu) => {
                    if matches!(
                        menu,
                        Menu::Title | Menu::Game { .. } | Menu::GameOver(_) | Menu::GameComplete(_)
                    ) {
                        menu_stack.clear();
                    }
                    menu_stack.push(menu);
                }
            }
        };
        Ok(msg)
    }
}
