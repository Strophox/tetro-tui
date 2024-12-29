use std::{
    collections::HashMap,
    env,
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
    FeedbackEvents, Game, GameConfig, GameMode, GameState, Limits,
};

use crate::{
    game_input_handlers::{combo_bot::ComboBotHandler, crossterm::CrosstermHandler, Interrupt},
    game_mods,
    game_renderers::{cached_renderer::CachedRenderer, Renderer},
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
    ChangeKeybinds,
    ConfigureGameplay,
    Scores,
    About,
    Quit(String),
}

impl std::fmt::Display for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Menu::Title => "Title Screen",
            Menu::NewGame => "New Game",
            Menu::Game { game, .. } => &format!("Game: {}", game.mode().name),
            Menu::GameOver(_) => "Game Over",
            Menu::GameComplete(_) => "Game Completed",
            Menu::Pause => "Pause",
            Menu::Settings => "Settings",
            Menu::ChangeKeybinds => "Change Keybinds",
            Menu::ConfigureGameplay => "Configure Gameplay",
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

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum GraphicsStyle {
    Electronika60,
    #[allow(clippy::upper_case_acronyms)]
    ASCII,
    Unicode,
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum GraphicsColor {
    Monochrome,
    Color16,
    Fullcolor,
    Experimental,
}

#[serde_with::serde_as]
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    #[serde_as(as = "HashMap<serde_with::json::JsonString, _>")]
    pub keybinds: HashMap<KeyCode, Button>,
    pub game_fps: f64,
    pub show_fps: bool,
    pub graphics_style: GraphicsStyle,
    pub graphics_color: GraphicsColor,
    pub graphics_color_board: GraphicsColor,
    pub save_data_on_exit: bool,
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
}

#[derive(Clone, Debug)]
pub struct TerminalApp<T: Write> {
    pub term: T,
    kitty_detected: bool,
    kitty_assumed: bool,
    settings: Settings,
    game_config: GameConfig,
    new_game_settings: NewGameSettings,
    combo_bot_enabled: bool,
    past_games: Vec<FinishedGameStats>,
}

