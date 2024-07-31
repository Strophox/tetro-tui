use std::{
    collections::HashMap,
    env,
    fmt::Debug,
    fs::File,
    io::{self, Read, Write},
    num::NonZeroU32,
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
    terminal::{self, Clear},
    ExecutableCommand, QueueableCommand,
};
use tetrs_engine::{Button, ButtonsPressed, Game, GameConfig, GameMode, GameState, Limits, RotationSystem, TetrominoGenerator};

use crate::game_renderers::{cached::Renderer, GameScreenRenderer};
use crate::{
    game_input_handler::{ButtonOrSignal, CrosstermHandler, Signal},
    puzzle_mode,
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
        game_renderer: Box<Renderer>,
    },
    GameOver(Box<FinishedGameStats>),
    GameComplete(Box<FinishedGameStats>),
    Pause,
    Settings,
    ChangeControls,
    ConfigureGame,
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
            Menu::ChangeControls => "Change Controls",
            Menu::ConfigureGame => "Configure Game",
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
    ASCII,
    Unicode,
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum GraphicsColor {
    Monochrome,
    Color16,
    ColorRGB,
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
    Level(NonZeroU32),
    Score(u32),
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct CustomModeSettings {
    name: String,
    start_level: NonZeroU32,
    increment_level: bool,
    mode_limit: Option<Stat>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct App<T: Write> {
    pub term: T,
    kitty_enabled: bool,
    settings: Settings,
    custom_mode_settings: CustomModeSettings,
    game_config: GameConfig,
    past_games: Vec<FinishedGameStats>,
}

impl<T: Write> Drop for App<T> {
    fn drop(&mut self) {
        // TODO: Handle errors?
        let savefile_path = Self::savefile_path();
        // If the user wants their data stored, try to do so.
        if self.settings.save_data_on_exit {
            if let Err(_e) = self.store_local(savefile_path) {
                // TODO: Make this debuggable.
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
        // Console epilogue: de-initialization.
        if self.kitty_enabled {
            let _ = self.term.execute(event::PopKeyboardEnhancementFlags);
        }
        let _ = terminal::disable_raw_mode();
        let _ = self.term.execute(style::ResetColor);
        let _ = self.term.execute(cursor::Show);
        let _ = self.term.execute(terminal::LeaveAlternateScreen);
    }
}

impl<T: Write> App<T> {
    pub const W_MAIN: u16 = 80;
    pub const H_MAIN: u16 = 24;

    pub const SAVEFILE_NAME: &'static str = ".tetrs_terminal.json";

    pub fn new(mut terminal: T, fps: Option<u32>) -> Self {
        // Console prologue: Initializion.
        // TODO: Handle errors?
        let _ = terminal.execute(terminal::EnterAlternateScreen);
        let _ = terminal.execute(terminal::SetTitle("Tetrs Terminal"));
        let _ = terminal.execute(cursor::Hide);
        let _ = terminal::enable_raw_mode();
        let kitty_enabled = terminal::supports_keyboard_enhancement().unwrap_or(false);
        if kitty_enabled {
            // TODO: Kinda iffy. Do we need all flags? What undesirable effects might there be?
            let _ = terminal.execute(event::PushKeyboardEnhancementFlags(
                event::KeyboardEnhancementFlags::all(),
            ));
        }
        let mut app = Self {
            term: terminal,
            settings: Settings {
                keybinds: CrosstermHandler::default_keybinds(),
                game_fps: 30.0,
                show_fps: false,
                graphics_style: GraphicsStyle::Unicode,
                graphics_color: GraphicsColor::ColorRGB,
                save_data_on_exit: false,
            },
            custom_mode_settings: CustomModeSettings {
                name: "Custom Mode".to_string(),
                start_level: NonZeroU32::MIN,
                increment_level: true,
                mode_limit: Some(Stat::Time(Duration::from_secs(60))),
            },
            game_config: GameConfig::default(),
            past_games: vec![],
            kitty_enabled,
        };
        if let Err(_e) = app.load_local() {
            // TODO: Make this debuggable.
            //eprintln!("Could not loading settings: {e}");
            //std::thread::sleep(Duration::from_secs(5));
        }
        if let Some(game_fps) = fps {
            app.settings.game_fps = game_fps.into();
        }
        app.game_config.no_soft_drop_lock = !kitty_enabled;
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

    fn store_local(&mut self, path: PathBuf) -> io::Result<()> {
        self.past_games = self
            .past_games
            .iter()
            .filter(|finished_game_stats| {
                finished_game_stats.was_successful()
                    || finished_game_stats.last_state.lines_cleared > 0
            })
            .cloned()
            .collect::<Vec<_>>();
        let save_state = (&self.settings, &self.custom_mode_settings, &self.past_games);
        let save_str = serde_json::to_string(&save_state)?;
        let mut file = File::create(path)?;
        // TODO: Handle error?
        let _ = file.write(save_str.as_bytes())?;
        Ok(())
    }

    fn load_local(&mut self) -> io::Result<()> {
        let mut file = File::open(Self::savefile_path())?;
        let mut save_str = String::new();
        file.read_to_string(&mut save_str)?;
        (self.settings, self.custom_mode_settings, self.past_games) =
            serde_json::from_str(&save_str)?;
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
                Menu::Title => self.title(),
                Menu::NewGame => self.newgame(),
                Menu::Game {
                    game,
                    time_started,
                    total_duration_paused,
                    last_paused,
                    running_game_stats,
                    game_renderer,
                } => self.game(
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
                Menu::ChangeControls => self.change_controls_menu(),
                Menu::ConfigureGame => self.configure_game_menu(),
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

    fn generic_placeholder_widget(
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
                    .queue(terminal::Clear(terminal::ClearType::All))?
                    .queue(MoveTo(x_main, y_main + y_selection))?
                    .queue(Print(format!("{:^w_main$}", "▀█▀ ██ ▀█▀ █▀▀ ▄█▀")))?
                    .queue(MoveTo(x_main, y_main + y_selection + 1))?
                    .queue(Print(format!("{:^w_main$}", "    █▄▄▄▄▄▄       ")))?;
            } else {
                self.term
                    .queue(terminal::Clear(terminal::ClearType::All))?
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
                    .queue(Clear(terminal::ClearType::All))?
                    .queue(MoveTo(0, y_main))?
                    .queue(PrintStyledContent(Self::DAVIS.italic()))?;
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

    fn title(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::About,
            Menu::Quit("quit from title menu. Have a nice day!".to_string()),
        ];
        self.generic_placeholder_widget("", selection)
    }

    fn newgame(&mut self) -> io::Result<MenuUpdate> {
        let preset_gamemodes = [
            GameMode::marathon(),
            GameMode::sprint(NonZeroU32::try_from(3).unwrap()),
            GameMode::ultra(NonZeroU32::try_from(3).unwrap()),
            GameMode::master(),
        ];
        let (d_time, d_score, d_pieces, d_lines, d_level) = (Duration::from_secs(5), 200, 10, 5, 1);
        let mut selected = 0usize;
        let mut selected_custom = 0usize;
        // There are the preset gamemodes + custom gamemode.
        let selected_cnt = preset_gamemodes.len() + 2;
        // There are four columns for the custom stat selection.
        let selected_custom_cnt = 4;
        loop {
            // First part: rendering the menu.
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            // Render menu title.
            self.term
                .queue(terminal::Clear(terminal::ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "* Start New Game *")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            // Render preset selection.
            let names = preset_gamemodes
                .iter()
                .cloned()
                .map(|gm| gm.name)
                .collect::<Vec<_>>();
            for (i, name) in names.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + 2 * u16::try_from(i).unwrap(),
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
            // Render puzzle mode option.
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + 2 * u16::try_from(selected_cnt - 2).unwrap(),
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == selected_cnt - 2 {
                        ">>> Puzzle Mode: Spins and perfect clears! <<<"
                    } else {
                        "Puzzle Mode ..."
                    }
                )))?;
            // Render custom mode option.
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + 2 * u16::try_from(selected_cnt - 1).unwrap(),
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == selected_cnt - 1 {
                        if selected_custom == 0 {
                            "▓▓> Custom Mode: (press right repeatedly to change 'limit')"
                        } else {
                            "  > Custom Mode: (press right repeatedly to change 'limit')"
                        }
                    } else {
                        "Custom Mode ..."
                    }
                )))?;
            // Render custom mode stuff.
            if selected == selected_cnt - 1 {
                let stats_strs = [
                    format!("* level start: {}", self.custom_mode_settings.start_level),
                    format!(
                        "* level increment: {}",
                        self.custom_mode_settings.increment_level
                    ),
                    format!("* limit: {:?}", self.custom_mode_settings.mode_limit),
                ];
                for (j, stat_str) in stats_strs.into_iter().enumerate() {
                    self.term
                        .queue(MoveTo(
                            x_main + 16 + 4 * u16::try_from(j).unwrap(),
                            y_main + y_selection + 4 + u16::try_from(j + 2 * selected_cnt).unwrap(),
                        ))?
                        .queue(Print(if j + 1 == selected_custom {
                            format!("▓▓{stat_str}")
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
                    let game = if selected == selected_cnt - 1 {
                        let CustomModeSettings {
                            name,
                            start_level,
                            increment_level,
                            mode_limit: custom_mode_limit,
                        } = self.custom_mode_settings.clone();
                        let limits = match custom_mode_limit {
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
                            Some(Stat::Level(max_lvl)) => Limits {
                                level: Some((true, max_lvl)),
                                ..Default::default()
                            },
                            Some(Stat::Score(max_pts)) => Limits {
                                score: Some((true, max_pts)),
                                ..Default::default()
                            },
                            None => Limits::default(),
                        };
                        Game::new(
                            GameMode {
                                name,
                                start_level,
                                increment_level,
                                limits,
                            },
                        )
                    } else if selected == selected_cnt - 2 {
                        puzzle_mode::make_game()
                    } else {
                        // SAFETY: Index < selected_cnt - 2 = preset_gamemodes.len().
                        Game::new(preset_gamemodes.into_iter().nth(selected).unwrap())
                    };

                    // TODO: Remove or make accessible.
                    // unsafe {
                    //     game.add_modifier(Box::new(
                    //         crate::game_mods::display_tetromino_likelihood_mod,
                    //     ))
                    // };

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
                    if selected_custom > 0 {
                        match selected_custom {
                            1 => {
                                self.custom_mode_settings.start_level = self
                                    .custom_mode_settings
                                    .start_level
                                    .saturating_add(d_level);
                            }
                            2 => {
                                self.custom_mode_settings.increment_level =
                                    !self.custom_mode_settings.increment_level;
                            }
                            3 => {
                                match self.custom_mode_settings.mode_limit {
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
                                    Some(Stat::Level(ref mut lvl)) => {
                                        *lvl = lvl.saturating_add(d_level);
                                    }
                                    None => {}
                                };
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        selected += selected_cnt - 1;
                    }
                }
                // Move selector down or decrease stat.
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    kind: Press | Repeat,
                    ..
                }) => {
                    // Selected custom stat; decrease it.
                    if selected_custom > 0 {
                        match selected_custom {
                            1 => {
                                self.custom_mode_settings.start_level = NonZeroU32::try_from(
                                    self.custom_mode_settings.start_level.get() - d_level,
                                )
                                .unwrap_or(NonZeroU32::MIN);
                            }
                            2 => {
                                self.custom_mode_settings.increment_level =
                                    !self.custom_mode_settings.increment_level;
                            }
                            3 => {
                                match self.custom_mode_settings.mode_limit {
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
                                    Some(Stat::Level(ref mut lvl)) => {
                                        *lvl = NonZeroU32::try_from(lvl.get() - d_level)
                                            .unwrap_or(NonZeroU32::MIN);
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
                    if selected == selected_cnt - 1 && selected_custom > 0 {
                        selected_custom += selected_custom_cnt - 1
                    }
                }
                // Move selector right (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    kind: Press | Repeat,
                    ..
                }) => {
                    // If custom gamemode selected, allow incrementing stat selection.
                    if selected == selected_cnt - 1 {
                        // If reached last stat, cycle through stats for limit.
                        if selected_custom == selected_custom_cnt - 1 {
                            self.custom_mode_settings.mode_limit =
                                match self.custom_mode_settings.mode_limit {
                                    Some(Stat::Time(_)) => Some(Stat::Score(9000)),
                                    Some(Stat::Score(_)) => Some(Stat::Pieces(100)),
                                    Some(Stat::Pieces(_)) => Some(Stat::Lines(40)),
                                    Some(Stat::Lines(_)) => {
                                        Some(Stat::Level(NonZeroU32::try_from(25).unwrap()))
                                    }
                                    Some(Stat::Level(_)) => None,
                                    None => Some(Stat::Time(Duration::from_secs(120))),
                                };
                        } else {
                            selected_custom += 1
                        }
                    }
                }
                // Other event: don't care.
                _ => {}
            }
            selected = selected.rem_euclid(selected_cnt);
            selected_custom = selected_custom.rem_euclid(selected_custom_cnt);
        }
    }

    fn game(
        &mut self,
        game: &mut Game,
        time_started: &mut Instant,
        last_paused: &mut Instant,
        total_duration_paused: &mut Duration,
        running_game_stats: &mut RunningGameStats,
        game_renderer: &mut impl GameScreenRenderer,
    ) -> io::Result<MenuUpdate> {
        // Update rotation system manually.
        game.config_mut().clone_from(&self.game_config);
        // Prepare channel with which to communicate `Button` inputs / game interrupt.
        let mut buttons_pressed = ButtonsPressed::default();
        let (tx, rx) = mpsc::channel::<ButtonOrSignal>();
        let _input_handler =
            CrosstermHandler::new(&tx, &self.settings.keybinds, self.kitty_enabled);
        // Game Loop
        let session_resumed = Instant::now();
        *total_duration_paused += session_resumed.saturating_duration_since(*last_paused);
        let mut clean_screen = true;
        let mut f = 0u32;
        let mut fps_counter = 0;
        let mut fps_counter_started = Instant::now();
        let menu_update = 'render_loop: loop {
            // Exit if game ended
            if game.ended() {
                let finished_game_stats = self.store_game(game, running_game_stats);
                let menu = if finished_game_stats.was_successful() {
                    Menu::GameComplete
                } else {
                    Menu::GameOver
                }(Box::new(finished_game_stats));
                break 'render_loop MenuUpdate::Push(menu);
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
            'idle_loop: loop {
                let frame_idle_remaining = next_frame_at - Instant::now();
                match rx.recv_timeout(frame_idle_remaining) {
                    Ok(Err(Signal::ExitProgram)) => {
                        self.store_game(game, running_game_stats);
                        break 'render_loop MenuUpdate::Push(Menu::Quit(
                            "exited with ctrl-c".to_string(),
                        ));
                    }
                    Ok(Err(Signal::ForfeitGame)) => {
                        game.forfeit();
                        let finished_game_stats = self.store_game(game, running_game_stats);
                        break 'render_loop MenuUpdate::Push(Menu::GameOver(Box::new(
                            finished_game_stats,
                        )));
                    }
                    Ok(Err(Signal::Pause)) => {
                        *last_paused = Instant::now();
                        break 'render_loop MenuUpdate::Push(Menu::Pause);
                    }
                    Ok(Err(Signal::WindowResize)) => {
                        clean_screen = true;
                        continue 'idle_loop;
                    }
                    Ok(Ok((instant, button, button_state))) => {
                        buttons_pressed[button] = button_state;
                        let game_time_userinput = instant.saturating_duration_since(*time_started)
                            - *total_duration_paused;
                        let game_now = std::cmp::max(game_time_userinput, game.state().time);
                        // TODO: Handle/ensure no Err.
                        if let Ok(evts) = game.update(Some(buttons_pressed), game_now) {
                            new_feedback_events.extend(evts);
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let game_time_now = Instant::now().saturating_duration_since(*time_started)
                            - *total_duration_paused;
                        // TODO: Handle/ensure no Err.
                        if let Ok(evts) = game.update(None, game_time_now) {
                            new_feedback_events.extend(evts);
                        }
                        break 'idle_loop;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // NOTE: We kind of rely on this not happening too often.
                        break 'render_loop MenuUpdate::Push(Menu::Pause);
                    }
                };
            }
            game_renderer.render(
                self,
                game,
                running_game_stats,
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
        if let Some(finished_state) = game.state().end {
            let h_console = terminal::size()?.1;
            if finished_state.is_ok() {
                for i in 0..h_console {
                    self.term
                        .execute(MoveTo(0, i))?
                        .execute(Clear(terminal::ClearType::CurrentLine))?;
                    std::thread::sleep(Duration::from_secs_f32(0.01));
                }
            } else {
                for i in (0..h_console).rev() {
                    self.term
                        .execute(MoveTo(0, i))?
                        .execute(Clear(terminal::ClearType::CurrentLine))?;
                    std::thread::sleep(Duration::from_secs_f32(0.01));
                }
            };
        }
        Ok(menu_update)
    }

    fn generic_game_ended(
        &mut self,
        selection: Vec<Menu>,
        success: bool,
        finished_game_stats: &FinishedGameStats,
    ) -> io::Result<MenuUpdate> {
        let FinishedGameStats {
            timestamp: _,
            actions,
            score_bonuses,
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
            next_pieces: _,
            pieces_played,
            lines_cleared,
            level,
            score,
            consecutive_line_clears: _,
            back_to_back_special_clears: _,
        } = last_state;
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
                .queue(Clear(terminal::ClearType::All))?
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
                .queue(Print(format!("{:^w_main$}", format!("Level: {level}",))))?
                .queue(MoveTo(x_main, y_main + y_selection + 6))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Lines: {}", lines_cleared)
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 7))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Tetrominos: {}", pieces_played.iter().sum::<u32>())
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 8))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Time: {}", format_duration(*game_time))
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 10))?
                .queue(Print(format!("{:^w_main$}", actions_str)))?
                .queue(MoveTo(x_main, y_main + y_selection + 11))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!(
                        "Average score bonus: {:.1}",
                        f64::from(score_bonuses.iter().sum::<u32>())
                            / (score_bonuses.len() as f64/*I give up*/)
                    )
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 13))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let names = selection
                .iter()
                .map(|menu| menu.to_string())
                .collect::<Vec<_>>();
            for (i, name) in names.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 14 + u16::try_from(i).unwrap(),
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
        self.generic_game_ended(selection, false, finished_game_stats)
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
        self.generic_game_ended(selection, true, finished_game_stats)
    }

    fn pause_menu(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::About,
            Menu::Quit("quit from pause".to_string()),
        ];
        self.generic_placeholder_widget("GAME PAUSED", selection)
    }

    fn settings_menu(&mut self) -> io::Result<MenuUpdate> {
        let selection_len = 7;
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(terminal::Clear(terminal::ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "% Settings %")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let labels = [
                "| Change Controls .. |".to_string(),
                "| Configure Game .. |".to_string(),
                format!("graphics : '{:?}'", self.settings.graphics_style),
                format!("color : '{:?}'", self.settings.graphics_color),
                format!("framerate : {}", self.settings.game_fps),
                format!("show fps : {}", self.settings.show_fps),
                if self.settings.save_data_on_exit {
                    "Keep savefile for tetrs : On"
                } else {
                    "Keep savefile for tetrs : Off [WARNING: data will be lost on exit!]"
                }
                .to_string(),
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
                    y_main + y_selection + 4 + u16::try_from(selection_len).unwrap() + 2,
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
                }) => {
                    match selected {
                        0 => break Ok(MenuUpdate::Push(Menu::ChangeControls)),
                        1 => break Ok(MenuUpdate::Push(Menu::ConfigureGame)),
                        _ => {},
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
                            GraphicsColor::Color16 => GraphicsColor::ColorRGB,
                            GraphicsColor::ColorRGB => GraphicsColor::Monochrome,
                        };
                    }
                    4 => {
                        self.settings.game_fps += 1.0;
                    }
                    5 => {
                        self.settings.show_fps = !self.settings.show_fps;
                    }
                    6 => {
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
                            GraphicsColor::Monochrome => GraphicsColor::ColorRGB,
                            GraphicsColor::Color16 => GraphicsColor::Monochrome,
                            GraphicsColor::ColorRGB => GraphicsColor::Color16,
                        };
                    }
                    4 => {
                        if self.settings.game_fps >= 1.0 {
                            self.settings.game_fps -= 1.0;
                        }
                    }
                    5 => {
                        self.settings.show_fps = !self.settings.show_fps;
                    }
                    6 => {
                        self.settings.save_data_on_exit = !self.settings.save_data_on_exit;
                    }
                    _ => {}
                },
                // Other event: don't care.
                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }

    fn change_controls_menu(&mut self) -> io::Result<MenuUpdate> {
        let button_selection = [
            Button::MoveLeft,
            Button::MoveRight,
            Button::RotateLeft,
            Button::RotateRight,
            Button::RotateAround,
            Button::DropSoft,
            Button::DropHard,
            Button::DropSonic,
        ];
        let selection_len = button_selection.len() + 1;
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(terminal::Clear(terminal::ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "| Change Controls |")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let button_names = button_selection
                .iter()
                .map(|&button| {
                    format!(
                        "{button:?} : {}",
                        format_keybinds(button, &self.settings.keybinds)
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
                        ">>> [reset keybinds] <<<"
                    } else {
                        "[reset keybinds]"
                    }
                )))?;
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(selection_len).unwrap() + 3,
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        "Press [Enter] to add a keybind to an action.",
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
                            ))?;
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

    fn configure_game_menu(&mut self) -> io::Result<MenuUpdate> {
        let selection_len = 12;
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(terminal::Clear(terminal::ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "& Configure Game (requires new game) &")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let labels = [
                format!("rotation system : {:?}", self.game_config.rotation_system),
                format!("piece generator : {}", match self.game_config.tetromino_generator {
                    TetrominoGenerator::Uniform => "Uniform",
                    TetrominoGenerator::Bag { .. } => "7-Bag",
                    TetrominoGenerator::Recency { .. } => "Recency/History",
                    TetrominoGenerator::TotalRelative { .. } => "Total Relative Counts",
                }),
                format!("preview count : {}", self.game_config.preview_count),
                format!("delayed auto shift** : {:?}", self.game_config.delayed_auto_shift),
                format!("auto repeat rate** : {:?}", self.game_config.auto_repeat_rate),
                format!("soft drop factor** : {}", self.game_config.soft_drop_factor),
                format!("hard drop delay : {:?}", self.game_config.hard_drop_delay),
                format!("ground time max : {:?}", self.game_config.ground_time_max),
                format!("line clear delay : {:?}", self.game_config.line_clear_delay),
                format!("appearance delay : {:?}", self.game_config.appearance_delay),
                format!("no soft drop lock* : {}", self.game_config.no_soft_drop_lock),
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
                        ">>> [restore defaults] <<<"
                    } else {
                        "[restore defaults]"
                    }
                )))?;
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(selection_len - 1).unwrap() + 4,
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}", format!(
                        "(*automatically {} because keyboard enhancements are {}available)",
                        if self.kitty_enabled { "off" } else { "on" },
                        if self.kitty_enabled { "" } else { "UN" }
                    ))))?;
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(selection_len - 1).unwrap() + 5,
                    ))?
                    .queue(Print(format!(
                    "{:^w_main$}",
                    "(**only take effect if keyboard enhancements are available)")))?;
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
                        self.game_config.no_soft_drop_lock = !self.kitty_enabled;
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
                        self.game_config.tetromino_generator = match self.game_config.tetromino_generator {
                            TetrominoGenerator::Uniform => TetrominoGenerator::bag(NonZeroU32::MIN),
                            TetrominoGenerator::Bag { .. } => TetrominoGenerator::recency(),
                            TetrominoGenerator::Recency { .. } => TetrominoGenerator::total_relative(),
                            TetrominoGenerator::TotalRelative { .. } => TetrominoGenerator::uniform(),
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
                        self.game_config.soft_drop_factor += 0.25;
                    }
                    6 => {
                        self.game_config.hard_drop_delay += Duration::from_millis(1);
                    }
                    7 => {
                        self.game_config.ground_time_max += Duration::from_millis(10);
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
                        self.game_config.tetromino_generator = match self.game_config.tetromino_generator {
                            TetrominoGenerator::Uniform => TetrominoGenerator::total_relative(),
                            TetrominoGenerator::Bag { .. } => TetrominoGenerator::uniform(),
                            TetrominoGenerator::Recency { .. } => TetrominoGenerator::bag(NonZeroU32::MIN),
                            TetrominoGenerator::TotalRelative { .. } => TetrominoGenerator::recency(),
                        };
                    }
                    2 => {
                        self.game_config.preview_count = self.game_config.preview_count.saturating_sub(1);
                    }
                    3 => {
                        self.game_config.delayed_auto_shift = self.game_config.delayed_auto_shift.saturating_sub(Duration::from_millis(1));
                    }
                    4 => {
                        self.game_config.auto_repeat_rate = self.game_config.auto_repeat_rate.saturating_sub(Duration::from_millis(1));
                    }
                    5 => {
                        if self.game_config.soft_drop_factor > 0.0 {
                            self.game_config.soft_drop_factor -= 0.25;
                        }
                    }
                    6 => {
                        if self.game_config.hard_drop_delay >= Duration::from_millis(1) {
                            self.game_config.hard_drop_delay = self.game_config.hard_drop_delay.saturating_sub(Duration::from_millis(1));
                        }
                    }
                    7 => {
                        self.game_config.ground_time_max = self.game_config.ground_time_max.saturating_sub(Duration::from_millis(10));
                    }
                    8 => {
                        self.game_config.line_clear_delay = self.game_config.line_clear_delay.saturating_sub(Duration::from_millis(10));
                    }
                    9 => {
                        self.game_config.appearance_delay = self.game_config.appearance_delay.saturating_sub(Duration::from_millis(10));
                    }
                    10 => {
                        self.game_config.no_soft_drop_lock = !self.game_config.no_soft_drop_lock;
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
        let max_entries = 16;
        let mut scroll = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(terminal::Clear(terminal::ClearType::All))?
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
                                            level: Some((_, max_lvl)),
                                            ..
                                        } = gamemode.limits
                                        else {
                                            panic!()
                                        };
                                        format!(" ({}/{} lvl)", last_state.level, max_lvl)
                                    },
                                )
                            }
                            "40-Lines" => {
                                format!(
                                    "{timestamp} ~ 40-Lines: {}{}",
                                    format_duration(last_state.time),
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
                                    "{timestamp} ~ Time Trial: {} lns{}",
                                    last_state.lines_cleared,
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
                                            format_duration(last_state.time),
                                            format_duration(max_dur)
                                        )
                                    },
                                )
                            }
                            "Master" => {
                                let Limits {
                                    lines: Some((_, max_lns)),
                                    ..
                                } = gamemode.limits
                                else {
                                    panic!()
                                };
                                format!(
                                    "{timestamp} ~ Master: {}/{} lns",
                                    last_state.lines_cleared, max_lns
                                )
                            }
                            "Puzzle" => {
                                format!(
                                    "{timestamp} ~ Puzzle Mode: {}{}",
                                    format_duration(last_state.time),
                                    if last_state.end.is_some_and(|end| end.is_ok()) {
                                        "".to_string()
                                    } else {
                                        let Limits {
                                            level: Some((_, max_lvl)),
                                            ..
                                        } = gamemode.limits
                                        else {
                                            panic!()
                                        };
                                        format!(" ({}/{} lvl)", last_state.level, max_lvl)
                                    },
                                )
                            }
                            _ => {
                                format!(
                                    "{timestamp} ~ Custom Mode: {} lns, {} pts, {}{}",
                                    last_state.lines_cleared,
                                    last_state.score,
                                    format_duration(last_state.time),
                                    [
                                        gamemode.limits.time.map(|(_, max_dur)| format!(
                                            " ({} / {})",
                                            format_duration(last_state.time),
                                            format_duration(max_dur)
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
                                        gamemode.limits.level.map(|(_, max_lvl)| format!(
                                            " ({}/{} lvl)",
                                            last_state.level, max_lvl
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
        /* TODO: About menu. */
        self.generic_placeholder_widget(
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
                                stats1.last_state.level.cmp(&stats2.last_state.level).reverse().then_with(||
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
                                    // Sort by desc lines.
                                    stats1.last_state.lines_cleared.cmp(&stats2.last_state.lines_cleared).reverse()
                                )
                            },
                            "Master" => {
                                // Sort desc by lines.
                                stats1.last_state.lines_cleared.cmp(&stats2.last_state.lines_cleared).reverse()
                            },
                            "Puzzle" => {
                                // Sort desc by level.
                                stats1.last_state.level.cmp(&stats2.last_state.level).reverse().then_with(||
                                    // Sort asc by time.
                                    stats1.last_state.time.cmp(&stats2.last_state.time)
                                )
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

    const DAVIS: &'static str = " ▀█▀ \"I am like Solomon because I built God's temple, an operating system. God said 640x480 16 color graphics but the operating system is 64-bit and multi-cored! Go draw a 16 color elephant. Then, draw a 24-bit elephant in MS Paint and be enlightened. Artist stopped photorealism when the camera was invented. A cartoon is actually better than photorealistic. For the next thousand years, first-person shooters are going to get boring. Tetris looks good.\" - In memory of Terry A. Davis";
}

pub fn format_duration(dur: Duration) -> String {
    format!(
        "{}min {}.{:02}sec",
        dur.as_secs() / 60,
        dur.as_secs() % 60,
        dur.as_millis() % 1000 / 10
    )
}

pub fn format_key(key: KeyCode) -> String {
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

pub fn format_keybinds(button: Button, keybinds: &HashMap<KeyCode, Button>) -> String {
    keybinds
        .iter()
        .filter_map(|(&k, &b)| (b == button).then_some(format_key(k)))
        .collect::<Vec<String>>()
        .join(" ")
}
