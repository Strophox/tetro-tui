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
    piece_generation::TetrominoSource, piece_rotation::RotationSystem, Button, Config,
    FeedbackMessages, Game, GameBuilder, PressedButtons, Rules, Stat, Tetromino,
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
    game_modifiers,
    game_renderers::{
        cached_renderer::CachedRenderer, color16_palette, empty_palette, fullcolor_palette,
        gruvbox_light_palette, gruvbox_palette, oklch2_palette, tet_str_small, Palette, Renderer,
    },
};

pub type Slots<T> = Vec<(String, T)>;

pub type RecordedUserInput = Vec<(
    tetrs_engine::GameTime,
    u16, /*tetrs_engine::PressedButtons*/
)>;

#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StartGameSettings {
    custom_initial_gravity: u32,
    custom_increase_gravity: bool,
    custom_end_condition: Option<Stat>,
    cheese_linelimit: Option<NonZeroUsize>,
    cheese_gap_size: usize,
    cheese_gravity: u32,
    combo_linelimit: Option<NonZeroUsize>,
    combo_start_layout: u16,
    experimental_mode_unlocked: bool,
    /// Custom starting layout when playing Combo mode (4-wide rows), encoded as binary.
    /// Example: '▀▄▄▀' => 0b_1001_0110 = 150
    custom_start_board: Option<String>, // TODO: Option<Board>,
    // TODO: Placeholder for proper snapshot functionality.
    custom_start_seed: Option<u64>,
    custom_game: Option<SavedGame>,
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