impl<T: Write> Drop for TerminalApp<T> {
    fn drop(&mut self) {
        // FIXME: Handle errors?
        let savefile_path = Self::savefile_path();
        // If the user wants their data stored, try to do so.
        if self.settings.save_data_on_exit {
            if let Err(_e) = self.store_savefile(savefile_path) {
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

impl<T: Write> TerminalApp<T> {
    pub const W_MAIN: u16 = 80;
    pub const H_MAIN: u16 = 24;

    pub const SAVEFILE_NAME: &'static str = ".tetrs_tui_savefile.json";

    pub fn new(
        mut terminal: T,
        combo_start_layout: Option<u16>,
        custom_start_board: Option<String>,
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
            kitty_detected,
            kitty_assumed: kitty_detected,
            settings: Settings {
                keybinds: CrosstermHandler::default_keybinds(),
                game_fps: 30.0,
                show_fps: false,
                graphics_style: GraphicsStyle::Unicode,
                graphics_color: GraphicsColor::Fullcolor,
                graphics_color_board: GraphicsColor::Fullcolor,
                save_data_on_exit: false,
            },
            game_config: GameConfig::default(),
            new_game_settings: NewGameSettings {
                initial_gravity: 1,
                increase_gravity: false,
                custom_mode_limit: None,
                cheese_mode_limit: Some(NonZeroUsize::try_from(20).unwrap()),
                cheese_mode_gap_size: 1,
                cheese_mode_gravity: 1,
                combo_start_layout: game_mods::combo_mode::LAYOUTS[0],
                descent_mode: false,
                custom_start_board: None,
            },
            combo_bot_enabled: false,
            past_games: vec![],
        };
        if app.load_savefile().is_err() {
            // FIXME: Make this debuggable.
            //eprintln!("Could not loading settings: {e}");
            //std::thread::sleep(Duration::from_secs(5));
        }
        if let Some(combo_start_layout) = combo_start_layout {
            app.new_game_settings.combo_start_layout = combo_start_layout;
        }
        if custom_start_board.is_some() {
            app.new_game_settings.custom_start_board = custom_start_board;
        }
        app.combo_bot_enabled = combo_bot_enabled;
        app.game_config.no_soft_drop_lock = !kitty_detected;
        app
    }

    fn savefile_path() -> PathBuf {
        let home_var = env::var("HOME");
        #[allow(clippy::collapsible_else_if)]
        if cfg!(target_os = "windows") {
            if let Ok(appdata_path) = env::var("APPDATA") {
                PathBuf::from(appdata_path)
            } else {
                PathBuf::from(".")
            }
        } else if cfg!(target_os = "linux") {
            if let Ok(home_path) = home_var {
                PathBuf::from(home_path).join(".config")
            } else {
                PathBuf::from(".")
            }
        } else if cfg!(target_os = "macos") {
            if let Ok(home_path) = home_var {
                PathBuf::from(home_path).join("Library/Application Support")
            } else {
                PathBuf::from(".")
            }
        } else {
            if let Ok(home_path) = home_var {
                PathBuf::from(home_path)
            } else {
                PathBuf::from(".")
            }
        }
        .join(Self::SAVEFILE_NAME)
    }

    fn store_savefile(&mut self, path: PathBuf) -> io::Result<()> {
        self.past_games = self
            .past_games
            .iter()
            .filter(|finished_game_stats| {
                finished_game_stats.was_successful()
                    || finished_game_stats.last_state.lines_cleared
                        > if finished_game_stats.gamemode.name == "Combo"
                            || finished_game_stats.gamemode.name == "Combo (Bot)"
                        {
                            9
                        } else {
                            0
                        }
            })
            .cloned()
            .collect::<Vec<_>>();
        let save_state = (
            &self.settings,
            &self.new_game_settings,
            &self.game_config,
            &self.past_games,
        );
        let save_str = serde_json::to_string(&save_state)?;
        let mut file = File::create(path)?;
        // FIXME: Handle error?
        let _ = file.write(save_str.as_bytes())?;
        Ok(())
    }

    fn load_savefile(&mut self) -> io::Result<()> {
        let mut file = File::open(Self::savefile_path())?;
        let mut save_str = String::new();
        file.read_to_string(&mut save_str)?;
        let save_state = serde_json::from_str(&save_str)?;
        (
            self.settings,
            self.new_game_settings,
            self.game_config,
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
                Menu::ChangeKeybinds => self.change_keybinds_menu(),
                Menu::ConfigureGameplay => self.configure_gameplay_menu(),
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
                        format!("[ {} ]", current_menu_name)
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
                                format!(">>> {name} <<<")
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
                        format!("{:^w_main$}", "Use [←] [→] [↑] [↓] [Esc] [Enter].",).italic(),
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
                    code: KeyCode::Esc,
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
                    code: KeyCode::Up,
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
                    code: KeyCode::Down,
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
            Menu::Quit("quit from title menu. Have a nice day!".to_string()),
        ];
        self.generic_placeholder_menu("", selection)
    }

    fn new_game_menu(&mut self) -> io::Result<MenuUpdate> {
        let normal_gamemodes: [(_, _, Box<dyn Fn() -> Game>); 4] = [
            (
                "40-Lines",
                "how fast can you clear forty lines?".to_string(),
                Box::new(|| Game::new(GameMode::sprint(3))),
            ),
            (
                "Marathon",
                "can you make it to gravity level 15?".to_string(),
                Box::new(|| Game::new(GameMode::marathon())),
            ),
            (
                "Time Trial",
                "get a highscore in 3 minutes!".to_string(),
                Box::new(|| Game::new(GameMode::ultra(1))),
            ),
            (
                "Master",
                "clear 30 levels starting at instant gravity.".to_string(),
                Box::new(|| Game::new(GameMode::master())),
            ),
        ];
        let mut selected = 0usize;
        let mut customization_selected = 0usize;
        let (d_time, d_score, d_pieces, d_lines, d_gravity) =
            (Duration::from_secs(5), 200, 10, 5, 1);
        loop {
            // First part: rendering the menu.
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            let ng = &mut self.new_game_settings;
            let mut special_gamemodes: Vec<(_, _, Box<dyn Fn() -> Game>)> = vec![
                (
                    "Puzzle",
                    "24 stages of perfect clears!".to_string(),
                    Box::new(game_mods::puzzle_mode::new_game),
                ),
                (
                    "Cheese",
                    format!("eat your way through! (limit: {:?})", ng.cheese_mode_limit),
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
                        "how long can you chain? (start: {:b})",
                        ng.combo_start_layout
                    ),
                    Box::new({
                        let combo_start_layout = ng.combo_start_layout;
                        let combo_bot_enabled = self.combo_bot_enabled;
                        move || {
                            let mut combo_game =
                                game_mods::combo_mode::new_game(1, combo_start_layout);
                            if combo_bot_enabled {
                                combo_game.mode_mut().name.push_str(" (Bot)");
                            }
                            combo_game
                        }
                    }),
                ),
            ];
            if ng.descent_mode {
                special_gamemodes.insert(
                    1,
                    (
                        "Descent",
                        "spin the piece and collect gems by touching them.".to_string(),
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
                .queue(Print(format!("{:^w_main$}", "* Start New Game *")))?
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
                            format!(">>> {name}: {details} <<<")
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
                            "  | Custom: (repeat [→] to toggle 'limit')"
                        } else if ng.custom_start_board.is_some() {
                            ">>> Custom: (press [del] to clear start)  "
                        } else {
                            ">>> Custom:                               "
                        }
                    } else {
                        "| Custom"
                    }
                )))?;
            // Render custom mode stuff.
            if selected == selection_size - 1 {
                let stats_strs = [
                    format!("| start gravity: {}", ng.initial_gravity),
                    format!("| gravity increase: {}", ng.increase_gravity),
                    format!("| limit: {:?}", ng.custom_mode_limit),
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
                        "app exited with ctrl-c".to_string(),
                    )))
                }
                // Exit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
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
                        let mut custom_game = Game::new(GameMode {
                            name: "Custom Mode".to_string(),
                            initial_gravity: ng.initial_gravity,
                            increase_gravity: ng.increase_gravity,
                            limits,
                        });
                        if let Some(ref custom_start_board_str) = ng.custom_start_board {
                            custom_game.add_modifier(game_mods::utils::custom_start_board(
                                custom_start_board_str,
                            ));
                        }
                        custom_game
                    };
                    // Set config.
                    game.config_mut().clone_from(&self.game_config);
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
                    code: KeyCode::Up,
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
                    code: KeyCode::Down,
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
                    code: KeyCode::Left,
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
                    code: KeyCode::Right,
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
                    // If custom gamemode selected, allow deleting custom start board.
                    if selected == selection_size - 1 {
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
        let mut combo_bot_handler = (game.mode().name == "Combo (Bot)")
            .then(|| ComboBotHandler::new(&button_sender, Duration::from_millis(100)));
        let mut inform_combo_bot = |game: &Game, evts: &FeedbackEvents| {
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
                    Ok(Err(Interrupt::ExitProgram)) => {
                        self.store_game(game, running_game_stats);
                        break 'render MenuUpdate::Push(Menu::Quit(
                            "exited with ctrl-c".to_string(),
                        ));
                    }
                    Ok(Err(Interrupt::ForfeitGame)) => {
                        game.forfeit();
                        let finished_game_stats = self.store_game(game, running_game_stats);
                        break 'render MenuUpdate::Push(Menu::GameOver(Box::new(
                            finished_game_stats,
                        )));
                    }
                    Ok(Err(Interrupt::Pause)) => {
                        *last_paused = Instant::now();
                        break 'render MenuUpdate::Push(Menu::Pause);
                    }
                    Ok(Err(Interrupt::WindowResize)) => {
                        clean_screen = true;
                        continue 'frame_idle;
                    }
                    Ok(Ok((instant, button, button_state))) => {
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
            time: game_time,
            end: _,
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
        if gamemode.name == "Puzzle" && success {
            self.new_game_settings.descent_mode = true;
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
                            "+ Game Completed! [{}] +",
                            gamemode.name.to_ascii_uppercase()
                        )
                    } else {
                        format!(
                            "- Game Over ({:?}). [{}] -",
                            last_state.end.unwrap().unwrap_err(),
                            gamemode.name
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
                            format!(">>> {name} <<<")
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
                    code: KeyCode::Esc,
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
                    code: KeyCode::Up,
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !selection.is_empty() {
                        selected += selection.len() - 1;
                    }
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
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
        self.generic_placeholder_menu("GAME PAUSED", selection)
    }

    fn settings_menu(&mut self) -> io::Result<MenuUpdate> {
        let selection_len = 7 + 1; // `+1` for hacky empty line.
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
                "Change Keybinds ...".to_string(),
                "Configure Gameplay ...".to_string(),
                format!("graphics : '{:?}'", self.settings.graphics_style),
                format!("colors : '{:?}'", self.settings.graphics_color),
                format!("framerate : {}", self.settings.game_fps),
                format!("show fps : {}", self.settings.show_fps),
                "".to_string(),
                if self.settings.save_data_on_exit {
                    "keep save file for tetrs : ON"
                } else {
                    "keep save file for tetrs : OFF*"
                }
                .to_string(),
                if self.settings.save_data_on_exit {
                    format!("(save file: {:?})", Self::savefile_path())
                } else {
                    "(*WARNING - data will be lost on exit.)".to_string()
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
                            format!(">>> {label} <<<")
                        } else {
                            label
                        }
                    )))?;
            }
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(selection_len + 1).unwrap() + 3,
                ))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "Use [←] [→] [↑] [↓] [Esc] [Enter].",).italic(),
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
                    code: KeyCode::Esc,
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: Press,
                    ..
                }) => match selected {
                    0 => break Ok(MenuUpdate::Push(Menu::ChangeKeybinds)),
                    1 => break Ok(MenuUpdate::Push(Menu::ConfigureGameplay)),
                    _ => {}
                },
                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 7 {
                        // Skip hacky empty line.
                        selected += selection_len - 2;
                    } else {
                        selected += selection_len - 1;
                    }
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 5 {
                        // Skip hacky empty line.
                        selected += 2;
                    } else {
                        selected += 1;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    2 => {
                        self.settings.graphics_style = match self.settings.graphics_style {
                            GraphicsStyle::Electronika60 => GraphicsStyle::ASCII,
                            GraphicsStyle::ASCII => GraphicsStyle::Unicode,
                            GraphicsStyle::Unicode => GraphicsStyle::Electronika60,
                        };
                    }
                    3 => {
                        self.settings.graphics_color = match self.settings.graphics_color {
                            GraphicsColor::Monochrome => GraphicsColor::Color16,
                            GraphicsColor::Color16 => GraphicsColor::Fullcolor,
                            GraphicsColor::Fullcolor => GraphicsColor::Experimental,
                            GraphicsColor::Experimental => GraphicsColor::Monochrome,
                        };
                        self.settings.graphics_color_board = self.settings.graphics_color;
                    }
                    4 => {
                        self.settings.game_fps += 1.0;
                    }
                    5 => {
                        self.settings.show_fps = !self.settings.show_fps;
                    }
                    6 => {} // Hacky empty line.
                    7 => {
                        self.settings.save_data_on_exit = !self.settings.save_data_on_exit;
                    }
                    _ => {}
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    2 => {
                        self.settings.graphics_style = match self.settings.graphics_style {
                            GraphicsStyle::Electronika60 => GraphicsStyle::Unicode,
                            GraphicsStyle::ASCII => GraphicsStyle::Electronika60,
                            GraphicsStyle::Unicode => GraphicsStyle::ASCII,
                        };
                    }
                    3 => {
                        self.settings.graphics_color = match self.settings.graphics_color {
                            GraphicsColor::Monochrome => GraphicsColor::Experimental,
                            GraphicsColor::Color16 => GraphicsColor::Monochrome,
                            GraphicsColor::Fullcolor => GraphicsColor::Color16,
                            GraphicsColor::Experimental => GraphicsColor::Fullcolor,
                        };
                        self.settings.graphics_color_board = self.settings.graphics_color;
                    }
                    4 => {
                        if self.settings.game_fps >= 1.0 {
                            self.settings.game_fps -= 1.0;
                        }
                    }
                    5 => {
                        self.settings.show_fps = !self.settings.show_fps;
                    }
                    6 => {} // Hacky empty line.
                    7 => {
                        self.settings.save_data_on_exit = !self.settings.save_data_on_exit;
                    }
                    _ => {}
                },
                // Other event: Just ignore.
                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }

    fn change_keybinds_menu(&mut self) -> io::Result<MenuUpdate> {
        let button_selection = [
            Button::MoveLeft,
            Button::MoveRight,
            Button::RotateLeft,
            Button::RotateRight,
            Button::RotateAround,
            Button::DropSoft,
            Button::DropHard,
            Button::DropSonic,
            Button::HoldPiece,
        ];
        let selection_len = button_selection.len() + 1;
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "| Change Keybinds |")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let button_names = button_selection
                .iter()
                .map(|&button| {
                    format!(
                        "{button:?} : {}",
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
                            format!(">>> {name} <<<")
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
                        ">>> Restore Defaults <<<"
                    } else {
                        "Restore Defaults"
                    }
                )))?
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(selection_len).unwrap() + 3,
                ))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "Press [Enter] to add keybinds.",).italic(),
                ))?
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(selection_len).unwrap() + 4,
                ))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "Press [Delete] to remove keybinds.",).italic(),
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
                    code: KeyCode::Esc,
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
                    code: KeyCode::Up,
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += selection_len - 1;
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
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
                    "= Configure Gameplay (requires New Game) ="
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let labels = [
                format!("rotation system : {:?}", self.game_config.rotation_system),
                format!(
                    "piece generator : {}",
                    match &self.game_config.tetromino_generator {
                        TetrominoSource::Uniform => "Uniform".to_string(),
                        TetrominoSource::Stock { .. } => "Bag (Stock)".to_string(),
                        TetrominoSource::Recency { .. } => "Recency-based".to_string(),
                        TetrominoSource::BalanceRelative { .. } =>
                            "Balance Relative Counts".to_string(),
                        TetrominoSource::Cycle { pattern, index: _ } =>
                            format!("Cycle Pattern {pattern:?}"),
                    }
                ),
                format!("preview count : {}", self.game_config.preview_count),
                format!(
                    "*delayed auto shift : {:?}",
                    self.game_config.delayed_auto_shift
                ),
                format!(
                    "*auto repeat rate : {:?}",
                    self.game_config.auto_repeat_rate
                ),
                format!("*soft drop factor : {}", self.game_config.soft_drop_factor),
                format!("hard drop delay : {:?}", self.game_config.hard_drop_delay),
                format!("ground time max : {:?}", self.game_config.ground_time_max),
                format!("line clear delay : {:?}", self.game_config.line_clear_delay),
                format!("appearance delay : {:?}", self.game_config.appearance_delay),
                format!(
                    "**no soft drop lock : {}",
                    self.game_config.no_soft_drop_lock
                ),
                format!(
                    "*assume enhanced key events (current game) : {}",
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
                            format!(">>> {label} <<<")
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
                        ">>> Restore Defaults <<<"
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
                        "(*should work - enhanced key events seemed available)"
                    } else {
                        "(*might NOT work - enhanced key events seemed unavailable)"
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
                        "(**were set to 'false' because enhanced key events seemed unavailable)"
                    } else {
                        "(**were set to 'true' because enhanced key events seemed available)"
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
                    code: KeyCode::Esc,
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: Press,
                    ..
                }) => {
                    if selected == selection_len - 1 {
                        self.game_config = GameConfig::default();
                        self.game_config.no_soft_drop_lock = !self.kitty_detected;
                        self.kitty_assumed = self.kitty_detected;
                    }
                }
                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += selection_len - 1;
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += 1;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.game_config.rotation_system = match self.game_config.rotation_system {
                            RotationSystem::Ocular => RotationSystem::Classic,
                            RotationSystem::Classic => RotationSystem::Super,
                            RotationSystem::Super => RotationSystem::Ocular,
                        };
                    }
                    1 => {
                        self.game_config.tetromino_generator = match self
                            .game_config
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
                        self.game_config.preview_count += 1;
                    }
                    3 => {
                        self.game_config.delayed_auto_shift += Duration::from_millis(1);
                    }
                    4 => {
                        self.game_config.auto_repeat_rate += Duration::from_millis(1);
                    }
                    5 => {
                        self.game_config.soft_drop_factor += 0.5;
                    }
                    6 => {
                        self.game_config.hard_drop_delay += Duration::from_millis(1);
                    }
                    7 => {
                        self.game_config.ground_time_max += Duration::from_millis(250);
                    }
                    8 => {
                        self.game_config.line_clear_delay += Duration::from_millis(10);
                    }
                    9 => {
                        self.game_config.appearance_delay += Duration::from_millis(10);
                    }
                    10 => {
                        self.game_config.no_soft_drop_lock = !self.game_config.no_soft_drop_lock;
                    }
                    11 => {
                        self.kitty_assumed = !self.kitty_assumed;
                    }
                    _ => {}
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.game_config.rotation_system = match self.game_config.rotation_system {
                            RotationSystem::Ocular => RotationSystem::Classic,
                            RotationSystem::Classic => RotationSystem::Super,
                            RotationSystem::Super => RotationSystem::Ocular,
                        };
                    }
                    1 => {
                        self.game_config.tetromino_generator = match self
                            .game_config
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
                        self.game_config.preview_count =
                            self.game_config.preview_count.saturating_sub(1);
                    }
                    3 => {
                        self.game_config.delayed_auto_shift = self
                            .game_config
                            .delayed_auto_shift
                            .saturating_sub(Duration::from_millis(1));
                    }
                    4 => {
                        self.game_config.auto_repeat_rate = self
                            .game_config
                            .auto_repeat_rate
                            .saturating_sub(Duration::from_millis(1));
                    }
                    5 => {
                        if self.game_config.soft_drop_factor > 0.0 {
                            self.game_config.soft_drop_factor -= 0.5;
                        }
                    }
                    6 => {
                        if self.game_config.hard_drop_delay >= Duration::from_millis(1) {
                            self.game_config.hard_drop_delay = self
                                .game_config
                                .hard_drop_delay
                                .saturating_sub(Duration::from_millis(1));
                        }
                    }
                    7 => {
                        self.game_config.ground_time_max = self
                            .game_config
                            .ground_time_max
                            .saturating_sub(Duration::from_millis(250));
                    }
                    8 => {
                        self.game_config.line_clear_delay = self
                            .game_config
                            .line_clear_delay
                            .saturating_sub(Duration::from_millis(10));
                    }
                    9 => {
                        self.game_config.appearance_delay = self
                            .game_config
                            .appearance_delay
                            .saturating_sub(Duration::from_millis(10));
                    }
                    10 => {
                        self.game_config.no_soft_drop_lock = !self.game_config.no_soft_drop_lock;
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
                .queue(Print(format!("{:^w_main$}", "# Scoreboard #")))?
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
                        match gamemode.name.as_str() {
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
                            "Combo (Bot)" => {
                                format!(
                                    "{timestamp} ~ Combo (Bot): {} lns",
                                    last_state.lines_cleared
                                )
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
                    code: KeyCode::Esc,
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    kind: Press | Repeat,
                    ..
                }) => {
                    scroll = scroll.saturating_sub(1);
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
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
                        match stats1.gamemode.name.as_str() {
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
