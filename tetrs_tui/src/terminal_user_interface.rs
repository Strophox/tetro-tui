use std::{
    fmt::Debug,
    fs::File,
    io::{self, Read, Write},
    num::NonZeroUsize,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{self, MoveTo},
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{self, Print, PrintStyledContent, Stylize},
    terminal::{self, Clear, ClearType},
    ExecutableCommand, QueueableCommand,
};

use tetrs_engine::{
    piece_generation::TetrominoSource, piece_rotation::RotationSystem, Board, Button, Config,
    FeedbackMessages, Game, GameBuilder, GameOver, Modifier, PressedButtons, Rules, Stat,
    Tetromino,
};

use crate::{
    game_input_handlers::{
        combo_bot_input_handler::ComboBotInputHandler,
        terminal_input_handler::{
            guideline_keybinds, tetrs_default_keybinds, vim_keybinds, Keybinds,
            TerminalInputHandler,
        },
        InputSignal,
    },
    game_modifiers::{self, cheese_game, combo_game, descent_game, puzzle_game},
    game_renderers::{
        color16_palette, diff_renderer::DiffRenderer, empty_palette, fullcolor_palette,
        gruvbox_light_palette, gruvbox_palette, oklch2_palette, tet_str_small, Palette, Renderer,
    },
};

pub type Slots<T> = Vec<(String, T)>;

pub type RecordedUserInput = Vec<(
    tetrs_engine::GameTime,
    u16, /*tetrs_engine::PressedButtons*/
)>;

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GameRestorationData {
    builder: tetrs_engine::GameBuilder,
    mod_identifiers: Vec<String>,
    recorded_user_input: RecordedUserInput,
}

impl GameRestorationData {
    fn new(game: &Game, recorded_user_input: &RecordedUserInput) -> GameRestorationData {
        let (builder, mod_identifiers) = game.blueprint();
        GameRestorationData {
            builder,
            mod_identifiers: mod_identifiers.map(str::to_owned).collect(),
            recorded_user_input: recorded_user_input.clone(),
        }
    }

