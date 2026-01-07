use std::{
    collections::HashMap,
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
    piece_generation::TetrominoSource, piece_rotation::RotationSystem, Button, ButtonsPressed,
    FeedbackMessages, Game, GameConfig, GameMode, GameState, Limits, Tetromino,
};

use crate::{
    game_input_handlers::{combo_bot::ComboBotHandler, crossterm::CrosstermHandler, InputSignal},
    game_mods,
    game_renderers::{self, cached_renderer::CachedRenderer, tet_str_small, Renderer},
};

// NOTE: This could be more general and less ad-hoc. Count number of I-Spins, J-Spins, etc..
pub type RunningGameStats = ([u32; 5], Vec<u32>);

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FinishedGameStats {
    timestamp: String,
    actions: [u32; 5],
    score_bonuses: Vec<u32>,
    gamemode: GameMode,
    last_state: GameState,
}

impl FinishedGameStats {
    fn was_successful(&self) -> bool {
        self.last_state.end.is_some_and(|fin| fin.is_ok())
    }
}

#[derive(Debug)]
enum Menu {
    Title,
    NewGame,
    Game {
        game: Box<Game>,
        time_started: Instant,
        last_paused: Instant,
        total_duration_paused: Duration,
        running_game_stats: RunningGameStats,
        game_renderer: Box<CachedRenderer>,
    },
    GameOver(Box<FinishedGameStats>),
    GameComplete(Box<FinishedGameStats>),
    Pause,
    Settings,
    ConfigureKeybinds,
    ConfigureGameplay,
    ConfigureGraphics,
    Scores,
    About,
    Quit(String),
}

impl std::fmt::Display for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Menu::Title => "Title Screen",
            Menu::NewGame => "New Game",
            Menu::Game { .. } => "Game", //&format!("Game {}", game.mode().name.as_ref().map_or("".to_string(), |ref name| format!("({name})"))),
            Menu::GameOver(_) => "Game Over",
            Menu::GameComplete(_) => "Game Completed",
            Menu::Pause => "Pause",
            Menu::Settings => "Settings",
            Menu::ConfigureKeybinds => "Configure Keybinds",
            Menu::ConfigureGameplay => "Configure Gameplay",
            Menu::ConfigureGraphics => "Configure Graphics",
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