#[serde_with::serde_as]
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphicsSettings {
    pub glyphset: Glyphset,
    palette_active: usize,
    palette_active_lockedtiles: usize,
    pub render_effects: bool,
    game_fps: f64,
    show_fps: bool,
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum ScoreboardSorting {
    Chronological,
    Semantic,
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum SavefileGranularity {
    Nothing,
    Settings,
    SettingsGamedata,
    SettingsGamedataUserinputs,
}

#[serde_with::serde_as]
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    graphics_slots: Slots<GraphicsSettings>,
    graphics_slots_that_should_not_be_changed: usize,
    graphics_active: usize,
    palette_slots: Slots<Palette>,
    palette_slots_that_should_not_be_changed: usize,
    // NOTE: Reconsider #[serde_as(as = "Vec<(_, std::collections::HashMap<serde_with::json::JsonString, _>)>")]
    #[serde_as(as = "Vec<(_, Vec<(_, _)>)>")]
    keybinds_slots: Slots<Keybinds>,
    keybinds_slots_that_should_not_be_changed: usize,
    keybinds_active: usize,
    config_slots: Slots<Config>,
    config_slots_that_should_not_be_changed: usize,
    config_active: usize,
    start_game_settings: StartGameSettings,
    scoreboard_sorting: ScoreboardSorting,
    save_on_exit: SavefileGranularity,
}

impl Default for Settings {
    fn default() -> Self {
        let graphics_slots = vec![
            ("default".to_owned(), GraphicsSettings {
                glyphset: Glyphset::Unicode,
                palette_active: 3,
                palette_active_lockedtiles: 3,
                render_effects: true,
                game_fps: 30.0,
                show_fps: false,
            }),
            (
                "high focus".to_owned(),
                GraphicsSettings {
                    glyphset: Glyphset::Unicode,
                    palette_active: 2,
                    palette_active_lockedtiles: 0,
                    render_effects: false,
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
        let start_game_settings = StartGameSettings {
            custom_initial_gravity: 1,
            custom_increase_gravity: true,
            custom_start_board: None,
            custom_start_seed: None,
            custom_end_condition: None,
            cheese_linelimit: Some(NonZeroUsize::try_from(20).unwrap()),
            cheese_gravity: 0,
            cheese_gap_size: 1,
            combo_linelimit: Some(NonZeroUsize::try_from(20).unwrap()),
            combo_start_layout: game_modifiers::combo_game::LAYOUTS[0],
            experimental_mode_unlocked: false,
            custom_game: None,
        };
        Self {
            graphics_slots_that_should_not_be_changed: graphics_slots.len(),
            graphics_slots,
            graphics_active: 0,
            palette_slots_that_should_not_be_changed: palette_slots.len(),
            palette_slots,
            keybinds_slots_that_should_not_be_changed: keybinds_slots.len(),
            keybinds_slots,
            keybinds_active: 0,
            config_slots_that_should_not_be_changed: config_slots.len(),
            config_slots,
            config_active: 0,
            start_game_settings,
            scoreboard_sorting: ScoreboardSorting::Chronological,
            save_on_exit: SavefileGranularity::Nothing,
        }
    }
}

impl Settings {
    pub fn graphics(&self) -> &GraphicsSettings {
        &self.graphics_slots[self.graphics_active].1
    }
    pub fn keybinds(&self) -> &Keybinds {
        &self.keybinds_slots[self.keybinds_active].1
    }
    pub fn config(&self) -> &Config {
        &self.config_slots[self.config_active].1
    }
    fn graphics_mut(&mut self) -> &mut GraphicsSettings {
        &mut self.graphics_slots[self.graphics_active].1
    }
    fn keybinds_mut(&mut self) -> &mut Keybinds {
        &mut self.keybinds_slots[self.keybinds_active].1
    }
    fn config_mut(&mut self) -> &mut Config {
        &mut self.config_slots[self.config_active].1
    }

    pub fn palette(&self) -> &Palette {
        &self.palette_slots[self.graphics().palette_active].1
    }
    pub fn palette_lockedtiles(&self) -> &Palette {
        &self.palette_slots[self.graphics().palette_active_lockedtiles].1
    }
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameMetaData {
    pub datetime: String,
    pub name: String,
    pub comparison_stat: Stat,
    pub recorded_user_input: RecordedUserInput,
}

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedGame {
    blueprint: tetrs_engine::GameBuilder,
    modifier_identifiers: Vec<String>,
    result: tetrs_engine::GameResult,
    time_elapsed: tetrs_engine::GameTime,
    pieces_locked: [u32; Tetromino::VARIANTS.len()],
    lines_cleared: usize,
    gravity_reached: u32,
    points_scored: u64,
    meta_data: GameMetaData,
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
        total_duration_paused: Duration,
        game_renderer: Box<CachedRenderer>,
    },
    GameOver(Box<SavedGame>),
    GameComplete(Box<SavedGame>),
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
            Menu::Game { .. } => "Game", //&format!("Game {}", game.mode().name.as_ref().map_or("".to_owned(), |ref name| format!("({name})"))),
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

#[derive(Clone, Debug)]
pub struct Application<T: Write> {
    pub term: T,
    settings: Settings,
    past_games: Vec<SavedGame>,
    kitty_detected: bool,
    kitty_assumed: bool,
    combo_bot_enabled: bool,
}

impl<T: Write> Drop for Application<T> {
    fn drop(&mut self) {
        // FIXME: Handle errors?
        let savefile_path = Self::savefile_path();
        // If the user wants their data stored, try to do so.
        if self.settings.save_on_exit != SavefileGranularity::Nothing {
            if let Err(_e) = self.store_save(savefile_path) {
                // FIXME: Make this debuggable.
                //eprintln!("Could not save settings this time: {e} ");
                //std::thread::sleep(Duration::from_secs(4));
            }
        // Otherwise check if savefile exists.
        } else if let Ok(exists) = savefile_path.try_exists() {
            // Delete it for them if it does.
            if exists {
                let _ = std::fs::remove_file(savefile_path);
            }
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
            term,
            settings: Settings::default(),
            past_games: Vec::default(),
            kitty_detected: false,
            kitty_assumed: false,
            combo_bot_enabled: false,
        };

        // Actually load in settings.
        if app.load_save(Self::savefile_path()).is_err() {
            // FIXME: Make this debuggable.
            //eprintln!("Could not loading settings: {e}");
            //std::thread::sleep(Duration::from_secs(5));
        }

        // Now that the settings are loaded, we handle custom flags set for this session.
        if custom_start_board.is_some() {
            app.settings.start_game_settings.custom_start_board = custom_start_board;
        }
        if custom_start_seed.is_some() {
            app.settings.start_game_settings.custom_start_seed = custom_start_seed;
        }
        app.combo_bot_enabled = combo_bot_enabled;
        app.kitty_detected = terminal::supports_keyboard_enhancement().unwrap_or(false);
        app.kitty_assumed = app.kitty_detected;
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

    fn store_save(&mut self, path: PathBuf) -> io::Result<()> {
        if self.settings.save_on_exit < SavefileGranularity::SettingsGamedata {
            // Clear past games if no game data is wished to be stored.
            self.past_games.clear();
        }
        if self.settings.save_on_exit < SavefileGranularity::SettingsGamedataUserinputs {
            // Clear past game inputs if no game input data is wished to be stored.
            for past_game in &mut self.past_games {
                past_game.meta_data.recorded_user_input.clear();
            }
        }
        let save_state = (&self.settings, &self.past_games);
        let save_str = serde_json::to_string(&save_state)?;
        let mut file = File::create(path)?;
        // FIXME: Handle error?
        let _ = file.write(save_str.as_bytes())?;
        Ok(())
    }

    fn load_save(&mut self, path: PathBuf) -> io::Result<()> {
        let mut file = File::open(path)?;
        let mut save_str = String::new();
        file.read_to_string(&mut save_str)?;
        let save_state = serde_json::from_str(&save_str)?;
        (self.settings, self.past_games) = save_state;
        Ok(())
    }

    fn log_game_as_past(&mut self, game: &Game, meta_data: &GameMetaData) -> SavedGame {
        let past_game = transcribe(game, meta_data);
        self.past_games.push(past_game.clone());
        past_game
    }

    fn sort_past_games_chronologically(&mut self) {
        self.past_games.sort_by(|pg1, pg2| {
            pg1.meta_data
                .datetime
                .cmp(&pg2.meta_data.datetime)
                .reverse()
        });
    }

    #[rustfmt::skip]
    fn sort_past_games_semantically(&mut self) {
        self.past_games.sort_by(|pg1, pg2|
            // Sort by gamemode (name).
            pg1.meta_data.name.cmp(&pg2.meta_data.name).then_with(||
            // Sort by if gamemode was finished successfully.
            pg1.result.is_ok().cmp(&pg2.result.is_ok()).then_with(||
            // Stop here if game didn't admit an end condition...
            if pg1.blueprint.rules.as_ref().unwrap().end_conditions.is_empty() {
                // Sort by score here for convenience, and because we currently only show score for custom games by default.
                pg1.points_scored.cmp(&pg2.points_scored)
            } else {
                // Sort by comparison stat...
                let o = match pg1.meta_data.comparison_stat {
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
                if pg1.blueprint.rules.as_ref().unwrap().end_conditions[0].0
                        < pg1.meta_data.comparison_stat
                    { o } else { o.reverse() }
            }
            )
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
                    total_duration_paused,
                    last_paused,
                    game_renderer,
                } => self.menu_game(
                    game,
                    meta_data,
                    time_started,
                    last_paused,
                    total_duration_paused,
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
                &str,
                Stat,
                String,
                Box<dyn Fn(&GameBuilder) -> Game>,
            )> = vec![
                (
                    "40-Lines",
                    Stat::TimeElapsed(Duration::ZERO),
                    "How fast can you clear forty lines?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::forty_lines()).build()
                    }),
                ),
                (
                    "Marathon",
                    Stat::PointsScored(0),
                    "Can you make it to level 15?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::marathon()).build()
                    }),
                ),
                (
                    "Time Trial",
                    Stat::PointsScored(0),
                    "What highscore can you get in 3 minutes?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::time_trial()).build()
                    }),
                ),
                (
                    "Master",
                    Stat::PointsScored(0),
                    "Can you clear 15 levels at instant gravity?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::master()).build()
                    }),
                ),
                (
                    "Puzzle",
                    Stat::TimeElapsed(Duration::ZERO),
                    "Get perfect clears in all 24 puzzle levels.".to_owned(),
                    Box::new(game_modifiers::puzzle_game::build_puzzle),
                ),
                (
                    "Cheese",
                    Stat::PiecesLocked(0),
                    format!(
                        "Eat through lines like Swiss cheese. Limit: {:?}",
                        self.settings.start_game_settings.cheese_linelimit
                    ),
                    Box::new({
                        let cheese_limit = self.settings.start_game_settings.cheese_linelimit;
                        let cheese_gap_size = self.settings.start_game_settings.cheese_gap_size;
                        let cheese_gravity = self.settings.start_game_settings.cheese_gravity;
                        move |builder: &GameBuilder| {
                            game_modifiers::cheese_game::build_cheese(
                                builder,
                                cheese_limit,
                                cheese_gap_size,
                                cheese_gravity,
                            )
                        }
                    }),
                ),
                (
                    "Combo",
                    Stat::TimeElapsed(Duration::ZERO),
                    format!(
                        "Get consecutive line clears. Limit: {:?}{}",
                        self.settings.start_game_settings.combo_linelimit,
                        if self.settings.start_game_settings.combo_start_layout
                            != crate::game_modifiers::combo_game::LAYOUTS[0]
                        {
                            format!(
                                ", Layout={:b}",
                                self.settings.start_game_settings.combo_start_layout
                            )
                        } else {
                            "".to_owned()
                        }
                    ),
                    Box::new({
                        let combo_start_layout =
                            self.settings.start_game_settings.combo_start_layout;
                        move |builder: &GameBuilder| {
                            game_modifiers::combo_game::build_combo(builder, 1, combo_start_layout)
                        }
                    }),
                ),
            ];
            if self.settings.start_game_settings.experimental_mode_unlocked {
                game_presets.insert(
                    5,
                    (
                        "Descent (experimental)",
                        Stat::PointsScored(0),
                        "Spin the piece and collect 'gems' by touching them.".to_owned(),
                        Box::new(game_modifiers::descent_game::build_descent),
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
                        } else if self
                            .settings
                            .start_game_settings
                            .custom_start_seed
                            .is_some()
                            || self
                                .settings
                                .start_game_settings
                                .custom_start_board
                                .is_some()
                        {
                            ">> Custom: (clear board/seed with [Del])"
                        } else {
                            ">> Custom: [→]                          "
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
                        self.settings.start_game_settings.custom_initial_gravity
                    ),
                    format!(
                        "| Auto-increase gravity: {}",
                        self.settings.start_game_settings.custom_increase_gravity
                    ),
                    format!(
                        "| Limit: {:?} [→]",
                        self.settings.start_game_settings.custom_end_condition
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
                                    || self
                                        .settings
                                        .start_game_settings
                                        .custom_end_condition
                                        .is_some()
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
                    let builder = Game::builder().config(self.settings.config().clone());
                    let (name, stat, game) = if selected < game_presets.len() {
                        let (name, stat, _desc, func) = &game_presets[selected];
                        (*name, stat, func(&builder))
                    } else {
                        let end_conditions =
                            match self.settings.start_game_settings.custom_end_condition {
                                Some(c) => vec![(c, true)],
                                None => vec![],
                            };
                        let rules = Rules {
                            initial_gravity: self
                                .settings
                                .start_game_settings
                                .custom_initial_gravity,
                            increase_gravity: self
                                .settings
                                .start_game_settings
                                .custom_increase_gravity,
                            end_conditions,
                        };
                        let mut custom_game_builder = Game::builder();
                        let _ = custom_game_builder.rules.insert(rules);
                        let _ = custom_game_builder
                            .config
                            .insert(self.settings.config().clone());
                        if let Some(seed) = self.settings.start_game_settings.custom_start_seed {
                            let _ = custom_game_builder.seed.insert(seed);
                        }
                        let custom_game = if let Some(ref custom_start_board_str) =
                            self.settings.start_game_settings.custom_start_board
                        {
                            custom_game_builder.build_modified([
                                game_modifiers::utils::custom_start_board(custom_start_board_str),
                            ])
                        } else {
                            custom_game_builder.build()
                        };
                        ("Custom", &Stat::PointsScored(0), custom_game)
                    };
                    let now = Instant::now();
                    break Ok(MenuUpdate::Push(Menu::Game {
                        game: Box::new(game),
                        meta_data: GameMetaData {
                            datetime: chrono::Utc::now().format("%Y-%m-%d_%H:%M").to_string(),
                            name: name.to_owned(),
                            comparison_stat: *stat,
                            recorded_user_input: RecordedUserInput::new(),
                        },
                        time_started: now,
                        last_paused: now,
                        total_duration_paused: Duration::ZERO,
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
                                self.settings.start_game_settings.custom_initial_gravity = self
                                    .settings
                                    .start_game_settings
                                    .custom_initial_gravity
                                    .saturating_add(d_gravity);
                            }
                            2 => {
                                self.settings.start_game_settings.custom_increase_gravity =
                                    !self.settings.start_game_settings.custom_increase_gravity;
                            }
                            3 => {
                                match self.settings.start_game_settings.custom_end_condition {
                                    Some(Stat::TimeElapsed(ref mut dur)) => {
                                        *dur += d_time;
                                    }
                                    Some(Stat::PiecesLocked(ref mut pcs)) => {
                                        *pcs += d_pieces;
                                    }
                                    Some(Stat::LinesCleared(ref mut lns)) => {
                                        *lns += d_lines;
                                    }
                                    Some(Stat::GravityReached(ref mut lvl)) => {
                                        *lvl += d_gravity;
                                    }
                                    Some(Stat::PointsScored(ref mut pts)) => {
                                        *pts += d_score;
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
                                self.settings.start_game_settings.custom_initial_gravity = self
                                    .settings
                                    .start_game_settings
                                    .custom_initial_gravity
                                    .saturating_sub(d_gravity);
                            }
                            2 => {
                                self.settings.start_game_settings.custom_increase_gravity =
                                    !self.settings.start_game_settings.custom_increase_gravity;
                            }
                            3 => {
                                match self.settings.start_game_settings.custom_end_condition {
                                    Some(Stat::TimeElapsed(ref mut t)) => {
                                        *t = t.saturating_sub(d_time);
                                    }
                                    Some(Stat::PiecesLocked(ref mut p)) => {
                                        *p = p.saturating_sub(d_pieces);
                                    }
                                    Some(Stat::LinesCleared(ref mut l)) => {
                                        *l = l.saturating_sub(d_lines);
                                    }
                                    Some(Stat::GravityReached(ref mut g)) => {
                                        *g = g.saturating_sub(d_gravity);
                                    }
                                    Some(Stat::PointsScored(ref mut s)) => {
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
                        if let Some(limit) = self.settings.start_game_settings.combo_linelimit {
                            self.settings.start_game_settings.combo_linelimit =
                                NonZeroUsize::try_from(limit.get() - 1).ok();
                        }
                    } else if selected == selection_size - 3 {
                        if let Some(limit) = self.settings.start_game_settings.cheese_linelimit {
                            self.settings.start_game_settings.cheese_linelimit =
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
                            self.settings.start_game_settings.custom_end_condition =
                                match self.settings.start_game_settings.custom_end_condition {
                                    Some(Stat::TimeElapsed(_)) => Some(Stat::PointsScored(9000)),
                                    Some(Stat::PointsScored(_)) => Some(Stat::PiecesLocked(100)),
                                    Some(Stat::PiecesLocked(_)) => Some(Stat::LinesCleared(40)),
                                    Some(Stat::LinesCleared(_)) => Some(Stat::GravityReached(20)),
                                    Some(Stat::GravityReached(_)) => None,
                                    None => Some(Stat::TimeElapsed(Duration::from_secs(180))),
                                };
                        } else {
                            customization_selected += 1
                        }
                    } else if selected == selection_size - 2 {
                        self.settings.start_game_settings.combo_linelimit = if let Some(limit) =
                            self.settings.start_game_settings.combo_linelimit
                        {
                            limit.checked_add(1)
                        } else {
                            Some(NonZeroUsize::MIN)
                        };
                    } else if selected == selection_size - 3 {
                        self.settings.start_game_settings.cheese_linelimit = if let Some(limit) =
                            self.settings.start_game_settings.cheese_linelimit
                        {
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
                    // If custom gamemode selected, allow deleting custom start board and seed.
                    if selected == selection_size - 1 {
                        self.settings.start_game_settings.custom_start_seed = None;
                        self.settings.start_game_settings.custom_start_board = None;
                    } else if selected == selection_size - 2 {
                        let new_layout_idx = if let Some(i) =
                            crate::game_modifiers::combo_game::LAYOUTS
                                .iter()
                                .position(|lay| {
                                    *lay == self.settings.start_game_settings.combo_start_layout
                                }) {
                            let layout_cnt = crate::game_modifiers::combo_game::LAYOUTS.len();
                            (i + 1) % layout_cnt
                        } else {
                            0
                        };
                        self.settings.start_game_settings.combo_start_layout =
                            crate::game_modifiers::combo_game::LAYOUTS[new_layout_idx];
                    }
                }
                // Other event: don't care.
                _ => {}
            }
        }
    }

    fn menu_game(
        &mut self,
        game: &mut Game,
        meta_data: &mut GameMetaData,
        time_started: &Instant,
        time_last_paused: &mut Instant,
        total_pause_duration: &mut Duration,
        game_renderer: &mut impl Renderer,
    ) -> io::Result<MenuUpdate> {
        if self.kitty_assumed {
            // FIXME: Kinda iffy. Do we need all flags? What undesirable effects might there be?
            let _ = self.term.execute(event::PushKeyboardEnhancementFlags(
                event::KeyboardEnhancementFlags::all(),
                // event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
            ));
        }
        // Prepare channel with which to communicate `Button` inputs / game interrupt.
        let mut buttons_pressed = tetrs_engine::PressedButtons::default();
        let (button_sender, button_receiver) = mpsc::channel();
        let _input_handler =
            TerminalInputHandler::new(&button_sender, self.settings.keybinds(), self.kitty_assumed);
        let mut combo_bot_handler = (self.combo_bot_enabled && meta_data.name == "Combo")
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
            if game.ended() {
                let past_game = self.log_game_as_past(game, meta_data);
                let menu = if past_game.result.is_ok() {
                    Menu::GameComplete
                } else {
                    Menu::GameOver
                }(Box::new(past_game));
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
                        let past_game = self.log_game_as_past(game, meta_data);
                        break 'render MenuUpdate::Push(Menu::GameOver(Box::new(past_game)));
                    }
                    Ok(InputSignal::Pause) => {
                        *time_last_paused = Instant::now();
                        break 'render MenuUpdate::Push(Menu::Pause);
                    }
                    Ok(InputSignal::WindowResize) => {
                        clean_screen = true;
                        continue 'frame_idle;
                    }
                    Ok(InputSignal::TakeSnapshot) => {
                        self.settings.start_game_settings.custom_start_board = Some(
                            String::from_iter(game.state().board.iter().rev().flat_map(|line| {
                                line.iter()
                                    .map(|cell| if cell.is_some() { 'X' } else { ' ' })
                            })),
                        );
                        self.settings.start_game_settings.custom_start_seed = Some(game.seed());
                        new_feedback_msgs.push((
                            game.state().time,
                            tetrs_engine::Feedback::Text("(Snapshot taken!)".to_owned()),
                        ));
                    }
                    Ok(InputSignal::ButtonInput(button, button_state, instant)) => {
                        buttons_pressed[button] = button_state;
                        let game_time_userinput = instant.saturating_duration_since(*time_started)
                            - *total_pause_duration;
                        let game_now = std::cmp::max(game_time_userinput, game.state().time);
                        // FIXME: Handle/ensure no Err.
                        meta_data
                            .recorded_user_input
                            .push((game_now, compress_buttons(&buttons_pressed)));
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
        if self.kitty_assumed {
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

    fn menu_game_ended(&mut self, past_game: &SavedGame) -> io::Result<MenuUpdate> {
        let SavedGame {
            blueprint: _,
            modifier_identifiers: _,
            result,
            time_elapsed,
            pieces_locked,
            lines_cleared,
            gravity_reached,
            points_scored,
            meta_data,
        } = past_game;
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::Quit("quit after game ended".to_owned()),
        ];
        // if gamemode.name.as_ref().map(String::as_str) == Some("Puzzle")
        if result.is_ok() && meta_data.name == "Puzzle" {
            self.settings.start_game_settings.experimental_mode_unlocked = true;
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
                    format!("Points scored: {points_scored}")
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 4))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Gravity reached: {gravity_reached}",)
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 5))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Lines cleared: {}", lines_cleared)
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 6))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Pieces locked: {}", pieces_locked.iter().sum::<u32>())
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
                    match self.settings.save_on_exit {
                        SavefileGranularity::Nothing => "OFF*",
                        SavefileGranularity::Settings => "ON (save settings)",
                        SavefileGranularity::SettingsGamedata => "ON (save settings, game stats)",
                        SavefileGranularity::SettingsGamedataUserinputs =>
                            "ON (save settings, game stats & inputs)",
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
                        if self.settings.save_on_exit == SavefileGranularity::Nothing {
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
                        self.settings.save_on_exit =
                            SavefileGranularity::SettingsGamedataUserinputs;
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
                        self.settings.save_on_exit = match self.settings.save_on_exit {
                            SavefileGranularity::Nothing => {
                                SavefileGranularity::SettingsGamedataUserinputs
                            }
                            SavefileGranularity::SettingsGamedataUserinputs => {
                                SavefileGranularity::SettingsGamedata
                            }
                            SavefileGranularity::SettingsGamedata => SavefileGranularity::Settings,
                            SavefileGranularity::Settings => SavefileGranularity::Nothing,
                        };
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 3 {
                        self.settings.save_on_exit = match self.settings.save_on_exit {
                            SavefileGranularity::Nothing => SavefileGranularity::Settings,
                            SavefileGranularity::Settings => SavefileGranularity::SettingsGamedata,
                            SavefileGranularity::SettingsGamedata => {
                                SavefileGranularity::SettingsGamedataUserinputs
                            }
                            SavefileGranularity::SettingsGamedataUserinputs => {
                                SavefileGranularity::Nothing
                            }
                        };
                    }
                }

                // Set save_on_exit to false.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press,
                    ..
                }) => {
                    if selected == 3 {
                        self.settings.save_on_exit = SavefileGranularity::Nothing;
                    }
                }

                // Other event: Just ignore.
                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }

    fn menu_adjust_keybinds(&mut self) -> io::Result<MenuUpdate> {
        // "Trying to modify a default slot: create copy of slot to allow safely modifying that."
        let if_slot_is_default_then_copy_and_switch = |settings: &mut Settings| {
            if settings.keybinds_active < settings.keybinds_slots_that_should_not_be_changed {
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
                settings.keybinds_active = settings.keybinds_slots.len() - 1;
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
                self.settings.keybinds_active + 1,
                self.settings.keybinds_slots.len(),
                self.settings.keybinds_slots[self.settings.keybinds_active].0,
                if self.settings.keybinds_slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.keybinds_active
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
                        if self.kitty_assumed {
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
                        if self.kitty_assumed {
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
                        if self.settings.keybinds_active
                            >= self.settings.keybinds_slots_that_should_not_be_changed
                        {
                            self.settings
                                .keybinds_slots
                                .remove(self.settings.keybinds_active);
                            self.settings.keybinds_active = 0;
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
                        self.settings.keybinds_active += 1;
                        self.settings.keybinds_active %= self.settings.keybinds_slots.len();
                    }
                }

                // Cycle slot to right.
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 0 {
                        self.settings.keybinds_active += self.settings.keybinds_slots.len() - 1;
                        self.settings.keybinds_active %= self.settings.keybinds_slots.len();
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
            if settings.config_active < settings.config_slots_that_should_not_be_changed {
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
                settings.config_active = settings.config_slots.len() - 1;
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
                self.settings.config_active + 1,
                self.settings.config_slots.len(),
                self.settings.config_slots[self.settings.config_active].0,
                if self.settings.config_slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.config_active
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
                    self.kitty_assumed
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
                        if self.kitty_detected {
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
                // Select.
                // Event::Key(KeyEvent {
                //     code: KeyCode::Enter | KeyCode::Char('e'),
                //     kind: Press,
                //     ..
                // }) => {
                //     if selected == selection_len - 1 {
                //         *self.settings.config_mut() = GameConfig::default();
                //         self.kitty_assumed = self.kitty_detected;
                //     }
                // }

                // Reset config, or delete entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press,
                    ..
                }) => {
                    if selected == 0 {
                        // If a custom slot, then remove it (and return to the 'default' 0th slot).
                        if self.settings.config_active
                            >= self.settings.config_slots_that_should_not_be_changed
                        {
                            self.settings
                                .config_slots
                                .remove(self.settings.config_active);
                            self.settings.config_active = 0;
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
                        self.settings.config_active += 1;
                        self.settings.config_active %= self.settings.config_slots.len();
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
                        self.kitty_assumed = !self.kitty_assumed;
                    }
                    _ => {}
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.config_active += self.settings.config_slots.len() - 1;
                        self.settings.config_active %= self.settings.config_slots.len();
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
                        self.kitty_assumed = !self.kitty_assumed;
                    }
                    _ => {}
                },
                // Other event: don't care.
                _ => {}
            }
            selected %= selection_len;
        }
    }

    fn menu_adjust_graphics(&mut self) -> io::Result<MenuUpdate> {
        let if_slot_is_default_then_copy_and_switch = |settings: &mut Settings| {
            if settings.graphics_active < settings.graphics_slots_that_should_not_be_changed {
                let mut n = 1;
                let new_custom_slot_name = loop {
                    let name = format!("custom_{n}");
                    if settings.graphics_slots.iter().any(|s| s.0 == name) {
                        n += 1;
                    } else {
                        break name;
                    }
                };
                let new_slot = (new_custom_slot_name, settings.graphics().clone());
                settings.graphics_slots.push(new_slot);
                settings.graphics_active = settings.graphics_slots.len() - 1;
            }
        };
        let selection_len = 7;
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
                self.settings.graphics_active + 1,
                self.settings.graphics_slots.len(),
                self.settings.graphics_slots[self.settings.graphics_active].0,
                if self.settings.graphics_slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.graphics_active
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
                        self.settings.graphics_active += 1;
                        self.settings.graphics_active %= self.settings.graphics_slots.len();
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
                        self.settings.graphics_mut().game_fps += 1.0;
                    }
                    6 => {
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
                        self.settings.graphics_active += self.settings.graphics_slots.len() - 1;
                        self.settings.graphics_active %= self.settings.graphics_slots.len();
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
                        if self.settings.graphics().game_fps >= 1.0 {
                            self.settings.graphics_mut().game_fps -= 1.0;
                        }
                    }
                    6 => {
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
                        if self.settings.graphics_active
                            >= self.settings.graphics_slots_that_should_not_be_changed
                        {
                            self.settings
                                .graphics_slots
                                .remove(self.settings.graphics_active);
                            self.settings.graphics_active = 0;
                        }
                    }
                }

                // Other event: Just ignore.
                _ => {}
            }
            selected %= selection_len;
        }
    }

    #[allow(clippy::len_zero)]
    fn menu_scoreboard(&mut self) -> io::Result<MenuUpdate> {
        const CAMERA_SIZE: usize = 14;
        const CAMERA_MARGIN: usize = 4;
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

            let fmt_comparison_stat = |p: &SavedGame| match p.meta_data.comparison_stat {
                Stat::TimeElapsed(_) => format!("time: {}", fmt_duration(&p.time_elapsed)),
                Stat::PiecesLocked(_) => format!("pieces: {}", p.pieces_locked.iter().sum::<u32>()),
                Stat::LinesCleared(_) => format!("lines: {}", p.lines_cleared),
                Stat::GravityReached(_) => format!("gravity: {}", p.gravity_reached),
                Stat::PointsScored(_) => format!("points: {}", p.points_scored),
            };

            let fmt_past_game = |p: &SavedGame| {
                format!(
                    "{} {} | {}{}",
                    p.meta_data.datetime,
                    p.meta_data.name,
                    fmt_comparison_stat(p),
                    if p.result.is_ok() { "" } else { " (unf.)" }
                )
            };

            match self.settings.scoreboard_sorting {
                ScoreboardSorting::Chronological => self.sort_past_games_chronologically(),
                ScoreboardSorting::Semantic => self.sort_past_games_semantically(),
            };

            for (i, entry) in self
                .past_games
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
                .past_games
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
                        format!("({:?} order [←|→])", self.settings.scoreboard_sorting)
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
                    kind: Press | Repeat,
                    ..
                }) if self.past_games.len() > 0 => {
                    // yo what the hell
                    cursor_pos += self.past_games.len() - 1;
                    cursor_pos %= self.past_games.len();
                    if cursor_pos == self.past_games.len() - 1 {
                        camera_pos = self.past_games.len().saturating_sub(CAMERA_SIZE);
                    } else if 0 < camera_pos && cursor_pos < camera_pos + CAMERA_MARGIN {
                        camera_pos -= 1;
                    }
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) if self.past_games.len() > 0 => {
                    // yo what the hell pt.2
                    cursor_pos += 1;
                    cursor_pos %= self.past_games.len();
                    if cursor_pos == 0 {
                        camera_pos = 0;
                    } else if camera_pos + CAMERA_SIZE - CAMERA_MARGIN < cursor_pos
                        && camera_pos < self.past_games.len().saturating_sub(CAMERA_SIZE)
                    {
                        camera_pos += 1;
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    self.settings.scoreboard_sorting = match self.settings.scoreboard_sorting {
                        ScoreboardSorting::Chronological => ScoreboardSorting::Semantic,
                        ScoreboardSorting::Semantic => ScoreboardSorting::Chronological,
                    };
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    self.settings.scoreboard_sorting = match self.settings.scoreboard_sorting {
                        ScoreboardSorting::Chronological => ScoreboardSorting::Semantic,
                        ScoreboardSorting::Semantic => ScoreboardSorting::Chronological,
                    };
                }

                // Delete entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press,
                    ..
                }) if self.past_games.len() > 0 => {
                    self.past_games.remove(cursor_pos);
                    if 0 < cursor_pos && cursor_pos == self.past_games.len() {
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

fn compress_buttons(button_state: &PressedButtons) -> u16 {
    button_state
        .iter()
        .fold(0, |int, b| (int << 1) | u16::from(*b))
}

// TODO: Use this to enable game replay.
#[allow(dead_code)]
fn decompress_buttons(mut int: u16) -> PressedButtons {
    let mut button_state = PressedButtons::default();
    for i in 0..Button::VARIANTS.len() {
        button_state[Button::VARIANTS.len() - 1 - i] = int & 1 != 0;
        int >>= 1;
    }
    button_state
}

fn transcribe(game: &Game, meta_data: &GameMetaData) -> SavedGame {
    SavedGame {
        meta_data: meta_data.clone(),
        result: game.state().result.unwrap(),
        time_elapsed: game.state().time,
        pieces_locked: game.state().pieces_locked,
        lines_cleared: game.state().lines_cleared,
        gravity_reached: game.state().gravity,
        points_scored: game.state().score,
        blueprint: game.blueprint(),
        modifier_identifiers: game.modifier_names().map(str::to_owned).collect(),
    }
}