    fn restore(&self, new_game_settings: &NewGameSettings) -> Game {
        // Step 1: Prepare builder.
        let builder = self.builder.clone();
        // Step 2: Build actual game by identifying mods to finalize builder with.
        let mut game = if let Some(mod_identifier) = self.mod_identifiers.first() {
            if mod_identifier == puzzle_game::MOD_IDENTIFIER {
                puzzle_game::build(&builder)
            } else if mod_identifier == cheese_game::MOD_IDENTIFIER {
                // FIXME: We're guessing the cheese settings based on our CURRENT settings (which might differ from the actual saved game)! But we don't have access to those settings right now.
                cheese_game::build(
                    &builder,
                    new_game_settings.cheese_linelimit,
                    new_game_settings.cheese_gapsize,
                    new_game_settings.cheese_gravity,
                )
            } else if mod_identifier == combo_game::MOD_IDENTIFIER {
                combo_game::build(
                    &builder,
                    new_game_settings.cheese_linelimit,
                    new_game_settings.combo_startlayout,
                )
            } else if mod_identifier == descent_game::MOD_IDENTIFIER {
                descent_game::build(&builder)
            } else {
                let spam_error_msg_modifier = Modifier {
                    identifier: "error_msg_cant_restore_mod".to_owned(),
                    mod_function: Box::new({
                        let m_i = mod_identifier.clone();
                        let mut init = false;
                        move |_config, _rules, state, _modpoint, msgs| {
                            if init {
                                return;
                            }
                            init = true;
                            msgs.push((
                                state.time,
                                tetrs_engine::Feedback::Text(format!(
                                    "ERROR can't restore mod {m_i:?}"
                                )),
                            ));
                        }
                    }),
                };
                builder.build_modified([spam_error_msg_modifier])
            }
        } else {
            builder.build()
        };

        // Step 3: Reenact recorded game inputs.
        let restore_feedback_verbosity = game.config().feedback_verbosity;

        game.config_mut().feedback_verbosity = tetrs_engine::FeedbackVerbosity::Quiet;
        for (input_time, int) in self.recorded_user_input.iter() {
            let button_state = decode_buttons(*int);
            // FIXME: Error handling?
            let _ = game.update(Some(button_state), *input_time);
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
    pub name: String,
    pub comparison_stat: (Stat, bool),
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct ScoreboardEntry {
    meta_data: GameMetaData,
    result: tetrs_engine::GameResult,
    time_elapsed: tetrs_engine::GameTime,
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
    // TODO: remove dead fix-me's like this
    // FIXME: We kind of abuse the SavedGame struct to store stats about old games, but so detailed that we can actually restore a game to its last state.
    custom_rules: Rules,
    custom_seed: Option<u64>,
    custom_board: Option<Board>, // TODO: Option<Board>
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
            cheese_linelimit: Some(NonZeroUsize::try_from(20).unwrap()),
            cheese_gravity: 0,
            cheese_gapsize: 1,
            combo_linelimit: Some(NonZeroUsize::try_from(20).unwrap()),
            combo_startlayout: game_modifiers::combo_game::LAYOUTS[0],
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
        game_renderer: Box<DiffRenderer>,
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
            Menu::Game { meta_data, .. } => &format!("Game ({})", meta_data.name), //&format!("Game {}", game.mode().name.as_ref().map_or("".to_owned(), |ref name| format!("({name})"))),
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
    saved_game: Option<(GameMetaData, GameRestorationData)>,
    scoreboard: Scoreboard,
}

impl<T: Write> Drop for Application<T> {
    fn drop(&mut self) {
        // FIXME: Handle errors?
        let savefile_path = Self::savefile_path();
        // If the user wants any of their data stored, try to do so.
        if self.save_on_exit != SavefileGranularity::NoSavefile {
            if let Err(_e) = self.store_savefile(savefile_path) {
                // FIXME: Make this debuggable.
                //eprintln!("Could not save settings this time: {e} ");
                //std::thread::sleep(Duration::from_secs(4));
            }
        // Otherwise explicitly check for savefile to make sure it's removed.
        } else if savefile_path.try_exists().is_ok_and(|exists| exists) {
            let _ = std::fs::remove_file(savefile_path);
        }
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
        // FIXME: Handle errors?
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
            saved_game: None,
            save_on_exit: SavefileGranularity::NoSavefile,
        };

        // Actually load in settings.
        if app.load_savefile(Self::savefile_path()).is_err() {
            // FIXME: Make this debuggable.
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
            app.new_game_settings.custom_board = custom_start_board.map(|s| decode_board(&s));
        }
        if custom_start_seed.is_some() {
            app.new_game_settings.custom_seed = custom_start_seed;
        }
        app
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
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
            &self.saved_game,
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
            self.saved_game,
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
            pg1.meta_data.name.cmp(&pg2.meta_data.name).then_with(||
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
        // Preparing main application loop.
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

    pub(crate) fn fetch_main_xy() -> (u16, u16) {
        let (w_console, h_console) = terminal::size().unwrap_or((0, 0));
        (
            w_console.saturating_sub(Self::W_MAIN) / 2,
            h_console.saturating_sub(Self::H_MAIN) / 2,
        )
    }

    fn generic_placeholder_menu(
        &mut self,
        current_menu_name: &str,
        selection: Vec<Menu>,
    ) -> io::Result<MenuUpdate> {
        let mut easteregg = 0isize;
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            if current_menu_name.is_empty() {
                self.term
                    .queue(Clear(ClearType::All))?
                    .queue(MoveTo(x_main, y_main + y_selection))?
                    .queue(Print(format!("{:^w_main$}", "▀█▀ ██ ▀█▀ █▀▀ ▄█▀")))?
                    .queue(MoveTo(x_main, y_main + y_selection + 1))?
                    .queue(Print(format!("{:^w_main$}", "    █▄▄▄▄▄▄       ")))?;
            } else {
                self.term
                    .queue(Clear(ClearType::All))?
                    .queue(MoveTo(x_main, y_main + y_selection))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        format!("- {} -", current_menu_name)
                    )))?
                    .queue(MoveTo(x_main, y_main + y_selection + 2))?
                    .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            }
            let names = selection
                .iter()
                .map(|menu| menu.to_string())
                .collect::<Vec<_>>();
            let n_names = names.len();
            if n_names == 0 {
                self.term
                    .queue(MoveTo(x_main, y_main + y_selection + 5))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        "(There isn't anything interesting implemented here yet... )",
                    )))?;
            } else {
                for (i, name) in names.into_iter().enumerate() {
                    self.term
                        .queue(MoveTo(
                            x_main,
                            y_main + y_selection + 4 + u16::try_from(i).unwrap(),
                        ))?
                        .queue(Print(format!(
                            "{:^w_main$}",
                            if i == selected {
                                format!(">> {name} <<")
                            } else {
                                name
                            }
                        )))?;
                }
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(n_names).unwrap() + 2,
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            "(Controls: [←][↓][↑][→] [Esc][Enter][Del] / hjklqed)",
                        )
                        .italic(),
                    ))?;
            }
            if easteregg.abs() == 42 {
                self.term
                    .queue(Clear(ClearType::All))?
                    .queue(MoveTo(0, y_main))?
                    .queue(PrintStyledContent(DAVIS.italic()))?;
            }
            self.term.flush()?;
            // Wait for new input.
            match event::read()? {
                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => {
                    break Ok(MenuUpdate::Push(Menu::Quit(
                        "exited with ctrl-c".to_owned(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e'),
                    kind: Press,
                    ..
                }) => {
                    if !selection.is_empty() {
                        let menu = selection.into_iter().nth(selected).unwrap();
                        break Ok(MenuUpdate::Push(menu));
                    }
                }
                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !selection.is_empty() {
                        selected += selection.len() - 1;
                    }
                    easteregg -= 1;
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !selection.is_empty() {
                        selected += 1;
                    }
                    easteregg += 1;
                }
                // Other event: don't care.
                _ => {}
            }
            if !selection.is_empty() {
                selected = selected.rem_euclid(selection.len());
            }
        }
    }

    fn menu_title(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::About,
            Menu::Quit("quit from title menu".to_owned()),
        ];
        self.generic_placeholder_menu("", selection)
    }

    fn menu_new_game(&mut self) -> io::Result<MenuUpdate> {
        let mut selected = 0usize;
        let mut customization_selected = 0usize;
        let (d_time, d_score, d_pieces, d_lines, d_gravity) =
            (Duration::from_secs(5), 100, 1, 1, 1);
        loop {
            #[allow(clippy::type_complexity)]
            let mut game_presets: Vec<(
                String,
                (Stat, bool),
                String,
                Box<dyn Fn(&GameBuilder) -> Game>,
            )> = vec![
                (
                    "40-Lines".to_owned(),
                    (Stat::TimeElapsed(Duration::ZERO), true),
                    "How fast can you clear forty lines?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::forty_lines()).build()
                    }),
                ),
                (
                    "Marathon".to_owned(),
                    (Stat::PointsScored(0), false),
                    "Can you make it to level 15?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::marathon()).build()
                    }),
                ),
                (
                    "Time Trial".to_owned(),
                    (Stat::PointsScored(0), false),
                    "What highscore can you get in 3 minutes?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::time_trial()).build()
                    }),
                ),
                (
                    "Master".to_owned(),
                    (Stat::PointsScored(0), false),
                    "Can you clear 15 levels at instant gravity?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::master()).build()
                    }),
                ),
                (
                    "Puzzle".to_owned(),
                    (Stat::TimeElapsed(Duration::ZERO), true),
                    "Get perfect clears in all 24 puzzle levels.".to_owned(),
                    Box::new(game_modifiers::puzzle_game::build),
                ),
                (
                    format!(
                        "{}Cheese",
                        if let Some(limit) = self.new_game_settings.cheese_linelimit {
                            format!("{limit}-")
                        } else {
                            "".to_owned()
                        }
                    ),
                    (Stat::PiecesLocked(0), true),
                    format!(
                        "Eat through lines like Swiss cheese. Limit: {:?}",
                        self.new_game_settings.cheese_linelimit
                    ),
                    Box::new({
                        let cheese_limit = self.new_game_settings.cheese_linelimit;
                        let cheese_gap_size = self.new_game_settings.cheese_gapsize;
                        let cheese_gravity = self.new_game_settings.cheese_gravity;
                        move |builder: &GameBuilder| {
                            game_modifiers::cheese_game::build(
                                builder,
                                cheese_limit,
                                cheese_gap_size,
                                cheese_gravity,
                            )
                        }
                    }),
                ),
                (
                    format!(
                        "{}Combo",
                        if let Some(limit) = self.new_game_settings.combo_linelimit {
                            format!("{limit}-")
                        } else {
                            "".to_owned()
                        }
                    ),
                    (Stat::TimeElapsed(Duration::ZERO), true),
                    format!(
                        "Get consecutive line clears. Limit: {:?}{}",
                        self.new_game_settings.combo_linelimit,
                        if self.new_game_settings.combo_startlayout != combo_game::LAYOUTS[0] {
                            format!(", Layout={:b}", self.new_game_settings.combo_startlayout)
                        } else {
                            "".to_owned()
                        }
                    ),
                    Box::new({
                        let combo_limit = self.new_game_settings.combo_linelimit;
                        let combo_start_layout = self.new_game_settings.combo_startlayout;
                        move |builder: &GameBuilder| {
                            game_modifiers::combo_game::build(
                                builder,
                                combo_limit,
                                combo_start_layout,
                            )
                        }
                    }),
                ),
            ];
            if self.new_game_settings.experimental_mode_unlocked {
                game_presets.insert(
                    5,
                    (
                        "Descent (experimental)".to_owned(),
                        (Stat::PointsScored(0), false),
                        "Spin the piece and collect 'gems' by touching them.".to_owned(),
                        Box::new(game_modifiers::descent_game::build),
                    ),
                )
            }
            // First part: rendering the menu.
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            // There are the normal, special, + the custom gamemode.
            let selection_size = game_presets.len() + 1;
            // There are four columns for the custom stat selection.
            let customization_selection_size = 4;
            selected = selected.rem_euclid(selection_size);
            customization_selected =
                customization_selected.rem_euclid(customization_selection_size);
            // Render menu title.
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "+ Start New Game +")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            // Render normal and special gamemodes.
            for (i, (name, _, desc, _)) in game_presets.iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(i + if 4 <= i { 1 } else { 0 }).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if i == selected {
                            format!(">> {name}: {desc} <<")
                        } else {
                            name.to_string()
                        }
                    )))?;
            }
            // Render custom mode option.
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(game_presets.len() + 1 + 1).unwrap(),
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == selection_size - 1 {
                        if customization_selected > 0 {
                            " | Custom:                             "
                        } else {
                            ">> Custom: [→] ([Del]=reset)            "
                        }
                    } else {
                        "Custom"
                    }
                )))?;
            // Render custom mode stuff.
            if selected == selection_size - 1 {
                let stats_strs = [
                    format!(
                        "| Initial gravity: {}",
                        self.new_game_settings.custom_rules.initial_gravity
                    ),
                    format!(
                        "| Auto-increase gravity: {}",
                        self.new_game_settings.custom_rules.progressive_gravity
                    ),
                    format!(
                        "| Limit: {:?} [→]",
                        self.new_game_settings.custom_rules.end_conditions.first()
                    ),
                ];
                for (j, stat_str) in stats_strs.into_iter().enumerate() {
                    self.term
                        .queue(MoveTo(
                            x_main + 25 + 4 * u16::try_from(j).unwrap(),
                            y_main
                                + y_selection
                                + 4
                                + u16::try_from(2 + j + selection_size).unwrap(),
                        ))?
                        .queue(Print(if j + 1 == customization_selected {
                            format!(
                                ">{stat_str}{}",
                                if customization_selected != 3
                                    || !self
                                        .new_game_settings
                                        .custom_rules
                                        .end_conditions
                                        .is_empty()
                                {
                                    " [↓|↑]"
                                } else {
                                    ""
                                }
                            )
                        } else {
                            stat_str
                        }))?;
                }
            }
            self.term.flush()?;
            // Wait for new input.
            match event::read()? {
                // Quit app.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => {
                    break Ok(MenuUpdate::Push(Menu::Quit(
                        "exited with ctrl-c".to_owned(),
                    )))
                }
                // Exit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Try select mode.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e'),
                    kind: Press,
                    ..
                }) => {
                    // Build one of the selected game modes.
                    let (game, meta_data, recorded_user_input) = if selected < game_presets.len() {
                        let (name, comparison_stat, _desc, build) = &game_presets[selected];
                        let preset_game =
                            build(&Game::builder().config(self.settings.config().clone()));
                        let new_meta_data = GameMetaData {
                            datetime: chrono::Utc::now().format("%Y-%m-%d_%H:%M").to_string(),
                            name: (*name).to_owned(),
                            comparison_stat: *comparison_stat,
                        };
                        let new_recorded_user_input = RecordedUserInput::new();
                        (preset_game, new_meta_data, new_recorded_user_input)
                    // Load saved game.
                    } else if let Some((game_meta_data, game_restoration_data)) = &self.saved_game {
                        let restored_game = game_restoration_data.restore(&self.new_game_settings);
                        let mut restored_meta_data = game_meta_data.clone();
                        restored_meta_data.name.push('\'');
                        let restored_recorded_user_input =
                            game_restoration_data.recorded_user_input.clone();
                        (
                            restored_game,
                            restored_meta_data,
                            restored_recorded_user_input,
                        )
                    // Build custom game.
                    } else {
                        let mut builder = Game::builder()
                            .config(self.settings.config().clone())
                            .rules(self.new_game_settings.custom_rules.clone());
                        // Optionally load custom seed.
                        if self.new_game_settings.custom_seed.is_some() {
                            builder.seed = self.new_game_settings.custom_seed;
                        }
                        // Optionally load custom board.
                        let custom_game = if let Some(board) = &self.new_game_settings.custom_board
                        {
                            builder
                                .build_modified([game_modifiers::utils::custom_start_board(board)])
                        // Otherwise just build a normal custom game.
                        } else {
                            builder.build()
                        };
                        let new_meta_data = GameMetaData {
                            datetime: chrono::Utc::now().format("%Y-%m-%d_%H:%M").to_string(),
                            name: "Custom".to_owned(),
                            comparison_stat: (Stat::PointsScored(0), false),
                        };
                        let new_recorded_user_input = RecordedUserInput::new();
                        (custom_game, new_meta_data, new_recorded_user_input)
                    };
                    let now = Instant::now();
                    let time_started = now - game.state().time;
                    break Ok(MenuUpdate::Push(Menu::Game {
                        game: Box::new(game),
                        meta_data,
                        time_started,
                        last_paused: now,
                        total_pause_duration: Duration::ZERO,
                        recorded_user_input,
                        game_renderer: Default::default(),
                    }));
                }
                // Move selector up or increase stat.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if customization_selected > 0 {
                        match customization_selected {
                            1 => {
                                self.new_game_settings.custom_rules.initial_gravity += d_gravity;
                            }
                            2 => {
                                self.new_game_settings.custom_rules.progressive_gravity ^= true;
                            }
                            3 => {
                                match self
                                    .new_game_settings
                                    .custom_rules
                                    .end_conditions
                                    .first_mut()
                                {
                                    Some((Stat::TimeElapsed(ref mut t), _)) => {
                                        *t += d_time;
                                    }
                                    Some((Stat::PiecesLocked(ref mut p), _)) => {
                                        *p += d_pieces;
                                    }
                                    Some((Stat::LinesCleared(ref mut l), _)) => {
                                        *l += d_lines;
                                    }
                                    Some((Stat::GravityReached(ref mut g), _)) => {
                                        *g += d_gravity;
                                    }
                                    Some((Stat::PointsScored(ref mut s), _)) => {
                                        *s += d_score;
                                    }
                                    None => {}
                                };
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        selected += selection_size - 1;
                    }
                }
                // Move selector down or decrease stat.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    // Selected custom stat; decrease it.
                    if customization_selected > 0 {
                        match customization_selected {
                            1 => {
                                let r = &mut self.new_game_settings.custom_rules.initial_gravity;
                                *r = r.saturating_sub(d_gravity);
                            }
                            2 => {
                                self.new_game_settings.custom_rules.progressive_gravity ^= true;
                            }
                            3 => {
                                match self
                                    .new_game_settings
                                    .custom_rules
                                    .end_conditions
                                    .first_mut()
                                {
                                    Some((Stat::TimeElapsed(ref mut t), _)) => {
                                        *t = t.saturating_sub(d_time);
                                    }
                                    Some((Stat::PiecesLocked(ref mut p), _)) => {
                                        *p = p.saturating_sub(d_pieces);
                                    }
                                    Some((Stat::LinesCleared(ref mut l), _)) => {
                                        *l = l.saturating_sub(d_lines);
                                    }
                                    Some((Stat::GravityReached(ref mut g), _)) => {
                                        *g = g.saturating_sub(d_gravity);
                                    }
                                    Some((Stat::PointsScored(ref mut s), _)) => {
                                        *s = s.saturating_sub(d_score);
                                    }
                                    None => {}
                                };
                            }
                            _ => unreachable!(),
                        }
                    // Move gamemode selector
                    } else {
                        selected += 1;
                    }
                }
                // Move selector left (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == selection_size - 1 && customization_selected > 0 {
                        customization_selected += customization_selection_size - 1
                    } else if selected == selection_size - 2 {
                        if let Some(limit) = self.new_game_settings.combo_linelimit {
                            self.new_game_settings.combo_linelimit =
                                NonZeroUsize::try_from(limit.get() - 1).ok();
                        }
                    } else if selected == selection_size - 3 {
                        if let Some(limit) = self.new_game_settings.cheese_linelimit {
                            self.new_game_settings.cheese_linelimit =
                                NonZeroUsize::try_from(limit.get() - 1).ok();
                        }
                    }
                }
                // Move selector right (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    // If custom gamemode selected, allow incrementing stat selection.
                    if selected == selection_size - 1 {
                        // If reached last stat, cycle through stats for limit.
                        if customization_selected == customization_selection_size - 1 {
                            self.new_game_settings.custom_rules.end_conditions =
                                match self.new_game_settings.custom_rules.end_conditions.first() {
                                    Some((Stat::TimeElapsed(_), _)) => {
                                        vec![(Stat::PointsScored(9000), true)]
                                    }
                                    Some((Stat::PointsScored(_), _)) => {
                                        vec![(Stat::PiecesLocked(100), true)]
                                    }
                                    Some((Stat::PiecesLocked(_), _)) => {
                                        vec![(Stat::LinesCleared(40), true)]
                                    }
                                    Some((Stat::LinesCleared(_), _)) => {
                                        vec![(Stat::GravityReached(20), true)]
                                    }
                                    Some((Stat::GravityReached(_), _)) => vec![],
                                    None => {
                                        vec![(Stat::TimeElapsed(Duration::from_secs(180)), true)]
                                    }
                                };
                        } else {
                            customization_selected += 1
                        }
                    } else if selected == selection_size - 2 {
                        self.new_game_settings.combo_linelimit =
                            if let Some(limit) = self.new_game_settings.combo_linelimit {
                                limit.checked_add(1)
                            } else {
                                Some(NonZeroUsize::MIN)
                            };
                    } else if selected == selection_size - 3 {
                        self.new_game_settings.cheese_linelimit =
                            if let Some(limit) = self.new_game_settings.cheese_linelimit {
                                limit.checked_add(1)
                            } else {
                                Some(NonZeroUsize::MIN)
                            };
                    }
                }
                // Move selector right (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    // If custom gamemode selected, allow deleting TODO
                    if selected == selection_size - 1 {
                        self.saved_game = None;
                        self.new_game_settings.custom_seed = None;
                        self.new_game_settings.custom_board = None;
                        self.new_game_settings.custom_rules = Rules::default();
                    } else if selected == selection_size - 2 {
                        let new_layout_idx = if let Some(i) =
                            crate::game_modifiers::combo_game::LAYOUTS
                                .iter()
                                .position(|lay| *lay == self.new_game_settings.combo_startlayout)
                        {
                            let layout_cnt = crate::game_modifiers::combo_game::LAYOUTS.len();
                            (i + 1) % layout_cnt
                        } else {
                            0
                        };
                        self.new_game_settings.combo_startlayout =
                            crate::game_modifiers::combo_game::LAYOUTS[new_layout_idx];
                    }
                }
                // Other event: don't care.
                _ => {}
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn menu_game(
        &mut self,
        game: &mut Game,
        meta_data: &mut GameMetaData,
        time_started: &Instant,
        time_last_paused: &mut Instant,
        total_pause_duration: &mut Duration,
        recorded_user_input: &mut RecordedUserInput,
        game_renderer: &mut impl Renderer,
    ) -> io::Result<MenuUpdate> {
        if self.runtime_data.kitty_assumed {
            // FIXME: Kinda iffy. Do we need all flags? What undesirable effects might there be?
            let _ = self.term.execute(event::PushKeyboardEnhancementFlags(
                event::KeyboardEnhancementFlags::all(),
                // event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
            ));
        }
        // Prepare channel with which to communicate `Button` inputs / game interrupt.
        let mut buttons_pressed = tetrs_engine::PressedButtons::default();
        let (button_sender, button_receiver) = mpsc::channel();
        let _input_handler = TerminalInputHandler::new(
            &button_sender,
            self.settings.keybinds(),
            self.runtime_data.kitty_assumed,
        );
        let mut combo_bot_handler = (self.runtime_data.combo_bot_enabled
            && meta_data.name == "Combo")
            .then(|| ComboBotInputHandler::new(&button_sender, Duration::from_millis(100)));
        let mut inform_combo_bot = |game: &Game, evts: &FeedbackMessages| {
            if let Some((_, state_sender)) = &mut combo_bot_handler {
                if evts.iter().any(|(_, feedback)| {
                    matches!(feedback, tetrs_engine::Feedback::PieceSpawned(_))
                }) {
                    let combo_state = ComboBotInputHandler::encode(game).unwrap();
                    if state_sender.send(combo_state).is_err() {
                        combo_bot_handler = None;
                    }
                }
            }
        };
        // Game Loop
        let session_resumed = Instant::now();
        *total_pause_duration += session_resumed.saturating_duration_since(*time_last_paused);
        let mut clean_screen = true;
        let mut f = 0u32;
        let mut fps_counter = 0;
        let mut fps_counter_started = Instant::now();
        let menu_update = 'render: loop {
            // Exit if game ended
            if let Some(game_result) = game.state().result {
                let scoreboard_entry = ScoreboardEntry::new(game, meta_data);
                let game_restoration_data = GameRestorationData::new(game, recorded_user_input);
                self.scoreboard
                    .entries
                    .push((scoreboard_entry.clone(), Some(game_restoration_data)));
                let menu = if game_result.is_ok() {
                    Menu::GameComplete
                } else {
                    Menu::GameOver
                }(Box::new(scoreboard_entry));
                break 'render MenuUpdate::Push(menu);
            }
            // Start next frame
            f += 1;
            fps_counter += 1;
            let next_frame_at = loop {
                let frame_at = session_resumed
                    + Duration::from_secs_f64(f64::from(f) / self.settings.graphics().game_fps);
                if frame_at < Instant::now() {
                    f += 1;
                } else {
                    break frame_at;
                }
            };
            let mut new_feedback_msgs = Vec::new();
            'frame_idle: loop {
                let frame_idle_remaining = next_frame_at - Instant::now();
                match button_receiver.recv_timeout(frame_idle_remaining) {
                    Ok(InputSignal::AbortProgram) => {
                        break 'render MenuUpdate::Push(Menu::Quit(
                            "exited with ctrl-c".to_owned(),
                        ));
                    }
                    Ok(InputSignal::ForfeitGame) => {
                        game.forfeit();
                        let scoreboard_entry = ScoreboardEntry::new(game, meta_data);
                        let game_restoration_data =
                            GameRestorationData::new(game, recorded_user_input);
                        self.scoreboard
                            .entries
                            .push((scoreboard_entry.clone(), Some(game_restoration_data)));
                        break 'render MenuUpdate::Push(Menu::GameOver(Box::new(scoreboard_entry)));
                    }
                    Ok(InputSignal::Pause) => {
                        *time_last_paused = Instant::now();
                        break 'render MenuUpdate::Push(Menu::Pause);
                    }
                    Ok(InputSignal::WindowResize) => {
                        clean_screen = true;
                        continue 'frame_idle;
                    }
                    Ok(InputSignal::StoreSavepoint) => {
                        let _ = self.saved_game.insert((
                            meta_data.clone(),
                            GameRestorationData::new(game, recorded_user_input),
                        ));
                        new_feedback_msgs.push((
                            game.state().time,
                            tetrs_engine::Feedback::Text("(Savepoint captured.)".to_owned()),
                        ));
                    }
                    Ok(InputSignal::StoreSeed) => {
                        let _ = self.new_game_settings.custom_seed.insert(game.seed());
                        new_feedback_msgs.push((
                            game.state().time,
                            tetrs_engine::Feedback::Text("(Seed captured.)".to_owned()),
                        ));
                    }
                    Ok(InputSignal::StoreBoard) => {
                        let _ = self
                            .new_game_settings
                            .custom_board
                            .insert(decode_board(&encode_board(&game.state().board)));
                        new_feedback_msgs.push((
                            game.state().time,
                            tetrs_engine::Feedback::Text("(Board captured.)".to_owned()),
                        ));
                    }
                    Ok(InputSignal::ButtonInput(button, button_state, instant)) => {
                        buttons_pressed[button] = button_state;
                        let game_time_userinput = instant.saturating_duration_since(*time_started)
                            - *total_pause_duration;
                        let game_now = std::cmp::max(game_time_userinput, game.state().time);
                        // FIXME: Handle/ensure no Err.
                        recorded_user_input.push((game_now, encode_buttons(&buttons_pressed)));
                        if let Ok(evts) = game.update(Some(buttons_pressed), game_now) {
                            inform_combo_bot(game, &evts);
                            new_feedback_msgs.extend(evts);
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let game_time_now = Instant::now().saturating_duration_since(*time_started)
                            - *total_pause_duration;
                        // FIXME: Handle/ensure no Err.
                        if let Ok(evts) = game.update(None, game_time_now) {
                            inform_combo_bot(game, &evts);
                            new_feedback_msgs.extend(evts);
                        }
                        break 'frame_idle;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // NOTE: We kind of rely on this not happening too often.
                        break 'render MenuUpdate::Push(Menu::Pause);
                    }
                };
            }
            game_renderer.render(self, game, meta_data, new_feedback_msgs, clean_screen)?;
            clean_screen = false;
            // FPS counter.
            if self.settings.graphics().show_fps {
                let now = Instant::now();
                if now.saturating_duration_since(fps_counter_started) >= Duration::from_secs(1) {
                    self.term
                        .execute(MoveTo(0, 0))?
                        .execute(Print(format!("{:_>6}", format!("{fps_counter}fps"))))?;
                    fps_counter = 0;
                    fps_counter_started = now;
                }
            }
        };
        // Console epilogue: De-initialization.
        if self.runtime_data.kitty_assumed {
            let _ = self.term.execute(event::PopKeyboardEnhancementFlags);
        }
        if let Some(finished_state) = game.state().result {
            let h_console = terminal::size()?.1;
            if finished_state.is_ok() {
                for i in 0..h_console {
                    self.term
                        .execute(MoveTo(0, i))?
                        .execute(Clear(ClearType::CurrentLine))?;
                    std::thread::sleep(Duration::from_secs_f32(0.01));
                }
            } else {
                for i in (0..h_console).rev() {
                    self.term
                        .execute(MoveTo(0, i))?
                        .execute(Clear(ClearType::CurrentLine))?;
                    std::thread::sleep(Duration::from_secs_f32(0.01));
                }
            };
        }
        Ok(menu_update)
    }

    fn menu_game_ended(&mut self, past_game: &ScoreboardEntry) -> io::Result<MenuUpdate> {
        let ScoreboardEntry {
            meta_data,
            result,
            time_elapsed,
            pieces_locked,
            lines_cleared,
            gravity_reached,
            points_scored,
        } = past_game;
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::Quit("quit after game ended".to_owned()),
        ];
        // if gamemode.name.as_ref().map(String::as_str) == Some("Puzzle")
        if result.is_ok() && meta_data.name == "Puzzle" {
            self.new_game_settings.experimental_mode_unlocked = true;
        }
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    match result {
                        Ok(()) => format!("++ Game Completed ({}) ++", meta_data.name),
                        Err(game_over_cause) => format!(
                            "-- Game Over ({}) by: {game_over_cause:?} --",
                            meta_data.name
                        ),
                    }
                )))?
                /*.queue(MoveTo(0, y_main + y_selection + 2))?
                .queue(Print(Self::produce_header()?))?*/
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?
                .queue(MoveTo(x_main, y_main + y_selection + 3))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Score: {points_scored}")
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 4))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Lines: {}", lines_cleared)
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 5))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Tetrominos locked: {}", pieces_locked.iter().sum::<u32>())
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 6))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Gravity reached: {gravity_reached}",)
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 7))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Time elapsed: {}", fmt_duration(time_elapsed))
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 8))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let names = selection
                .iter()
                .map(|menu| menu.to_string())
                .collect::<Vec<_>>();
            for (i, name) in names.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 10 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if i == selected {
                            format!(">> {name} <<")
                        } else {
                            name
                        }
                    )))?;
            }
            self.term.flush()?;
            // Wait for new input.
            match event::read()? {
                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => {
                    break Ok(MenuUpdate::Push(Menu::Quit(
                        "exited with ctrl-c".to_owned(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e'),
                    kind: Press,
                    ..
                }) => {
                    if !selection.is_empty() {
                        let menu = selection.into_iter().nth(selected).unwrap();
                        break Ok(MenuUpdate::Push(menu));
                    }
                }
                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !selection.is_empty() {
                        selected += selection.len() - 1;
                    }
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !selection.is_empty() {
                        selected += 1;
                    }
                }
                // Other event: don't care.
                _ => {}
            }
            if !selection.is_empty() {
                selected = selected.rem_euclid(selection.len());
            }
        }
    }

    fn menu_pause(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::About,
            Menu::Quit("quit from pause".to_owned()),
        ];
        self.generic_placeholder_menu("Game Paused", selection)
    }

    fn menu_settings(&mut self) -> io::Result<MenuUpdate> {
        let selection_len = 4;
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "% Settings %")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let labels = [
                "Adjust Graphics...".to_owned(),
                "Adjust Keybinds...".to_owned(),
                "Adjust Gameplay...".to_owned(),
                format!(
                    "Keep save file: {}",
                    match self.save_on_exit {
                        SavefileGranularity::NoSavefile => "OFF*",
                        SavefileGranularity::RememberSettings => "ON (save settings)",
                        SavefileGranularity::RememberSettingsScoreboard =>
                            "ON (save settings, scores)",
                        SavefileGranularity::RememberSettingsScoreboardGamereplays =>
                            "ON (save settings, scores, game replays)",
                    }
                ),
            ];
            for (i, label) in labels.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if i == selected {
                            format!(">> {label} <<")
                        } else {
                            label
                        }
                    )))?;
            }
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(selection_len).unwrap() + 1,
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        if self.save_on_exit == SavefileGranularity::NoSavefile {
                            "(*WARNING: current data will be lost on exit)".to_owned()
                        } else {
                            format!("(Save file at {:?})", Self::savefile_path())
                        },
                    )
                    .italic(),
                ))?;
            self.term.flush()?;
            // Wait for new input.
            match event::read()? {
                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => {
                    break Ok(MenuUpdate::Push(Menu::Quit(
                        "exited with ctrl-c".to_owned(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e'),
                    kind: Press,
                    ..
                }) => match selected {
                    0 => break Ok(MenuUpdate::Push(Menu::AdjustGraphics)),
                    1 => break Ok(MenuUpdate::Push(Menu::AdjustKeybinds)),
                    2 => break Ok(MenuUpdate::Push(Menu::AdjustGameplay)),
                    3 => {
                        self.save_on_exit =
                            SavefileGranularity::RememberSettingsScoreboardGamereplays;
                    }
                    _ => {}
                },
                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += selection_len - 1;
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += 1;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 3 {
                        self.save_on_exit = match self.save_on_exit {
                            SavefileGranularity::NoSavefile => {
                                SavefileGranularity::RememberSettingsScoreboardGamereplays
                            }
                            SavefileGranularity::RememberSettingsScoreboardGamereplays => {
                                SavefileGranularity::RememberSettingsScoreboard
                            }
                            SavefileGranularity::RememberSettingsScoreboard => {
                                SavefileGranularity::RememberSettings
                            }
                            SavefileGranularity::RememberSettings => {
                                SavefileGranularity::NoSavefile
                            }
                        };
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 3 {
                        self.save_on_exit = match self.save_on_exit {
                            SavefileGranularity::NoSavefile => {
                                SavefileGranularity::RememberSettings
                            }
                            SavefileGranularity::RememberSettings => {
                                SavefileGranularity::RememberSettingsScoreboard
                            }
                            SavefileGranularity::RememberSettingsScoreboard => {
                                SavefileGranularity::RememberSettingsScoreboardGamereplays
                            }
                            SavefileGranularity::RememberSettingsScoreboardGamereplays => {
                                SavefileGranularity::NoSavefile
                            }
                        };
                    }
                }

                // Set save_on_exit to false.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 3 {
                        self.save_on_exit = SavefileGranularity::NoSavefile;
                    }
                }

                // Other event: Just ignore.
                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }

    fn menu_adjust_graphics(&mut self) -> io::Result<MenuUpdate> {
        let if_slot_is_default_then_copy_and_switch = |settings: &mut Settings| {
            if settings.graphics_slot_active < settings.graphics_slots_that_should_not_be_changed {
                let mut n = 1;
                let new_custom_slot_name = loop {
                    let name = format!("custom_{n}");
                    if settings.graphics_slots.iter().any(|s| s.0 == name) {
                        n += 1;
                    } else {
                        break name;
                    }
                };
                let new_slot = (new_custom_slot_name, *settings.graphics());
                settings.graphics_slots.push(new_slot);
                settings.graphics_slot_active = settings.graphics_slots.len() - 1;
            }
        };
        let selection_len = 8;
        let mut selected = 1usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "# Graphics Settings #")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw slot label.
            let slot_label = format!(
                "Slot ({}/{}): \"{}\"{}",
                self.settings.graphics_slot_active + 1,
                self.settings.graphics_slots.len(),
                self.settings.graphics_slots[self.settings.graphics_slot_active].0,
                if self.settings.graphics_slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.graphics_slot_active
                            < self.settings.graphics_slots_that_should_not_be_changed
                        {
                            ""
                        } else {
                            "Del|"
                        }
                    )
                }
            );
            self.term
                .queue(MoveTo(x_main, y_main + y_selection + 3))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == 0 {
                        format!(">> {slot_label} <<")
                    } else {
                        slot_label
                    }
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 4))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            let labels = [
                format!("Glyphset: {:?}", self.settings.graphics().glyphset),
                format!(
                    "Color palette: '{}'",
                    self.settings.palette_slots[self.settings.graphics().palette_active].0
                ),
                format!(
                    "Colored locked tiles: {}",
                    self.settings.graphics().palette_active_lockedtiles != 0
                ),
                format!(
                    "Render effects: {}",
                    self.settings.graphics().render_effects
                ),
                format!(
                    "Show ghost piece: {}",
                    self.settings.graphics().show_ghost_piece
                ),
                format!("Framerate: {}", self.settings.graphics().game_fps),
                format!("Show fps: {}", self.settings.graphics().show_fps),
            ];
            for (i, label) in labels.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 6 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if i + 1 == selected {
                            format!(">> {label} <<")
                        } else {
                            label
                        }
                    )))?;
            }
            self.term.queue(MoveTo(
                x_main + u16::try_from((w_main - 27) / 2).unwrap(),
                y_main + y_selection + 6 + u16::try_from(selection_len).unwrap() + 2,
            ))?;
            for tet in Tetromino::VARIANTS {
                self.term.queue(PrintStyledContent(
                    tet_str_small(&tet).with(
                        *self
                            .settings
                            .palette()
                            .get(&tet.tiletypeid().get())
                            .unwrap_or(&style::Color::Reset),
                    ),
                ))?;
                self.term.queue(Print(' '))?;
            }
            self.term.flush()?;

            // Wait for new input.
            match event::read()? {
                // Abort program.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => {
                    break Ok(MenuUpdate::Push(Menu::Quit(
                        "exited with ctrl-c".to_owned(),
                    )))
                }

                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += selection_len - 1;
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += 1;
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.graphics_slot_active += 1;
                        self.settings.graphics_slot_active %= self.settings.graphics_slots.len();
                    }
                    1 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().glyphset =
                            match self.settings.graphics().glyphset {
                                Glyphset::Electronika60 => Glyphset::ASCII,
                                Glyphset::ASCII => Glyphset::Unicode,
                                Glyphset::Unicode => Glyphset::Electronika60,
                            };
                    }
                    2 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_active += 1;
                        self.settings.graphics_mut().palette_active %=
                            self.settings.palette_slots.len();
                        self.settings.graphics_mut().palette_active_lockedtiles =
                            self.settings.graphics_mut().palette_active;
                    }
                    3 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_active_lockedtiles =
                            if self.settings.graphics().palette_active_lockedtiles == 0 {
                                self.settings.graphics_mut().palette_active
                            } else {
                                0
                            };
                    }
                    4 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().render_effects ^= true;
                    }
                    5 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_ghost_piece ^= true;
                    }
                    6 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().game_fps += 1.0;
                    }
                    7 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_fps ^= true;
                    }
                    _ => {}
                },

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.graphics_slot_active +=
                            self.settings.graphics_slots.len() - 1;
                        self.settings.graphics_slot_active %= self.settings.graphics_slots.len();
                    }
                    1 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().glyphset =
                            match self.settings.graphics().glyphset {
                                Glyphset::Electronika60 => Glyphset::Unicode,
                                Glyphset::ASCII => Glyphset::Electronika60,
                                Glyphset::Unicode => Glyphset::ASCII,
                            };
                    }
                    2 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_active +=
                            self.settings.palette_slots.len() - 1;
                        self.settings.graphics_mut().palette_active %=
                            self.settings.palette_slots.len();
                        self.settings.graphics_mut().palette_active_lockedtiles =
                            self.settings.graphics_mut().palette_active;
                    }
                    3 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_active_lockedtiles =
                            if self.settings.graphics().palette_active_lockedtiles == 0 {
                                self.settings.graphics_mut().palette_active
                            } else {
                                0
                            };
                    }
                    4 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().render_effects ^= true;
                    }
                    5 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_ghost_piece ^= true;
                    }
                    6 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        if self.settings.graphics().game_fps >= 1.0 {
                            self.settings.graphics_mut().game_fps -= 1.0;
                        }
                    }
                    7 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_fps ^= true;
                    }
                    _ => {}
                },

                // Reset graphics, or delete entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press,
                    ..
                }) => {
                    if selected == 0 {
                        // If a custom slot, then remove it (and return to the 'default' 0th slot).
                        if self.settings.graphics_slot_active
                            >= self.settings.graphics_slots_that_should_not_be_changed
                        {
                            self.settings
                                .graphics_slots
                                .remove(self.settings.graphics_slot_active);
                            self.settings.graphics_slot_active = 0;
                        }
                    }
                }

                // Other event: Just ignore.
                _ => {}
            }
            selected %= selection_len;
        }
    }

    fn menu_adjust_keybinds(&mut self) -> io::Result<MenuUpdate> {
        // "Trying to modify a default slot: create copy of slot to allow safely modifying that."
        let if_slot_is_default_then_copy_and_switch = |settings: &mut Settings| {
            if settings.keybinds_slot_active < settings.keybinds_slots_that_should_not_be_changed {
                let mut n = 1;
                let new_custom_slot_name = loop {
                    let name = format!("custom_{n}");
                    if settings.keybinds_slots.iter().any(|s| s.0 == name) {
                        n += 1;
                    } else {
                        break name;
                    }
                };
                let new_slot = (new_custom_slot_name, settings.keybinds().clone());
                settings.keybinds_slots.push(new_slot);
                settings.keybinds_slot_active = settings.keybinds_slots.len() - 1;
            }
        };
        let buttons_available = Button::VARIANTS;
        // +1 for available slot selection.
        let selection_len = 1 + buttons_available.len();
        // Go to actual keybind selection on menu entry.
        let mut selected = 1usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            // Draw menu title.
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "@ Keybinds @")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw slot label.
            let slot_label = format!(
                "Slot ({}/{}): \"{}\"{}",
                self.settings.keybinds_slot_active + 1,
                self.settings.keybinds_slots.len(),
                self.settings.keybinds_slots[self.settings.keybinds_slot_active].0,
                if self.settings.keybinds_slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.keybinds_slot_active
                            < self.settings.keybinds_slots_that_should_not_be_changed
                        {
                            ""
                        } else {
                            "Del|"
                        }
                    )
                }
            );
            self.term
                .queue(MoveTo(x_main, y_main + y_selection + 3))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == 0 {
                        format!(">> {slot_label} <<")
                    } else {
                        slot_label
                    }
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 4))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw keybinds selection.
            let button_names = buttons_available.iter().map(|&button| {
                format!(
                    "{button:?}: {}",
                    fmt_keybinds(button, self.settings.keybinds())
                )
            });
            for (i, name) in button_names.enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 6 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        // +1 because the first button is Slot selection.
                        if i + 1 == selected {
                            format!(">> {name} <<")
                        } else {
                            name
                        }
                    )))?;
            }

            // Draw footer legend.
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 6 + u16::try_from(buttons_available.len()).unwrap() + 1,
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        "(Controls: [Enter]=add [Esc]=cancel [Del]=clear)",
                    )
                    .italic(),
                ))?;
            self.term.flush()?;

            // Wait for new input.
            match event::read()? {
                // Abort program.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => {
                    break Ok(MenuUpdate::Push(Menu::Quit(
                        "exited with ctrl-c".to_owned(),
                    )))
                }

                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                // Modify keybind.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e'),
                    kind: Press,
                    ..
                }) => {
                    // `> 0` because 0 is slot selection.
                    if selected > 0 {
                        let current_button = buttons_available[selected - 1];
                        self.term
                            .execute(MoveTo(
                                x_main,
                                y_main
                                    + y_selection
                                    + 4
                                    + u16::try_from(selection_len).unwrap()
                                    + 2,
                            ))?
                            .execute(PrintStyledContent(
                                format!(
                                    "{:^w_main$}",
                                    format!("Press a key for {current_button:?}..."),
                                )
                                .italic(),
                            ))?
                            .execute(cursor::MoveToNextLine(1))?
                            .execute(Clear(ClearType::CurrentLine))?;
                        // Wait until appropriate keypress detected.
                        if self.runtime_data.kitty_assumed {
                            // FIXME: Kinda iffy. Do we need all flags? What undesirable effects might there be?
                            let _ = self.term.execute(event::PushKeyboardEnhancementFlags(
                                event::KeyboardEnhancementFlags::all(),
                                // event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
                            ));
                        }
                        loop {
                            if let Event::Key(KeyEvent {
                                code, kind: Press, ..
                            }) = event::read()?
                            {
                                // Add key pressed unless it's `Esc`.
                                if code != KeyCode::Esc {
                                    if_slot_is_default_then_copy_and_switch(&mut self.settings);
                                    self.settings.keybinds_mut().insert(code, current_button);
                                }
                                break;
                            }
                        }
                        // Console epilogue: De-initialization.
                        if self.runtime_data.kitty_assumed {
                            let _ = self.term.execute(event::PopKeyboardEnhancementFlags);
                        }
                    }
                }

                // Delete keybind, or entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press,
                    ..
                }) => {
                    if selected == 0 {
                        // If a custom slot, then remove it (and return to the 'default' 0th slot).
                        if self.settings.keybinds_slot_active
                            >= self.settings.keybinds_slots_that_should_not_be_changed
                        {
                            self.settings
                                .keybinds_slots
                                .remove(self.settings.keybinds_slot_active);
                            self.settings.keybinds_slot_active = 0;
                        }
                    } else {
                        // Trying to modify a default slot: create copy of slot to allow safely modifying that.
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        // Remove all keys bound to the selected action button.
                        let button_selected = buttons_available[selected - 1];
                        self.settings
                            .keybinds_mut()
                            .retain(|_code, button| *button != button_selected);
                    }
                }

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += selection_len - 1;
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += 1;
                }

                // Cycle slot to right.
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 0 {
                        self.settings.keybinds_slot_active += 1;
                        self.settings.keybinds_slot_active %= self.settings.keybinds_slots.len();
                    }
                }

                // Cycle slot to right.
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 0 {
                        self.settings.keybinds_slot_active +=
                            self.settings.keybinds_slots.len() - 1;
                        self.settings.keybinds_slot_active %= self.settings.keybinds_slots.len();
                    }
                }

                // Other IO event: no action.
                _ => {}
            }
            selected %= selection_len;
        }
    }

    fn menu_adjust_gameplay(&mut self) -> io::Result<MenuUpdate> {
        let if_slot_is_default_then_copy_and_switch = |settings: &mut Settings| {
            if settings.config_slot_active < settings.config_slots_that_should_not_be_changed {
                let mut n = 1;
                let new_custom_slot_name = loop {
                    let name = format!("custom_{n}");
                    if settings.config_slots.iter().any(|s| s.0 == name) {
                        n += 1;
                    } else {
                        break name;
                    }
                };
                let new_slot = (new_custom_slot_name, settings.config().clone());
                settings.config_slots.push(new_slot);
                settings.config_slot_active = settings.config_slots.len() - 1;
            }
        };
        let selection_len = 10;
        let mut selected = 1usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;

            // Draw menu title.
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    "= Gameplay Configurations (apply on New Game) ="
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw slot label.
            let slot_label = format!(
                "Slot ({}/{}): \"{}\"{}",
                self.settings.config_slot_active + 1,
                self.settings.config_slots.len(),
                self.settings.config_slots[self.settings.config_slot_active].0,
                if self.settings.config_slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.config_slot_active
                            < self.settings.config_slots_that_should_not_be_changed
                        {
                            ""
                        } else {
                            "Del|"
                        }
                    )
                }
            );
            self.term
                .queue(MoveTo(x_main, y_main + y_selection + 3))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == 0 {
                        format!(">> {slot_label} <<")
                    } else {
                        slot_label
                    }
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 4))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw config selection.
            let labels = [
                format!(
                    "Rotation system: {:?}",
                    self.settings.config().rotation_system
                ),
                format!(
                    "Piece generation: {}",
                    match &self.settings.config().tetromino_generator {
                        TetrominoSource::Uniform => "Uniformly random".to_owned(),
                        TetrominoSource::Stock { .. } => "Bag".to_owned(),
                        TetrominoSource::Recency { .. } => "Recency".to_owned(),
                        TetrominoSource::BalanceRelative { .. } =>
                            "Balance relative counts".to_owned(),
                        TetrominoSource::Cycle { pattern, index: _ } =>
                            format!("Cycling pattern {pattern:?}"),
                    }
                ),
                format!("Preview size: {}", self.settings.config().preview_count),
                format!(
                    "Delayed auto shift: {:?} *",
                    self.settings.config().delayed_auto_shift
                ),
                format!(
                    "Auto repeat rate: {:?} *",
                    self.settings.config().auto_repeat_rate
                ),
                format!(
                    "Soft drop factor: {} *",
                    self.settings.config().soft_drop_factor
                ),
                format!(
                    "Line clear delay: {:?}",
                    self.settings.config().line_clear_delay
                ),
                format!(
                    "Appearance delay: {:?}",
                    self.settings.config().appearance_delay
                ),
                format!(
                    "(/!\\ Override) Assume enhanced-key-events: {} *",
                    self.runtime_data.kitty_assumed
                ),
            ];
            for (i, label) in labels.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 6 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if i + 1 == selected {
                            format!(">> {label} <<")
                        } else {
                            label
                        }
                    )))?;
            }
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 6 + u16::try_from(selection_len).unwrap(),
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        if self.runtime_data.kitty_detected {
                            "(*Should apply, since enhanced-key-events seem available)"
                        } else {
                            "(*Might NOT apply since enhanced-key-events seem unavailable)"
                        },
                    )
                    .italic(),
                ))?;

            self.term.flush()?;
            // Wait for new input.
            match event::read()? {
                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => {
                    break Ok(MenuUpdate::Push(Menu::Quit(
                        "exited with ctrl-c".to_owned(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                // Reset config, or delete entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press,
                    ..
                }) => {
                    if selected == 0 {
                        // If a custom slot, then remove it (and return to the 'default' 0th slot).
                        if self.settings.config_slot_active
                            >= self.settings.config_slots_that_should_not_be_changed
                        {
                            self.settings
                                .config_slots
                                .remove(self.settings.config_slot_active);
                            self.settings.config_slot_active = 0;
                        }
                    }
                }

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += selection_len - 1;
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += 1;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.config_slot_active += 1;
                        self.settings.config_slot_active %= self.settings.config_slots.len();
                    }
                    1 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().rotation_system =
                            match self.settings.config().rotation_system {
                                RotationSystem::Ocular => RotationSystem::Classic,
                                RotationSystem::Classic => RotationSystem::Super,
                                RotationSystem::Super => RotationSystem::Ocular,
                            };
                    }
                    2 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().tetromino_generator = match self
                            .settings
                            .config()
                            .tetromino_generator
                        {
                            TetrominoSource::Uniform => TetrominoSource::bag(),
                            TetrominoSource::Stock { .. } => TetrominoSource::recency(),
                            TetrominoSource::Recency { .. } => TetrominoSource::balance_relative(),
                            TetrominoSource::BalanceRelative { .. } => TetrominoSource::uniform(),
                            TetrominoSource::Cycle { .. } => TetrominoSource::uniform(),
                        };
                    }
                    3 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().preview_count += 1;
                    }
                    4 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().delayed_auto_shift += Duration::from_millis(1);
                    }
                    5 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().auto_repeat_rate += Duration::from_millis(1);
                    }
                    6 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().soft_drop_factor += 0.5;
                    }
                    7 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().line_clear_delay += Duration::from_millis(10);
                    }
                    8 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().appearance_delay += Duration::from_millis(10);
                    }
                    9 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.runtime_data.kitty_assumed ^= true;
                    }
                    _ => {}
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.config_slot_active += self.settings.config_slots.len() - 1;
                        self.settings.config_slot_active %= self.settings.config_slots.len();
                    }
                    1 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().rotation_system =
                            match self.settings.config().rotation_system {
                                RotationSystem::Ocular => RotationSystem::Super,
                                RotationSystem::Super => RotationSystem::Classic,
                                RotationSystem::Classic => RotationSystem::Ocular,
                            };
                    }
                    2 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().tetromino_generator =
                            match self.settings.config().tetromino_generator {
                                TetrominoSource::Uniform => TetrominoSource::balance_relative(),
                                TetrominoSource::Stock { .. } => TetrominoSource::uniform(),
                                TetrominoSource::Recency { .. } => TetrominoSource::bag(),
                                TetrominoSource::BalanceRelative { .. } => {
                                    TetrominoSource::recency()
                                }
                                TetrominoSource::Cycle { .. } => TetrominoSource::uniform(),
                            };
                    }
                    3 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().preview_count =
                            self.settings.config().preview_count.saturating_sub(1);
                    }
                    4 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().delayed_auto_shift = self
                            .settings
                            .config()
                            .delayed_auto_shift
                            .saturating_sub(Duration::from_millis(1));
                    }
                    5 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().auto_repeat_rate = self
                            .settings
                            .config()
                            .auto_repeat_rate
                            .saturating_sub(Duration::from_millis(1));
                    }
                    6 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        if self.settings.config().soft_drop_factor > 0.0 {
                            self.settings.config_mut().soft_drop_factor -= 0.5;
                        }
                    }
                    7 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().line_clear_delay = self
                            .settings
                            .config()
                            .line_clear_delay
                            .saturating_sub(Duration::from_millis(10));
                    }
                    8 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().appearance_delay = self
                            .settings
                            .config()
                            .appearance_delay
                            .saturating_sub(Duration::from_millis(10));
                    }
                    9 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.runtime_data.kitty_assumed ^= true;
                    }
                    _ => {}
                },
                // Other event: don't care.
                _ => {}
            }
            selected %= selection_len;
        }
    }

    #[allow(clippy::len_zero)]
    fn menu_scoreboard(&mut self) -> io::Result<MenuUpdate> {
        const CAMERA_SIZE: usize = 14;
        const CAMERA_MARGIN: usize = 3;
        let mut cursor_pos = 0usize;
        let mut camera_pos = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "* Scoreboard *")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            let fmt_comparison_stat = |p: &ScoreboardEntry| match p.meta_data.comparison_stat.0 {
                Stat::TimeElapsed(_) => format!("time: {}", fmt_duration(&p.time_elapsed)),
                Stat::PiecesLocked(_) => format!("pieces: {}", p.pieces_locked.iter().sum::<u32>()),
                Stat::LinesCleared(_) => format!("lines: {}", p.lines_cleared),
                Stat::GravityReached(_) => format!("gravity: {}", p.gravity_reached),
                Stat::PointsScored(_) => format!("score: {}", p.points_scored),
            };

            let fmt_past_game = |(e, _): &(ScoreboardEntry, Option<GameRestorationData>)| {
                format!(
                    "{} {} | {}{}",
                    e.meta_data.datetime,
                    e.meta_data.name,
                    fmt_comparison_stat(e),
                    if e.result.is_ok() { "" } else { " (unf.)" }
                )
            };

            match self.scoreboard.sorting {
                ScoreboardSorting::Chronological => self.sort_past_games_chronologically(),
                ScoreboardSorting::Semantic => self.sort_past_games_semantically(),
            };

            for (i, entry) in self
                .scoreboard
                .entries
                .iter()
                .skip(camera_pos)
                .take(CAMERA_SIZE)
                .map(fmt_past_game)
                .enumerate()
            {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:<w_main$}",
                        if cursor_pos == camera_pos + i {
                            format!(">{}", entry)
                        } else {
                            entry
                        }
                    )))?;
            }
            let entries_left = self
                .scoreboard
                .entries
                .len()
                .saturating_sub(camera_pos + CAMERA_SIZE);
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(CAMERA_SIZE).unwrap(),
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!(
                        "{}{}",
                        if entries_left > 0 {
                            format!("... +{entries_left} more ")
                        } else {
                            "".to_owned()
                        },
                        format!("({:?} order [←|→])", self.scoreboard.sorting)
                    )
                )))?;
            self.term.flush()?;

            // Wait for new input.
            match event::read()? {
                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => {
                    break Ok(MenuUpdate::Push(Menu::Quit(
                        "exited with ctrl-c".to_owned(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: kind @ (Press | Repeat),
                    ..
                }) if self.scoreboard.entries.len() > 0 => {
                    // We allow wrapping cursor pos, but only on manual presses (if detectable).
                    if 0 < cursor_pos || kind == Press {
                        // Cursor pos possibly wraps back down.
                        cursor_pos += self.scoreboard.entries.len() - 1;
                        cursor_pos %= self.scoreboard.entries.len();
                        // If it does, then manually reset camera to bottom of scoreboard.
                        if cursor_pos == self.scoreboard.entries.len() - 1 {
                            camera_pos = self.scoreboard.entries.len().saturating_sub(CAMERA_SIZE);
                        // Otherwise cursor just moved normally, and we may have to adapt camera (unless it hit scoreboard end).
                        } else if 0 < camera_pos && cursor_pos < camera_pos + CAMERA_MARGIN {
                            camera_pos -= 1;
                        }
                    }
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: kind @ (Press | Repeat),
                    ..
                }) if self.scoreboard.entries.len() > 0 => {
                    // We allow wrapping cursor pos, but only on manual presses (if detectable).
                    if cursor_pos < self.scoreboard.entries.len() - 1 || kind == Press {
                        // Cursor pos possibly wraps back up.
                        cursor_pos += 1;
                        cursor_pos %= self.scoreboard.entries.len();
                        // If it does, then manually reset camera to bottom of scoreboard.
                        if cursor_pos == 0 {
                            camera_pos = 0;
                        // Otherwise cursor just moved normally, and we may have to adapt camera (unless it hit scoreboard end).
                        } else if camera_pos + CAMERA_SIZE - CAMERA_MARGIN <= cursor_pos
                            && camera_pos
                                < self.scoreboard.entries.len().saturating_sub(CAMERA_SIZE)
                        {
                            camera_pos += 1;
                        }
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    self.scoreboard.sorting = match self.scoreboard.sorting {
                        ScoreboardSorting::Chronological => ScoreboardSorting::Semantic,
                        ScoreboardSorting::Semantic => ScoreboardSorting::Chronological,
                    };
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    self.scoreboard.sorting = match self.scoreboard.sorting {
                        ScoreboardSorting::Chronological => ScoreboardSorting::Semantic,
                        ScoreboardSorting::Semantic => ScoreboardSorting::Chronological,
                    };
                }

                // Delete entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press | Repeat,
                    ..
                }) if self.scoreboard.entries.len() > 0 => {
                    self.scoreboard.entries.remove(cursor_pos);
                    if 0 < cursor_pos && cursor_pos == self.scoreboard.entries.len() {
                        cursor_pos -= 1;
                        camera_pos = camera_pos.saturating_sub(1);
                    }
                }
                // Other event: don't care.
                _ => {}
            };
        }
    }

    fn menu_about(&mut self) -> io::Result<MenuUpdate> {
        /* FIXME: About menu. */
        self.generic_placeholder_menu(
            "About tetrs - Visit https://github.com/Strophox/tetrs",
            vec![],
        )
    }
}