// For the "New Game" menu.
#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum Stat {
    Time(Duration),
    Pieces(u32),
    Lines(usize),
    Gravity(u32),
    Score(u64),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NewGameSettings {
    initial_gravity: u32,
    increase_gravity: bool,
    custom_mode_limit: Option<Stat>,
    cheese_mode_limit: Option<NonZeroUsize>,
    cheese_mode_gap_size: usize,
    cheese_mode_gravity: u32,
    combo_start_layout: u16,
    descent_mode: bool,
    custom_start_board: Option<String>,
    // TODO: Placeholder for proper snapshot functionality.
    custom_start_seed: Option<u64>,
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum GraphicsGlyphset {
    Electronika60,
    #[allow(clippy::upper_case_acronyms)]
    ASCII,
    Unicode,
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum GraphicsColoring {
    Monochrome,
    Color16,
    Fullcolor,
    Experimental,
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum SavefileGranularity {
    Nothing,
    Settings,
    SettingsAndGames,
}

#[serde_with::serde_as]
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub new_game: NewGameSettings,
    pub game_config: GameConfig,
    #[serde_as(as = "HashMap<serde_with::json::JsonString, _>")]
    pub keybinds: HashMap<KeyCode, Button>,
    pub game_fps: f64,
    pub show_fps: bool,
    pub graphics_glyphset: GraphicsGlyphset,
    pub graphics_coloring: GraphicsColoring,
    pub graphics_coloring_locked: GraphicsColoring,
    pub save_on_exit: SavefileGranularity,
}

#[derive(Clone, Debug)]
pub struct Application<T: Write> {
    pub term: T,
    settings: Settings,
    past_games: Vec<FinishedGameStats>,
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
        mut terminal: T,
        custom_start_seed: Option<u64>,
        custom_start_board: Option<String>,
        combo_start_layout: Option<u16>,
        combo_bot_enabled: bool,
    ) -> Self {
        // Console prologue: Initialization.
        // FIXME: Handle errors?
        let _ = terminal.execute(terminal::EnterAlternateScreen);
        let _ = terminal.execute(terminal::SetTitle("tetrs - Terminal User Interface"));
        let _ = terminal.execute(cursor::Hide);
        let _ = terminal::enable_raw_mode();
        let kitty_detected = terminal::supports_keyboard_enhancement().unwrap_or(false);
        let mut app = Self {
            term: terminal,
            settings: Settings {
                new_game: NewGameSettings {
                    initial_gravity: 1,
                    increase_gravity: false,
                    custom_mode_limit: None,
                    cheese_mode_limit: Some(NonZeroUsize::try_from(20).unwrap()),
                    cheese_mode_gap_size: 1,
                    cheese_mode_gravity: 0,
                    combo_start_layout: game_mods::combo_mode::LAYOUTS[0],
                    descent_mode: false,
                    custom_start_board: None,
                    custom_start_seed: None,
                },
                game_config: GameConfig::default(),
                keybinds: CrosstermHandler::default_keybinds(),
                game_fps: 30.0,
                show_fps: false,
                graphics_glyphset: GraphicsGlyphset::Unicode,
                graphics_coloring: GraphicsColoring::Fullcolor,
                graphics_coloring_locked: GraphicsColoring::Fullcolor,
                save_on_exit: SavefileGranularity::Nothing,
            },
            past_games: vec![],
            kitty_detected,
            kitty_assumed: kitty_detected,
            combo_bot_enabled: false,
        };
        if app.load_save(Self::savefile_path()).is_err() {
            // FIXME: Make this debuggable.
            //eprintln!("Could not loading settings: {e}");
            //std::thread::sleep(Duration::from_secs(5));
        }
        if let Some(combo_start_layout) = combo_start_layout {
            app.settings.new_game.combo_start_layout = combo_start_layout;
        }
        if custom_start_board.is_some() {
            app.settings.new_game.custom_start_board = custom_start_board;
        }
        if let Some(custom_start_seed) = custom_start_seed {
            app.settings.new_game
                .custom_start_seed = Some(custom_start_seed);
        }
        app.combo_bot_enabled = combo_bot_enabled;
        app.settings.game_config.no_soft_drop_lock = !kitty_detected;
        app
    }

    fn savefile_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(Self::SAVEFILE_NAME)
    }

    fn store_save(&mut self, path: PathBuf) -> io::Result<()> {
        // Only save past games if needed.
        self.past_games = if self.settings.save_on_exit == SavefileGranularity::SettingsAndGames {
            self
                .past_games
                .iter()
                .filter(|finished_game_stats| {
                    finished_game_stats.was_successful()
                        || finished_game_stats.last_state.lines_cleared > 0
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let save_state = (
            &self.settings,
            &self.past_games,
        );
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
        (
            self.settings,
            self.past_games,
        ) = save_state;
        Ok(())
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
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
                Menu::Title => self.title_menu(),
                Menu::NewGame => self.new_game_menu(),
                Menu::Game {
                    game,
                    time_started,
                    total_duration_paused,
                    last_paused,
                    running_game_stats,
                    game_renderer,
                } => self.game_menu(
                    game,
                    time_started,
                    last_paused,
                    total_duration_paused,
                    running_game_stats,
                    game_renderer.as_mut(),
                ),
                Menu::Pause => self.pause_menu(),
                Menu::GameOver(finished_stats) => self.game_over_menu(finished_stats),
                Menu::GameComplete(finished_stats) => self.game_complete_menu(finished_stats),
                Menu::Scores => self.scores_menu(),
                Menu::About => self.about_menu(),
                Menu::Settings => self.settings_menu(),
                Menu::ConfigureKeybinds => self.configure_keybinds_menu(),
                Menu::ConfigureGameplay => self.configure_gameplay_menu(),
                Menu::ConfigureGraphics => self.configure_graphics_menu(),
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
                        "There isn't anything interesting implemented here... (yet)",
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
                        format!("{:^w_main$}", "(menu controls: [↓][↑][←][→] [Enter] [Esc] [Delete], vim)",).italic(),
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
                        "exited with ctrl-c".to_string(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
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

    fn title_menu(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::About,
            Menu::Quit("quit from title menu".to_string()),
        ];
        self.generic_placeholder_menu("", selection)
    }

    fn new_game_menu(&mut self) -> io::Result<MenuUpdate> {
        let normal_gamemodes: [(_, _, Box<dyn Fn() -> Game>); 4] = [
            (
                "40-Lines",
                "How fast can you clear forty lines?".to_string(),
                Box::new(|| Game::new(GameMode::sprint(3))),
            ),
            (
                "Marathon",
                "Can you make it to level 15?".to_string(),
                Box::new(|| Game::new(GameMode::marathon())),
            ),
            (
                "Time Trial",
                "What highscore can you get in 3 minutes?".to_string(),
                Box::new(|| Game::new(GameMode::ultra(1))),
            ),
            (
                "Master",
                "Can you clear 30 levels at instant gravity?".to_string(),
                Box::new(|| Game::new(GameMode::master())),
            ),
        ];
        let mut selected = 0usize;
        let mut customization_selected = 0usize;
        let (d_time, d_score, d_pieces, d_lines, d_gravity) =
            (Duration::from_secs(5), 100, 1, 1, 1);
        loop {
            // First part: rendering the menu.
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            let ng = &mut self.settings.new_game;
            let mut special_gamemodes: Vec<(_, _, Box<dyn Fn() -> Game>)> = vec![
                (
                    "Puzzle",
                    "Can you get perfect clears in all 24 puzzle levels?".to_string(),
                    Box::new(game_mods::puzzle_mode::new_game),
                ),
                (
                    "Cheese",
                    format!("How well can you eat through lines like Swiss cheese? [lines: {:?}]", ng.cheese_mode_limit),
                    Box::new({
                        let cheese_mode_limit = ng.cheese_mode_limit;
                        let cheese_mode_gap_size = ng.cheese_mode_gap_size;
                        let cheese_mode_gravity = ng.cheese_mode_gravity;
                        move || {
                            game_mods::cheese_mode::new_game(
                                cheese_mode_limit,
                                cheese_mode_gap_size,
                                cheese_mode_gravity,
                            )
                        }
                    }),
                ),
                (
                    "Combo",
                    format!(
                        "How long can you go on? [start pattern: {:b}]",
                        ng.combo_start_layout
                    ),
                    Box::new({
                        let combo_start_layout = ng.combo_start_layout;
                        move || {
                            game_mods::combo_mode::new_game(1, combo_start_layout)
                        }
                    }),
                ),
            ];
            if ng.descent_mode {
                special_gamemodes.insert(
                    1,
                    (
                        "Descent (experimental)",
                        "Spin the piece and collect 'gems' by touching them.".to_string(),
                        Box::new(game_mods::descent_mode::new_game),
                    ),
                )
            }
            // There are the normal, special, + the custom gamemode.
            let selection_size = normal_gamemodes.len() + special_gamemodes.len() + 1;
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
            for (i, (name, details, _)) in normal_gamemodes
                .iter()
                .chain(special_gamemodes.iter())
                .enumerate()
            {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(i + if normal_gamemodes.len() <= i { 1 } else { 0 })
                                .unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if i == selected {
                            format!(">> {name}: {details} <<")
                        } else {
                            name.to_string()
                        }
                    )))?;
            }
            // Render custom mode option.
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main
                        + y_selection
                        + 4
                        + u16::try_from(normal_gamemodes.len() + 1 + special_gamemodes.len() + 1)
                            .unwrap(),
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == selection_size - 1 {
                        if customization_selected > 0 {
                            " | Custom:                             "
                        } else if ng.custom_start_seed.is_some()
                            || ng.custom_start_board.is_some()
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
                    format!("| Initial gravity: {}", ng.initial_gravity),
                    format!("| Auto-increase gravity: {}", ng.increase_gravity),
                    format!("| Limit: {:?} [→]", ng.custom_mode_limit),
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
                            format!(">{stat_str}")
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
                        "exited with ctrl-c".to_string(),
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
                    code: KeyCode::Enter,
                    kind: Press,
                    ..
                }) => {
                    let mut game = if selected < normal_gamemodes.len() {
                        normal_gamemodes[selected].2()
                    } else if selected < normal_gamemodes.len() + special_gamemodes.len() {
                        special_gamemodes[selected - normal_gamemodes.len()].2()
                    } else {
                        let limits = match ng.custom_mode_limit {
                            Some(Stat::Time(max_dur)) => Limits {
                                time: Some((true, max_dur)),
                                ..Default::default()
                            },
                            Some(Stat::Pieces(max_pcs)) => Limits {
                                pieces: Some((true, max_pcs)),
                                ..Default::default()
                            },
                            Some(Stat::Lines(max_lns)) => Limits {
                                lines: Some((true, max_lns)),
                                ..Default::default()
                            },
                            Some(Stat::Gravity(max_lvl)) => Limits {
                                gravity: Some((true, max_lvl)),
                                ..Default::default()
                            },
                            Some(Stat::Score(max_pts)) => Limits {
                                score: Some((true, max_pts)),
                                ..Default::default()
                            },
                            None => Limits::default(),
                        };
                        let custom_mode = GameMode {
                            name: Some("Custom Mode".to_string()),
                            initial_gravity: ng.initial_gravity,
                            increase_gravity: ng.increase_gravity,
                            limits,
                        };
                        let mut custom_game_builder = Game::builder(custom_mode);
                        if let Some(seed) = ng.custom_start_seed {
                            custom_game_builder.seed(seed);
                        }
                        if let Some(ref custom_start_board_str) = ng.custom_start_board {
                            custom_game_builder.build_modified(vec![game_mods::utils::custom_start_board(
                                custom_start_board_str,
                            )])
                        } else {
                            custom_game_builder.build()
                        }
                    };
                    // Set config.
                    game.config_mut().clone_from(&self.settings.game_config);
                    let now = Instant::now();
                    break Ok(MenuUpdate::Push(Menu::Game {
                        game: Box::new(game),
                        time_started: now,
                        last_paused: now,
                        total_duration_paused: Duration::ZERO,
                        running_game_stats: RunningGameStats::default(),
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
                                ng.initial_gravity = ng.initial_gravity.saturating_add(d_gravity);
                            }
                            2 => {
                                ng.increase_gravity = !ng.increase_gravity;
                            }
                            3 => {
                                match ng.custom_mode_limit {
                                    Some(Stat::Time(ref mut dur)) => {
                                        *dur += d_time;
                                    }
                                    Some(Stat::Score(ref mut pts)) => {
                                        *pts += d_score;
                                    }
                                    Some(Stat::Pieces(ref mut pcs)) => {
                                        *pcs += d_pieces;
                                    }
                                    Some(Stat::Lines(ref mut lns)) => {
                                        *lns += d_lines;
                                    }
                                    Some(Stat::Gravity(ref mut lvl)) => {
                                        *lvl = lvl.saturating_add(d_gravity);
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
                                ng.initial_gravity = ng.initial_gravity.saturating_sub(d_gravity);
                            }
                            2 => {
                                ng.increase_gravity = !ng.increase_gravity;
                            }
                            3 => {
                                match ng.custom_mode_limit {
                                    Some(Stat::Time(ref mut dur)) => {
                                        *dur = dur.saturating_sub(d_time);
                                    }
                                    Some(Stat::Score(ref mut pts)) => {
                                        *pts = pts.saturating_sub(d_score);
                                    }
                                    Some(Stat::Pieces(ref mut pcs)) => {
                                        *pcs = pcs.saturating_sub(d_pieces);
                                    }
                                    Some(Stat::Lines(ref mut lns)) => {
                                        *lns = lns.saturating_sub(d_lines);
                                    }
                                    Some(Stat::Gravity(ref mut lvl)) => {
                                        *lvl = lvl.saturating_sub(d_gravity);
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
                        let new_layout_idx = if let Some(i) = game_mods::combo_mode::LAYOUTS
                            .iter()
                            .position(|lay| *lay == ng.combo_start_layout)
                        {
                            let layout_cnt = game_mods::combo_mode::LAYOUTS.len();
                            (i + layout_cnt - 1) % layout_cnt
                        } else {
                            0
                        };
                        ng.combo_start_layout = game_mods::combo_mode::LAYOUTS[new_layout_idx];
                    } else if selected == selection_size - 3 {
                        if let Some(limit) = ng.cheese_mode_limit {
                            ng.cheese_mode_limit = NonZeroUsize::try_from(limit.get() - 1).ok();
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
                            ng.custom_mode_limit = match ng.custom_mode_limit {
                                Some(Stat::Time(_)) => Some(Stat::Score(9000)),
                                Some(Stat::Score(_)) => Some(Stat::Pieces(100)),
                                Some(Stat::Pieces(_)) => Some(Stat::Lines(40)),
                                Some(Stat::Lines(_)) => Some(Stat::Gravity(20)),
                                Some(Stat::Gravity(_)) => None,
                                None => Some(Stat::Time(Duration::from_secs(180))),
                            };
                        } else {
                            customization_selected += 1
                        }
                    } else if selected == selection_size - 2 {
                        let new_layout_idx = if let Some(i) = crate::game_mods::combo_mode::LAYOUTS
                            .iter()
                            .position(|lay| *lay == ng.combo_start_layout)
                        {
                            let layout_cnt = crate::game_mods::combo_mode::LAYOUTS.len();
                            (i + 1) % layout_cnt
                        } else {
                            0
                        };
                        ng.combo_start_layout =
                            crate::game_mods::combo_mode::LAYOUTS[new_layout_idx];
                    } else if selected == selection_size - 3 {
                        ng.cheese_mode_limit = if let Some(limit) = ng.cheese_mode_limit {
                            limit.checked_add(1)
                        } else {
                            Some(NonZeroUsize::MIN)
                        };
                    }
                }
                // Move selector right (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Delete,
                    kind: Press | Repeat,
                    ..
                }) => {
                    // If custom gamemode selected, allow deleting custom start board and seed.
                    if selected == selection_size - 1 {
                        ng.custom_start_seed = None;
                        ng.custom_start_board = None;
                    }
                }
                // Other event: don't care.
                _ => {}
            }
        }
    }

    fn game_menu(
        &mut self,
        game: &mut Game,
        time_started: &mut Instant,
        last_paused: &mut Instant,
        total_duration_paused: &mut Duration,
        running_game_stats: &mut RunningGameStats,
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
        let mut buttons_pressed = ButtonsPressed::default();
        let (button_sender, button_receiver) = mpsc::channel();
        let _input_handler =
            CrosstermHandler::new(&button_sender, &self.settings.keybinds, self.kitty_assumed);
        let mut combo_bot_handler = (self.combo_bot_enabled && game.mode().name.as_ref().is_some_and(|n| n == "Combo"))
            .then(|| ComboBotHandler::new(&button_sender, Duration::from_millis(100)));
        let mut inform_combo_bot = |game: &Game, evts: &FeedbackMessages| {
            if let Some((_, state_sender)) = &mut combo_bot_handler {
                if evts.iter().any(|(_, feedback)| {
                    matches!(feedback, tetrs_engine::Feedback::PieceSpawned(_))
                }) {
                    let combo_state = ComboBotHandler::encode(game).unwrap();
                    if state_sender.send(combo_state).is_err() {
                        combo_bot_handler = None;
                    }
                }
            }
        };
        // Game Loop
        let session_resumed = Instant::now();
        *total_duration_paused += session_resumed.saturating_duration_since(*last_paused);
        let mut clean_screen = true;
        let mut f = 0u32;
        let mut fps_counter = 0;
        let mut fps_counter_started = Instant::now();
        let menu_update = 'render: loop {
            // Exit if game ended
            if game.ended() {
                let finished_game_stats = self.store_game(game, running_game_stats);
                let menu = if finished_game_stats.was_successful() {
                    Menu::GameComplete
                } else {
                    Menu::GameOver
                }(Box::new(finished_game_stats));
                break 'render MenuUpdate::Push(menu);
            }
            // Start next frame
            f += 1;
            fps_counter += 1;
            let next_frame_at = loop {
                let frame_at = session_resumed
                    + Duration::from_secs_f64(f64::from(f) / self.settings.game_fps);
                if frame_at < Instant::now() {
                    f += 1;
                } else {
                    break frame_at;
                }
            };
            let mut new_feedback_events = Vec::new();
            'frame_idle: loop {
                let frame_idle_remaining = next_frame_at - Instant::now();
                match button_receiver.recv_timeout(frame_idle_remaining) {
                    Ok(InputSignal::AbortProgram) => {
                        self.store_game(game, running_game_stats);
                        break 'render MenuUpdate::Push(Menu::Quit(
                            "exited with ctrl-c".to_string(),
                        ));
                    }
                    Ok(InputSignal::ForfeitGame) => {
                        game.forfeit();
                        let finished_game_stats = self.store_game(game, running_game_stats);
                        break 'render MenuUpdate::Push(Menu::GameOver(Box::new(
                            finished_game_stats,
                        )));
                    }
                    Ok(InputSignal::Pause) => {
                        *last_paused = Instant::now();
                        break 'render MenuUpdate::Push(Menu::Pause);
                    }
                    Ok(InputSignal::WindowResize) => {
                        clean_screen = true;
                        continue 'frame_idle;
                    }
                    Ok(InputSignal::TakeSnapshot) => {
                        self.settings.new_game.custom_start_board = Some(String::from_iter(
                            game.state().board.iter().rev().flat_map(|line| {
                                line.iter()
                                    .map(|cell| if cell.is_some() { 'X' } else { ' ' })
                            }),
                        ));
                        self.settings.new_game
                            .custom_start_seed = Some(game.seed());
                        new_feedback_events.push((
                            game.state().time,
                            tetrs_engine::Feedback::Message("(Snapshot taken!)".to_string()),
                        ));
                    }
                    Ok(InputSignal::ButtonInput(button, button_state, instant)) => {
                        buttons_pressed[button] = button_state;
                        let game_time_userinput = instant.saturating_duration_since(*time_started)
                            - *total_duration_paused;
                        let game_now = std::cmp::max(game_time_userinput, game.state().time);
                        // FIXME: Handle/ensure no Err.
                        if let Ok(evts) = game.update(Some(buttons_pressed), game_now) {
                            inform_combo_bot(game, &evts);
                            new_feedback_events.extend(evts);
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let game_time_now = Instant::now().saturating_duration_since(*time_started)
                            - *total_duration_paused;
                        // FIXME: Handle/ensure no Err.
                        if let Ok(evts) = game.update(None, game_time_now) {
                            inform_combo_bot(game, &evts);
                            new_feedback_events.extend(evts);
                        }
                        break 'frame_idle;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // NOTE: We kind of rely on this not happening too often.
                        break 'render MenuUpdate::Push(Menu::Pause);
                    }
                };
            }
            game_renderer.render(
                self,
                running_game_stats,
                game,
                new_feedback_events,
                clean_screen,
            )?;
            clean_screen = false;
            // FPS counter.
            if self.settings.show_fps {
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
        if let Some(finished_state) = game.state().end {
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

    fn game_ended_menu(
        &mut self,
        selection: Vec<Menu>,
        success: bool,
        finished_game_stats: &FinishedGameStats,
    ) -> io::Result<MenuUpdate> {
        let FinishedGameStats {
            timestamp: _,
            actions,
            score_bonuses: _,
            gamemode,
            last_state,
        } = finished_game_stats;
        let GameState {
            end: _,
            time: game_time,
            events: _,
            buttons_pressed: _,
            board: _,
            active_piece_data: _,
            hold_piece: _,
            next_pieces: _,
            pieces_played,
            lines_cleared,
            gravity,
            score,
            consecutive_line_clears: _,
            back_to_back_special_clears: _,
        } = last_state;
        // if gamemode.name.as_ref().map(String::as_str) == Some("Puzzle")
        if gamemode.name.as_ref().is_some_and(|n| n == "Puzzle") && success {
            self.settings.new_game.descent_mode = true;
        }
        let actions_str = [
            format!(
                "{} Single{}",
                actions[1],
                if actions[1] != 1 { "s" } else { "" }
            ),
            format!(
                "{} Double{}",
                actions[2],
                if actions[2] != 1 { "s" } else { "" }
            ),
            format!(
                "{} Triple{}",
                actions[3],
                if actions[3] != 1 { "s" } else { "" }
            ),
            format!(
                "{} Quadruple{}",
                actions[4],
                if actions[4] != 1 { "s" } else { "" }
            ),
            format!(
                "{} Spin{}",
                actions[0],
                if actions[0] != 1 { "s" } else { "" }
            ),
        ]
        .join(", ");
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
                    if success {
                        format!(
                            "++ Game Completed{} ++",
                            gamemode.name.as_ref().map_or("".to_string(), |name| format!(" ({name})"))
                        )
                    } else {
                        format!(
                            "-- Game Over{} by: {:?} --",
                            gamemode.name.as_ref().map_or("".to_string(), |name| format!(" ({name})")),
                            last_state.end.unwrap().unwrap_err()
                        )
                    }
                )))?
                /*.queue(MoveTo(0, y_main + y_selection + 2))?
                .queue(Print(Self::produce_header()?))?*/
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?
                .queue(MoveTo(x_main, y_main + y_selection + 4))?
                .queue(Print(format!("{:^w_main$}", format!("Score: {score}"))))?
                .queue(MoveTo(x_main, y_main + y_selection + 5))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Gravity: {gravity}",)
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 6))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Lines: {}", lines_cleared)
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 7))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Pieces: {}", pieces_played.iter().sum::<u32>())
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 8))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Time: {}", fmt_duration(*game_time))
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 10))?
                .queue(Print(format!("{:^w_main$}", actions_str)))?
                .queue(MoveTo(x_main, y_main + y_selection + 12))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let names = selection
                .iter()
                .map(|menu| menu.to_string())
                .collect::<Vec<_>>();
            for (i, name) in names.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 13 + u16::try_from(i).unwrap(),
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
                        "exited with ctrl-c".to_string(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
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

    fn game_over_menu(
        &mut self,
        finished_game_stats: &FinishedGameStats,
    ) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::Quit("quit after game over".to_string()),
        ];
        self.game_ended_menu(selection, false, finished_game_stats)
    }

    fn game_complete_menu(
        &mut self,
        finished_game_stats: &FinishedGameStats,
    ) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::Quit("quit after game complete".to_string()),
        ];
        self.game_ended_menu(selection, true, finished_game_stats)
    }

    fn pause_menu(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::About,
            Menu::Quit("quit from pause".to_string()),
        ];
        self.generic_placeholder_menu("Game Paused", selection)
    }

    fn settings_menu(&mut self) -> io::Result<MenuUpdate> {
        let selection_len = 4 + 1; // `+1` for hacky empty line.
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
                "Configure Graphics...".to_string(),
                "Configure Keybinds...".to_string(),
                "Configure Gameplay...".to_string(),
                "".to_string(),
                format!("Keep save file: {}", match self.settings.save_on_exit {
                    SavefileGranularity::Nothing => "OFF*",
                    SavefileGranularity::Settings => "ON [without games; only settings]",
                    SavefileGranularity::SettingsAndGames => "ON",
                }),
                if self.settings.save_on_exit == SavefileGranularity::Nothing {
                    "(*WARNING: current data will be lost on exit)".to_string()
                } else {
                    format!("(Save file at {:?})", Self::savefile_path())
                },
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
                        "exited with ctrl-c".to_string(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: Press,
                    ..
                }) => match selected {
                    0 => break Ok(MenuUpdate::Push(Menu::ConfigureGraphics)),
                    1 => break Ok(MenuUpdate::Push(Menu::ConfigureKeybinds)),
                    2 => break Ok(MenuUpdate::Push(Menu::ConfigureGameplay)),
                    _ => {}
                },
                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 4 {
                        // Skip hacky empty line.
                        selected += selection_len - 2;
                    } else {
                        selected += selection_len - 1;
                    }
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 2 {
                        // Skip hacky empty line.
                        selected += 2;
                    } else {
                        selected += 1;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => match selected { // TODO add more cases to switch slots for keybinds/gameplayconfigs/graphicsconfigs...
                    4 => {
                        self.settings.save_on_exit = match self.settings.save_on_exit {
                            SavefileGranularity::Nothing => SavefileGranularity::SettingsAndGames,
                            SavefileGranularity::Settings => SavefileGranularity::Nothing,
                            SavefileGranularity::SettingsAndGames => SavefileGranularity::Settings,
                        };
                    }
                    _ => {}
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    4 => {
                        self.settings.save_on_exit = match self.settings.save_on_exit {
                            SavefileGranularity::Nothing => SavefileGranularity::Settings,
                            SavefileGranularity::Settings => SavefileGranularity::SettingsAndGames,
                            SavefileGranularity::SettingsAndGames => SavefileGranularity::Nothing,
                        };
                    }
                    _ => {}
                },
                // Other event: Just ignore.
                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }

    fn configure_keybinds_menu(&mut self) -> io::Result<MenuUpdate> {
        let button_selection = Button::VARIANTS;
        let selection_len = button_selection.len() + 1;
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "# Configure Keybinds #")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let button_names = button_selection
                .iter()
                .map(|&button| {
                    format!(
                        "{button:?}: {}",
                        fmt_keybinds(button, &self.settings.keybinds)
                    )
                })
                .collect::<Vec<_>>();
            for (i, name) in button_names.into_iter().enumerate() {
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
                    y_main + y_selection + 4 + u16::try_from(selection_len - 1).unwrap() + 1,
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == selection_len - 1 {
                        ">> Restore defaults <<"
                    } else {
                        "Restore defaults"
                    }
                )))?
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(selection_len).unwrap() + 3,
                ))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "(Menu controls: [Enter] to add, [Del] to remove keybinds)",).italic(),
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
                        "exited with ctrl-c".to_string(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select button to modify.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: Press,
                    ..
                }) => {
                    if selected == selection_len - 1 {
                        self.settings.keybinds = CrosstermHandler::default_keybinds();
                    } else {
                        let current_button = button_selection[selected];
                        self.term
                            .execute(MoveTo(
                                x_main,
                                y_main
                                    + y_selection
                                    + 4
                                    + u16::try_from(selection_len).unwrap()
                                    + 3,
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
                        loop {
                            if let Event::Key(KeyEvent {
                                code, kind: Press, ..
                            }) = event::read()?
                            {
                                self.settings.keybinds.insert(code, current_button);
                                break;
                            }
                        }
                    }
                }
                // Select button to delete.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete,
                    kind: Press,
                    ..
                }) => {
                    if selected == selection_len - 1 {
                        self.settings.keybinds.clear();
                    } else {
                        let current_button = button_selection[selected];
                        self.settings
                            .keybinds
                            .retain(|_code, button| *button != current_button);
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
                // Other event: don't care.
                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }

    fn configure_gameplay_menu(&mut self) -> io::Result<MenuUpdate> {
        let selection_len = 13;
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
                    "@ Configure Gameplay (requires New Game) @"
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let labels = [
                format!("Rotation system: {:?}", self.settings.game_config.rotation_system),
                format!(
                    "Piece generator: {}",
                    match &self.settings.game_config.tetromino_generator {
                        TetrominoSource::Uniform => "Uniform".to_string(),
                        TetrominoSource::Stock { .. } => "Bag (Stock)".to_string(),
                        TetrominoSource::Recency { .. } => "Recency-based".to_string(),
                        TetrominoSource::BalanceRelative { .. } =>
                            "Balance Relative Counts".to_string(),
                        TetrominoSource::Cycle { pattern, index: _ } =>
                            format!("Cycle Pattern {pattern:?}"),
                    }
                ),
                format!("Preview size: {}", self.settings.game_config.preview_count),
                format!(
                    "*Delayed auto shift: {:?}",
                    self.settings.game_config.delayed_auto_shift
                ),
                format!(
                    "*Auto repeat rate: {:?}",
                    self.settings.game_config.auto_repeat_rate
                ),
                format!("*Soft drop factor: {}", self.settings.game_config.soft_drop_factor),
                format!("Hard drop delay: {:?}", self.settings.game_config.hard_drop_delay),
                format!("Ground time max: {:?}", self.settings.game_config.ground_time_max),
                format!("Line clear delay: {:?}", self.settings.game_config.line_clear_delay),
                format!("Appearance delay: {:?}", self.settings.game_config.appearance_delay),
                format!(
                    "**No soft drop lock: {}",
                    self.settings.game_config.no_soft_drop_lock
                ),
                format!(
                    "*Assume enhanced key events (in current game): {}",
                    self.kitty_assumed
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
                    y_main + y_selection + 4 + u16::try_from(selection_len - 1).unwrap() + 1,
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == selection_len - 1 {
                        ">> Restore Defaults <<"
                    } else {
                        "Restore Defaults"
                    }
                )))?;
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(selection_len - 1).unwrap() + 4,
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if self.kitty_detected {
                        "(*Should work - enhanced key events seemed available)"
                    } else {
                        "(*Might NOT work - enhanced key events seemed unavailable)"
                    },
                )))?;
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(selection_len - 1).unwrap() + 5,
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if !self.kitty_detected {
                        "(**Were set to 'false' because enhanced key events seemed unavailable)"
                    } else {
                        "(**Were set to 'true' because enhanced key events seemed available)"
                    }
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
                        "exited with ctrl-c".to_string(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: Press,
                    ..
                }) => {
                    if selected == selection_len - 1 {
                        self.settings.game_config = GameConfig::default();
                        self.settings.game_config.no_soft_drop_lock = !self.kitty_detected;
                        self.kitty_assumed = self.kitty_detected;
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
                        self.settings.game_config.rotation_system = match self.settings.game_config.rotation_system {
                            RotationSystem::Ocular => RotationSystem::Classic,
                            RotationSystem::Classic => RotationSystem::Super,
                            RotationSystem::Super => RotationSystem::Ocular,
                        };
                    }
                    1 => {
                        self.settings.game_config.tetromino_generator = match self
                            .settings.game_config
                            .tetromino_generator
                        {
                            TetrominoSource::Uniform => TetrominoSource::bag(),
                            TetrominoSource::Stock { .. } => TetrominoSource::recency(),
                            TetrominoSource::Recency { .. } => TetrominoSource::balance_relative(),
                            TetrominoSource::BalanceRelative { .. } => TetrominoSource::uniform(),
                            TetrominoSource::Cycle { .. } => TetrominoSource::uniform(),
                        };
                    }
                    2 => {
                        self.settings.game_config.preview_count += 1;
                    }
                    3 => {
                        self.settings.game_config.delayed_auto_shift += Duration::from_millis(1);
                    }
                    4 => {
                        self.settings.game_config.auto_repeat_rate += Duration::from_millis(1);
                    }
                    5 => {
                        self.settings.game_config.soft_drop_factor += 0.5;
                    }
                    6 => {
                        self.settings.game_config.hard_drop_delay += Duration::from_millis(1);
                    }
                    7 => {
                        self.settings.game_config.ground_time_max += Duration::from_millis(250);
                    }
                    8 => {
                        self.settings.game_config.line_clear_delay += Duration::from_millis(10);
                    }
                    9 => {
                        self.settings.game_config.appearance_delay += Duration::from_millis(10);
                    }
                    10 => {
                        self.settings.game_config.no_soft_drop_lock = !self.settings.game_config.no_soft_drop_lock;
                    }
                    11 => {
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
                        self.settings.game_config.rotation_system = match self.settings.game_config.rotation_system {
                            RotationSystem::Ocular => RotationSystem::Super,
                            RotationSystem::Super => RotationSystem::Classic,
                            RotationSystem::Classic => RotationSystem::Ocular,
                        };
                    }
                    1 => {
                        self.settings.game_config.tetromino_generator = match self
                            .settings.game_config
                            .tetromino_generator
                        {
                            TetrominoSource::Uniform => TetrominoSource::balance_relative(),
                            TetrominoSource::Stock { .. } => TetrominoSource::uniform(),
                            TetrominoSource::Recency { .. } => TetrominoSource::bag(),
                            TetrominoSource::BalanceRelative { .. } => TetrominoSource::recency(),
                            TetrominoSource::Cycle { .. } => TetrominoSource::uniform(),
                        };
                    }
                    2 => {
                        self.settings.game_config.preview_count =
                            self.settings.game_config.preview_count.saturating_sub(1);
                    }
                    3 => {
                        self.settings.game_config.delayed_auto_shift = self
                            .settings.game_config
                            .delayed_auto_shift
                            .saturating_sub(Duration::from_millis(1));
                    }
                    4 => {
                        self.settings.game_config.auto_repeat_rate = self
                            .settings.game_config
                            .auto_repeat_rate
                            .saturating_sub(Duration::from_millis(1));
                    }
                    5 => {
                        if self.settings.game_config.soft_drop_factor > 0.0 {
                            self.settings.game_config.soft_drop_factor -= 0.5;
                        }
                    }
                    6 => {
                        if self.settings.game_config.hard_drop_delay >= Duration::from_millis(1) {
                            self.settings.game_config.hard_drop_delay = self
                                .settings.game_config
                                .hard_drop_delay
                                .saturating_sub(Duration::from_millis(1));
                        }
                    }
                    7 => {
                        self.settings.game_config.ground_time_max = self
                            .settings.game_config
                            .ground_time_max
                            .saturating_sub(Duration::from_millis(250));
                    }
                    8 => {
                        self.settings.game_config.line_clear_delay = self
                            .settings.game_config
                            .line_clear_delay
                            .saturating_sub(Duration::from_millis(10));
                    }
                    9 => {
                        self.settings.game_config.appearance_delay = self
                            .settings.game_config
                            .appearance_delay
                            .saturating_sub(Duration::from_millis(10));
                    }
                    10 => {
                        self.settings.game_config.no_soft_drop_lock = !self.settings.game_config.no_soft_drop_lock;
                    }
                    11 => {
                        self.kitty_assumed = !self.kitty_assumed;
                    }
                    _ => {}
                },
                // Other event: don't care.
                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }

    fn configure_graphics_menu(&mut self) -> io::Result<MenuUpdate> {
        let selection_len = 4;
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "* Configure Graphics *")))?
                .queue(MoveTo(x_main, y_main+y_selection+2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let labels = [
                format!("Glyphset: {:?}", self.settings.graphics_glyphset),
                format!("Coloring: {:?}", self.settings.graphics_coloring),
                format!("Framerate: {}", self.settings.game_fps),
                format!("Show fps: {}", self.settings.show_fps),
            ];
            for (i, label) in labels.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main+y_selection+2+2+u16::try_from(i).unwrap(),
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
            self.term.queue(MoveTo(
                x_main + u16::try_from((w_main - 27) / 2).unwrap(),
                y_main + y_selection + 4 + u16::try_from(selection_len).unwrap() + 2,
            ))?;
            for tet in Tetromino::VARIANTS {
                self.term.queue(PrintStyledContent(
                    tet_str_small(&tet).with(
                        game_renderers::tile_to_color(self.settings.graphics_coloring)(
                            tet.tiletypeid(),
                        )
                        .unwrap_or(style::Color::Reset),
                    ),
                ))?;
                self.term.queue(Print(' '))?;
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
                        "exited with ctrl-c".to_string(),
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
                        self.settings.graphics_glyphset = match self.settings.graphics_glyphset {
                            GraphicsGlyphset::Electronika60 => GraphicsGlyphset::ASCII,
                            GraphicsGlyphset::ASCII => GraphicsGlyphset::Unicode,
                            GraphicsGlyphset::Unicode => GraphicsGlyphset::Electronika60,
                        };
                    }
                    1 => {
                        self.settings.graphics_coloring = match self.settings.graphics_coloring {
                            GraphicsColoring::Monochrome => GraphicsColoring::Color16,
                            GraphicsColoring::Color16 => GraphicsColoring::Fullcolor,
                            GraphicsColoring::Fullcolor => GraphicsColoring::Experimental,
                            GraphicsColoring::Experimental => GraphicsColoring::Monochrome,
                        };
                        self.settings.graphics_coloring_locked = self.settings.graphics_coloring;
                    }
                    2 => {
                        self.settings.game_fps += 1.0;
                    }
                    3 => {
                        self.settings.show_fps = !self.settings.show_fps;
                    }
                    _ => {}
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.graphics_glyphset = match self.settings.graphics_glyphset {
                            GraphicsGlyphset::Electronika60 => GraphicsGlyphset::Unicode,
                            GraphicsGlyphset::ASCII => GraphicsGlyphset::Electronika60,
                            GraphicsGlyphset::Unicode => GraphicsGlyphset::ASCII,
                        };
                    }
                    1 => {
                        self.settings.graphics_coloring = match self.settings.graphics_coloring {
                            GraphicsColoring::Monochrome => GraphicsColoring::Experimental,
                            GraphicsColoring::Color16 => GraphicsColoring::Monochrome,
                            GraphicsColoring::Fullcolor => GraphicsColoring::Color16,
                            GraphicsColoring::Experimental => GraphicsColoring::Fullcolor,
                        };
                        self.settings.graphics_coloring_locked = self.settings.graphics_coloring;
                    }
                    2 => {
                        if self.settings.game_fps >= 1.0 {
                            self.settings.game_fps -= 1.0;
                        }
                    }
                    3 => {
                        self.settings.show_fps = !self.settings.show_fps;
                    }
                    _ => {}
                },
                // Other event: Just ignore.
                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }

    // TODO: Enable deleting of entries manually in scoreboard menu.
    fn scores_menu(&mut self) -> io::Result<MenuUpdate> {
        let max_entries = 14;
        let mut scroll = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "= Scoreboard =")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let entries = self
                .past_games
                .iter()
                .skip(scroll)
                .take(max_entries)
                .map(
                    |FinishedGameStats {
                         timestamp,
                         actions: _,
                         score_bonuses: _,
                         gamemode,
                         last_state,
                     }| {
                        // Here I would like to point out the slight poetic quality of this variable
                        // name. We are declaring a variable with an empty string in it to
                        // explicitly borrow it once, merely to satisfy the Rust borrow checker
                        // which would otherwise complain about an empty string not living long
                        // enough (despite our basic intention of using it as an arbitrary,
                        // unimportant and immutable placeholder.)
                        // The variable name `empty` may come to mind first, with other choices such
                        // as `empty_string`, `none`, `nothing`, `null` or just `nil`.
                        // Notice: "nil" is the Latin word for "nothing". This is actually a
                        // 'syncopated' (contracted) version of "nihil", which itself stems from
                        // "nihilum", all meaning 'nothing'. The etymology of "nihilum" suggests a
                        // 'univerbation' (combination) of "ne" + "hilum". Here, "ne" means
                        // 'not'/'no' but the origins of "hilum" are vague:
                        // It is suspected to be a variant of "filum" - i.e. 'thread'; 'string'.
                        // Behold: "nil" literally means "not even a String".
                        // 
                        // Also, "nihilum" is the origin for the English word 'nihilism', which
                        // aptly describes how I feel having to write this sort of code to satisfy
                        // the borrow checker. Probably a skill issue.
                        let nil = &String::new();
                        let name = gamemode.name.as_ref().unwrap_or(nil).as_str();
                        match name {
                            "Marathon" => {
                                format!(
                                    "{timestamp} ~ Marathon: {} pts{}",
                                    last_state.score,
                                    if last_state.end.is_some_and(|end| end.is_ok()) {
                                        "".to_string()
                                    } else {
                                        let Limits {
                                            gravity: Some((_, max_lvl)),
                                            ..
                                        } = gamemode.limits
                                        else {
                                            panic!()
                                        };
                                        format!(" ({}/{} lvl)", last_state.gravity, max_lvl)
                                    },
                                )
                            }
                            "40-Lines" => {
                                format!(
                                    "{timestamp} ~ 40-Lines: {}{}",
                                    fmt_duration(last_state.time),
                                    if last_state.end.is_some_and(|end| end.is_ok()) {
                                        "".to_string()
                                    } else {
                                        let Limits {
                                            lines: Some((_, max_lns)),
                                            ..
                                        } = gamemode.limits
                                        else {
                                            panic!()
                                        };
                                        format!(" ({}/{} lns)", last_state.lines_cleared, max_lns)
                                    },
                                )
                            }
                            "Time Trial" => {
                                format!(
                                    "{timestamp} ~ Time Trial: {} pts{}",
                                    last_state.score,
                                    if last_state.end.is_some_and(|end| end.is_ok()) {
                                        "".to_string()
                                    } else {
                                        let Limits {
                                            time: Some((_, max_dur)),
                                            ..
                                        } = gamemode.limits
                                        else {
                                            panic!()
                                        };
                                        format!(
                                            " ({} / {})",
                                            fmt_duration(last_state.time),
                                            fmt_duration(max_dur)
                                        )
                                    },
                                )
                            }
                            "Master" => {
                                let Limits {
                                    gravity: Some((_, max_lvl)),
                                    ..
                                } = gamemode.limits
                                else {
                                    panic!()
                                };
                                format!(
                                    "{timestamp} ~ Master: gravity lvl {}/{}",
                                    last_state.gravity, max_lvl
                                )
                            }
                            "Puzzle" => {
                                format!(
                                    "{timestamp} ~ Puzzle: {}{}",
                                    fmt_duration(last_state.time),
                                    if last_state.end.is_some_and(|end| end.is_ok()) {
                                        "".to_string()
                                    } else {
                                        let Limits {
                                            gravity: Some((_, max_lvl)),
                                            ..
                                        } = gamemode.limits
                                        else {
                                            panic!()
                                        };
                                        format!(" ({}/{} lvl)", last_state.gravity, max_lvl)
                                    },
                                )
                            }
                            "Descent" => {
                                format!(
                                    "{timestamp} ~ Descent: {} gems, depth {}",
                                    last_state.score, last_state.lines_cleared,
                                )
                            }
                            "Cheese" => {
                                format!(
                                    "{timestamp} ~ Cheese: {} ({}/{} lns)",
                                    last_state.pieces_played.iter().sum::<u32>(),
                                    last_state.lines_cleared,
                                    gamemode
                                        .limits
                                        .lines
                                        .map_or("∞".to_string(), |(_, max_lns)| max_lns
                                            .to_string())
                                )
                            }
                            "Combo" => {
                                format!("{timestamp} ~ Combo: {} lns", last_state.lines_cleared)
                            }
                            _ => {
                                format!(
                                    "{timestamp} ~ Custom Mode: {} lns, {} pts, {}{}",
                                    last_state.lines_cleared,
                                    last_state.score,
                                    fmt_duration(last_state.time),
                                    [
                                        gamemode.limits.time.map(|(_, max_dur)| format!(
                                            " ({} / {})",
                                            fmt_duration(last_state.time),
                                            fmt_duration(max_dur)
                                        )),
                                        gamemode.limits.pieces.map(|(_, max_pcs)| format!(
                                            " ({}/{} pcs)",
                                            last_state.pieces_played.iter().sum::<u32>(),
                                            max_pcs
                                        )),
                                        gamemode.limits.lines.map(|(_, max_lns)| format!(
                                            " ({}/{} lns)",
                                            last_state.lines_cleared, max_lns
                                        )),
                                        gamemode.limits.gravity.map(|(_, max_lvl)| format!(
                                            " ({}/{} lvl)",
                                            last_state.gravity, max_lvl
                                        )),
                                        gamemode.limits.score.map(|(_, max_pts)| format!(
                                            " ({}/{} pts)",
                                            last_state.score, max_pts
                                        )),
                                    ]
                                    .into_iter()
                                    .find_map(|limit_text| limit_text)
                                    .unwrap_or_default()
                                )
                            }
                        }
                    },
                )
                .collect::<Vec<_>>();
            let n_entries = entries.len();
            for (i, entry) in entries.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!("{:<w_main$}", entry)))?;
            }
            let entries_left = self.past_games.len().saturating_sub(max_entries + scroll);
            if entries_left > 0 {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(n_entries).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        format!("...  (+{entries_left} more)")
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
                        "exited with ctrl-c".to_string(),
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
                }) => {
                    scroll = scroll.saturating_sub(1);
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if entries_left > 0 {
                        scroll += 1;
                    }
                }
                // Other event: don't care.
                _ => {}
            }
        }
    }

    fn about_menu(&mut self) -> io::Result<MenuUpdate> {
        /* FIXME: About menu. */
        self.generic_placeholder_menu(
            "About tetrs - Visit https://github.com/Strophox/tetrs",
            vec![],
        )
    }

    fn store_game(
        &mut self,
        game: &Game,
        running_game_stats: &mut RunningGameStats,
    ) -> FinishedGameStats {
        let finished_game_stats = FinishedGameStats {
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string(),
            actions: running_game_stats.0,
            score_bonuses: running_game_stats.1.clone(),
            gamemode: game.mode().clone(),
            last_state: game.state().clone(),
        };
        self.past_games.push(finished_game_stats.clone());
        self.past_games
            .sort_by(|stats1, stats2| {
                // First sort by gamemode.
                stats1.gamemode.name.cmp(&stats2.gamemode.name).then_with(|| {
                    // Sort by whether game was finished successfully or not.
                    let end1 = stats1.last_state.end.is_some_and(|end| end.is_ok());
                    let end2 = stats2.last_state.end.is_some_and(|end| end.is_ok());
                    end1.cmp(&end2).reverse().then_with(|| {
                        // Depending on gamemode, sort differently.
                        match stats1.gamemode.name.as_ref().unwrap_or(&"".to_string()).as_str() {
                            "Marathon" => {
                                // Sort desc by level.
                                stats1.last_state.gravity.cmp(&stats2.last_state.gravity).reverse().then_with(||
                                    // Sort desc by score.

                                    stats1.last_state.score.cmp(&stats2.last_state.score).reverse()
                                )
                            },
                            "40-Lines" => {
                                // Sort desc by lines.
                                stats1.last_state.lines_cleared.cmp(&stats2.last_state.lines_cleared).reverse().then_with(||
                                    // Sort asc by time.
                                    stats1.last_state.time.cmp(&stats2.last_state.time)
                                )
                            },
                            "Time Trial" => {
                                // Sort asc by time.
                                stats1.last_state.time.cmp(&stats2.last_state.time).then_with(||
                                    // Sort by desc score.
                                    stats1.last_state.score.cmp(&stats2.last_state.score).reverse()
                                )
                            },
                            "Master" => {
                                // Sort desc by lines.
                                stats1.last_state.lines_cleared.cmp(&stats2.last_state.lines_cleared).reverse()
                            },
                            "Puzzle" => {
                                // Sort desc by level.
                                stats1.last_state.gravity.cmp(&stats2.last_state.gravity).reverse().then_with(||
                                    // Sort asc by time.
                                    stats1.last_state.time.cmp(&stats2.last_state.time)
                                )
                            },
                            "Descent" => {
                                // Sort desc by score.
                                stats1.last_state.score.cmp(&stats2.last_state.score).reverse().then_with(||
                                    // Sort desc by depth.
                                    stats1.last_state.lines_cleared.cmp(&stats2.last_state.lines_cleared).reverse()
                                )
                            },
                            "Cheese" => {
                                // Sort desc by lines.
                                stats1.last_state.lines_cleared.cmp(&stats2.last_state.lines_cleared).reverse().then_with(||
                                    // Sort asc by number of pieces played.
                                    stats1.last_state.pieces_played.iter().sum::<u32>().cmp(&stats2.last_state.pieces_played.iter().sum::<u32>())
                                )
                            },
                            "Combo" => {
                                // Sort desc by lines.
                                stats1.last_state.lines_cleared.cmp(&stats2.last_state.lines_cleared).reverse()
                            },
                            _ => {
                                // Sort desc by lines.
                                stats1.last_state.lines_cleared.cmp(&stats2.last_state.lines_cleared).reverse()
                            },
                        }.then_with(|| {
                            // Sort asc by timestamp.
                            stats1.timestamp.cmp(&stats2.timestamp)
                        })
                    })
                })
            });
        finished_game_stats
    }
}