const DAVIS: &str = " ▀█▀ \"I am like Solomon because I built God's temple, an operating system. God said 640x480 16 color graphics but the operating system is 64-bit and multi-cored! Go draw a 16 color elephant. Then, draw a 24-bit elephant in MS Paint and be enlightened. Artist stopped photorealism when the camera was invented. A cartoon is actually better than photorealistic. For the next thousand years, first-person shooters are going to get boring. Tetris looks good.\" - In memory of Terry A. Davis";

pub fn fmt_duration(dur: &Duration) -> String {
    format!(
        "{}min {}.{:02}s",
        dur.as_secs() / 60,
        dur.as_secs() % 60,
        dur.as_millis() % 1000 / 10
    )
}

pub fn fmt_key(key: KeyCode) -> String {
    use crossterm::event::ModifierKeyCode as M;
    use KeyCode as K;
    format!("[{}]", 'String_not_str: {
        match key {
            K::Backspace => "Back",
            //K::Enter => "Enter",
            K::Left => "←",
            K::Right => "→",
            K::Up => "↑",
            K::Down => "↓",
            //K::Home => "Home",
            //K::End => "End",
            //K::Insert => "Insert",
            K::Delete => "Del",
            //K::Menu => "Menu",
            K::PageUp => "PgUp",
            K::PageDown => "PgDn",
            //K::Tab => "Tab",
            //K::CapsLock => "CapsLock",
            K::F(k) => break 'String_not_str format!("F{k}"),
            K::Char(' ') => "Space",
            K::Char(c) => break 'String_not_str c.to_uppercase().to_string(),
            //K::Esc => "Esc",
            K::Modifier(M::LeftShift) => "LShift",
            K::Modifier(M::RightShift) => "RShift",
            K::Modifier(M::LeftControl) => "LCtrl",
            K::Modifier(M::RightControl) => "RCtrl",
            K::Modifier(M::LeftSuper) => "LSuper",
            K::Modifier(M::RightSuper) => "RSuper",
            K::Modifier(M::LeftAlt) => "LAlt",
            K::Modifier(M::RightAlt) => "RAlt",
            K::Modifier(M::IsoLevel3Shift) => "AltGr",
            k => break 'String_not_str format!("{:?}", k),
        }
        .to_string()
    })
}

pub fn fmt_keybinds(button: Button, keybinds: &Keybinds) -> String {
    keybinds
        .iter()
        .filter_map(|(&k, &b)| (b == button).then_some(fmt_key(k)))
        .collect::<Vec<String>>()
        .join("")
}

fn encode_buttons(button_state: &PressedButtons) -> u16 {
    button_state
        .iter()
        .fold(0, |int, b| (int << 1) | u16::from(*b))
}

fn decode_buttons(mut int: u16) -> PressedButtons {
    let mut button_state = PressedButtons::default();
    for i in 0..Button::VARIANTS.len() {
        button_state[Button::VARIANTS.len() - 1 - i] = int & 1 != 0;
        int >>= 1;
    }
    button_state
}

#[allow(dead_code)]
fn encode_board(board: &Board) -> String {
    board
        .iter()
        .map(|line| {
            line.iter()
                .map(|tile| if tile.is_some() { 'X' } else { ' ' })
                .collect::<String>()
        })
        .collect()
}

fn decode_board(string: &str) -> Board {
    let grey_tile = Some(std::num::NonZeroU8::try_from(254).unwrap());
    let mut chars = string.chars();
    std::iter::repeat_n(tetrs_engine::Line::default(), Game::HEIGHT)
        .map(|mut line| {
            for tile in &mut line {
                if let Some(char) = chars.next() {
                    *tile = if char != ' ' { grey_tile } else { None };
                } else {
                    break;
                }
            }
            line
        })
        .collect()
}