const DAVIS: &str = " ▀█▀ \"I am like Solomon because I built God's temple, an operating system. God said 640x480 16 color graphics but the operating system is 64-bit and multi-cored! Go draw a 16 color elephant. Then, draw a 24-bit elephant in MS Paint and be enlightened. Artist stopped photorealism when the camera was invented. A cartoon is actually better than photorealistic. For the next thousand years, first-person shooters are going to get boring. Tetris looks good.\" - In memory of Terry A. Davis";

pub fn fmt_duration(dur: Duration) -> String {
    format!(
        "{}min {}.{:02}sec",
        dur.as_secs() / 60,
        dur.as_secs() % 60,
        dur.as_millis() % 1000 / 10
    )
}

pub fn fmt_key(key: KeyCode) -> String {
    format!(
        "[{}]",
        match key {
            KeyCode::Backspace => "Back".to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Left => "←".to_string(),
            KeyCode::Right => "→".to_string(),
            KeyCode::Up => "↑".to_string(),
            KeyCode::Down => "↓".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PgUp".to_string(),
            KeyCode::PageDown => "PgDn".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::Delete => "Del".to_string(),
            KeyCode::F(n) => format!("F{n}"),
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Char(c) => c.to_uppercase().to_string(),
            KeyCode::Esc => "Esc".to_string(),
            k => format!("{:?}", k),
        }
    )
}

pub fn fmt_keybinds(button: Button, keybinds: &HashMap<KeyCode, Button>) -> String {
    keybinds
        .iter()
        .filter_map(|(&k, &b)| (b == button).then_some(fmt_key(k)))
        .collect::<Vec<String>>()
        .join(" ")
}
