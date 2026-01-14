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
    piece_generation::TetrominoSource, piece_rotation::RotationSystem, Button, FeedbackMessages,
    Game, GameConfig, GameMode, GameState, Limits, PressedButtons, Tetromino,
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
        cached_renderer::CachedRenderer, color16_palette, empty_palette, experimental_palette,
        fullcolor_palette, tet_str_small, Palette, Renderer,
    },
};

pub type Slots<T> = Vec<(String, T)>;

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FinishedGameStats {
    timestamp: String,
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
        game_renderer: CachedRenderer,
    },
    GameOver(Box<FinishedGameStats>),
    GameComplete(Box<FinishedGameStats>),
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
            Menu::Game { .. } => "Game", //&format!("Game {}", game.mode().name.as_ref().map_or("".to_string(), |ref name| format!("({name})"))),
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
    custom_initial_gravity: u32,
    custom_increase_gravity: bool,
    custom_mode_limit: Option<Stat>,
    cheese_mode_linelimit: Option<NonZeroUsize>,
    cheese_mode_gap_size: usize,
    cheese_mode_gravity: u32,
    combo_mode_linelimit: Option<NonZeroUsize>,
    combo_start_layout: u16,
    experimental_mode_unlocked: bool,
    /// Custom starting layout when playing Combo mode (4-wide rows), encoded as binary.
    /// Example: '▀▄▄▀' => 0b_1001_0110 = 150
    custom_start_board: Option<String>,
    // TODO: Placeholder for proper snapshot functionality.
    custom_start_seed: Option<u64>,
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
    game_fps: f64,
    show_fps: bool,
    pub glyphset: Glyphset,
    palette_active: usize,
    palette_active_lockedtiles: usize,
    pub render_effects: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            game_fps: 30.0,
            show_fps: false,
            glyphset: Glyphset::Unicode,
            palette_active: 2,
            palette_active_lockedtiles: 2,
            render_effects: true,
        }
    }
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum SavefileGranularity {
    Nothing,
    Settings,
    Everything,
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
    config_slots: Slots<GameConfig>,
    config_slots_that_should_not_be_changed: usize,
    config_active: usize,
    new_game: NewGameSettings,
    save_on_exit: SavefileGranularity,
}

impl Default for Settings {
    fn default() -> Self {
        let graphics_slots = vec![
            ("default".to_string(), GraphicsSettings::default()),
            (
                "high focus".to_string(),
                GraphicsSettings {
                    palette_active_lockedtiles: 0,
                    render_effects: false,
                    game_fps: 60.0,
                    ..GraphicsSettings::default()
                },
            ),
        ];
        let palette_slots = vec![
            ("Monochrome".to_string(), empty_palette()), // NOTE: The slot at index 0 is the special 'monochrome'/no palette slot.
            ("16-color".to_string(), color16_palette()),
            ("Fullcolor".to_string(), fullcolor_palette()),
            ("Experimental/Custom".to_string(), experimental_palette()),
        ];
        let keybinds_slots = vec![
            ("tetrs default".to_string(), tetrs_default_keybinds()),
            ("Vim-like".to_string(), vim_keybinds()),
            ("TTC default".to_string(), guideline_keybinds()),
        ];
        let config_slots = vec![
            ("default".to_string(), GameConfig::default()),
            (
                "high finesse".to_string(),
                GameConfig {
                    preview_count: 9,
                    delayed_auto_shift: Duration::from_millis(110),
                    auto_repeat_rate: Duration::from_millis(0),
                    ..GameConfig::default()
                },
            ),
        ];
        let new_game = NewGameSettings {
            custom_initial_gravity: 1,
            custom_increase_gravity: true,
            custom_start_board: None,
            custom_start_seed: None,
            custom_mode_limit: None,
            cheese_mode_linelimit: Some(NonZeroUsize::try_from(20).unwrap()),
            cheese_mode_gravity: 0,
            cheese_mode_gap_size: 1,
            combo_mode_linelimit: Some(NonZeroUsize::try_from(20).unwrap()),
            combo_start_layout: game_modifiers::combo_mode::LAYOUTS[0],
            experimental_mode_unlocked: false,
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
            new_game,
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
    pub fn config(&self) -> &GameConfig {
        &self.config_slots[self.config_active].1
    }
    fn graphics_mut(&mut self) -> &mut GraphicsSettings {
        &mut self.graphics_slots[self.graphics_active].1
    }
    fn keybinds_mut(&mut self) -> &mut Keybinds {
        &mut self.keybinds_slots[self.keybinds_active].1
    }
    fn config_mut(&mut self) -> &mut GameConfig {
        &mut self.config_slots[self.config_active].1
    }

    pub fn palette(&self) -> &Palette {
        &self.palette_slots[self.graphics().palette_active].1
    }
    pub fn palette_lockedtiles(&self) -> &Palette {
        &self.palette_slots[self.graphics().palette_active_lockedtiles].1
    }
}

#[derive(Clone, Debug)]
pub struct Application<T: Write> {
    pub term: T,
    past_games: Vec<FinishedGameStats>,
    settings: Settings,
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
            app.settings.new_game.custom_start_board = custom_start_board;
        }
        if custom_start_seed.is_some() {
            app.settings.new_game.custom_start_seed = custom_start_seed;
        }
        app.combo_bot_enabled = combo_bot_enabled;
        app.kitty_detected = terminal::supports_keyboard_enhancement().unwrap_or(false);
        app.kitty_assumed = app.kitty_detected;
        app
    }

    fn savefile_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(Self::SAVEFILE_NAME)
    }

    fn store_save(&mut self, path: PathBuf) -> io::Result<()> {
        // Only save past games if needed.
        self.past_games = if self.settings.save_on_exit == SavefileGranularity::Everything {
            self.past_games
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
                Menu::Title => self.menu_title(),
                Menu::NewGame => self.menu_new_game(),
                Menu::Game {
                    game,
                    time_started,
                    total_duration_paused,
                    last_paused,
                    game_renderer,
                } => self.menu_game(
                    game,
                    time_started,
                    last_paused,
                    total_duration_paused,
                    game_renderer,
                ),
                Menu::Pause => self.menu_pause(),
                Menu::GameOver(finished_stats) => self.menu_game_over(finished_stats),
                Menu::GameComplete(finished_stats) => self.menu_game_complete(finished_stats),
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
            Menu::Quit("quit from title menu".to_string()),
        ];
        self.generic_placeholder_menu("", selection)
    }

    fn menu_new_game(&mut self) -> io::Result<MenuUpdate> {
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
                "Can you clear 15 levels at instant gravity?".to_string(),
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
                    "Get perfect clears in all 24 puzzle levels.".to_string(),
                    Box::new(game_modifiers::puzzle_mode::new_game),
                ),
                (
                    "Cheese",
                    format!(
                        "Eat through lines like Swiss cheese. Limit: {:?}",
                        ng.cheese_mode_linelimit
                    ),
                    Box::new({
                        let cheese_mode_limit = ng.cheese_mode_linelimit;
                        let cheese_mode_gap_size = ng.cheese_mode_gap_size;
                        let cheese_mode_gravity = ng.cheese_mode_gravity;
                        move || {
                            game_modifiers::cheese_mode::new_game(
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
                        "Get consecutive line clears. Limit: {:?}{}",
                        ng.combo_mode_linelimit,
                        if ng.combo_start_layout != crate::game_modifiers::combo_mode::LAYOUTS[0] {
                            format!(", Layout={:b}", ng.combo_start_layout)
                        } else {
                            "".to_string()
                        }
                    ),
                    Box::new({
                        let combo_start_layout = ng.combo_start_layout;
                        move || game_modifiers::combo_mode::new_game(1, combo_start_layout)
                    }),
                ),
            ];
            if ng.experimental_mode_unlocked {
                special_gamemodes.insert(
                    1,
                    (
                        "Descent (experimental)",
                        "Spin the piece and collect 'gems' by touching them.".to_string(),
                        Box::new(game_modifiers::descent_mode::new_game),
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
                        } else if ng.custom_start_seed.is_some() || ng.custom_start_board.is_some()
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
                    format!("| Initial gravity: {}", ng.custom_initial_gravity),
                    format!("| Auto-increase gravity: {}", ng.custom_increase_gravity),
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
                            format!(
                                ">{stat_str}{}",
                                if customization_selected != 3 || ng.custom_mode_limit.is_some() {
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
                    code: KeyCode::Enter | KeyCode::Char('e'),
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
                                ..Limits::default()
                            },
                            Some(Stat::Pieces(max_pcs)) => Limits {
                                pieces: Some((true, max_pcs)),
                                ..Limits::default()
                            },
                            Some(Stat::Lines(max_lns)) => Limits {
                                lines: Some((true, max_lns)),
                                ..Limits::default()
                            },
                            Some(Stat::Gravity(max_lvl)) => Limits {
                                gravity: Some((true, max_lvl)),
                                ..Limits::default()
                            },
                            Some(Stat::Score(max_pts)) => Limits {
                                score: Some((true, max_pts)),
                                ..Limits::default()
                            },
                            None => Limits::default(),
                        };
                        let custom_mode = GameMode {
                            name: Some("Custom Mode".to_string()),
                            initial_gravity: ng.custom_initial_gravity,
                            increase_gravity: ng.custom_increase_gravity,
                            limits,
                        };
                        let mut custom_game_builder = Game::builder(custom_mode);
                        if let Some(seed) = ng.custom_start_seed {
                            custom_game_builder.seed(seed);
                        }
                        if let Some(ref custom_start_board_str) = ng.custom_start_board {
                            custom_game_builder.build_modified([
                                game_modifiers::utils::custom_start_board(custom_start_board_str),
                            ])
                        } else {
                            custom_game_builder.build()
                        }
                    };
                    // Set config.
                    game.config_mut().clone_from(self.settings.config());
                    let now = Instant::now();
                    break Ok(MenuUpdate::Push(Menu::Game {
                        game: Box::new(game),
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
                                ng.custom_initial_gravity =
                                    ng.custom_initial_gravity.saturating_add(d_gravity);
                            }
                            2 => {
                                ng.custom_increase_gravity = !ng.custom_increase_gravity;
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
                                ng.custom_initial_gravity =
                                    ng.custom_initial_gravity.saturating_sub(d_gravity);
                            }
                            2 => {
                                ng.custom_increase_gravity = !ng.custom_increase_gravity;
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
                        if let Some(limit) = ng.combo_mode_linelimit {
                            ng.combo_mode_linelimit = NonZeroUsize::try_from(limit.get() - 1).ok();
                        }
                    } else if selected == selection_size - 3 {
                        if let Some(limit) = ng.cheese_mode_linelimit {
                            ng.cheese_mode_linelimit = NonZeroUsize::try_from(limit.get() - 1).ok();
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
                        ng.combo_mode_linelimit = if let Some(limit) = ng.combo_mode_linelimit {
                            limit.checked_add(1)
                        } else {
                            Some(NonZeroUsize::MIN)
                        };
                    } else if selected == selection_size - 3 {
                        ng.cheese_mode_linelimit = if let Some(limit) = ng.cheese_mode_linelimit {
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
                        ng.custom_start_seed = None;
                        ng.custom_start_board = None;
                    } else if selected == selection_size - 2 {
                        let new_layout_idx = if let Some(i) =
                            crate::game_modifiers::combo_mode::LAYOUTS
                                .iter()
                                .position(|lay| *lay == ng.combo_start_layout)
                        {
                            let layout_cnt = crate::game_modifiers::combo_mode::LAYOUTS.len();
                            (i + 1) % layout_cnt
                        } else {
                            0
                        };
                        ng.combo_start_layout =
                            crate::game_modifiers::combo_mode::LAYOUTS[new_layout_idx];
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
        time_started: &mut Instant,
        last_paused: &mut Instant,
        total_duration_paused: &mut Duration,
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
        let mut buttons_pressed = PressedButtons::default();
        let (button_sender, button_receiver) = mpsc::channel();
        let _input_handler =
            TerminalInputHandler::new(&button_sender, self.settings.keybinds(), self.kitty_assumed);
        let mut combo_bot_handler = (self.combo_bot_enabled
            && game.mode().name.as_ref().is_some_and(|n| n == "Combo"))
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
        *total_duration_paused += session_resumed.saturating_duration_since(*last_paused);
        let mut clean_screen = true;
        let mut f = 0u32;
        let mut fps_counter = 0;
        let mut fps_counter_started = Instant::now();
        let menu_update = 'render: loop {
            // Exit if game ended
            if game.ended() {
                let finished_game_stats = self.store_game(game);
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
                    + Duration::from_secs_f64(f64::from(f) / self.settings.graphics().game_fps);
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
                        self.store_game(game);
                        break 'render MenuUpdate::Push(Menu::Quit(
                            "exited with ctrl-c".to_string(),
                        ));
                    }
                    Ok(InputSignal::ForfeitGame) => {
                        game.forfeit();
                        let finished_game_stats = self.store_game(game);
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
                        self.settings.new_game.custom_start_seed = Some(game.seed());
                        new_feedback_events.push((
                            game.state().time,
                            tetrs_engine::Feedback::Text("(Snapshot taken!)".to_string()),
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
            game_renderer.render(self, game, new_feedback_events, clean_screen)?;
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

    fn menu_game_ended(
        &mut self,
        selection: Vec<Menu>,
        success: bool,
        finished_game_stats: &FinishedGameStats,
    ) -> io::Result<MenuUpdate> {
        let FinishedGameStats {
            timestamp: _,
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
            rng: _,
        } = last_state;
        // if gamemode.name.as_ref().map(String::as_str) == Some("Puzzle")
        if gamemode.name.as_ref().is_some_and(|n| n == "Puzzle") && success {
            self.settings.new_game.experimental_mode_unlocked = true;
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
                    if success {
                        format!(
                            "++ Game Completed{} ++",
                            gamemode
                                .name
                                .as_ref()
                                .map_or("".to_string(), |name| format!(" ({name})"))
                        )
                    } else {
                        format!(
                            "-- Game Over{} by: {:?} --",
                            gamemode
                                .name
                                .as_ref()
                                .map_or("".to_string(), |name| format!(" ({name})")),
                            last_state.end.unwrap().unwrap_err()
                        )
                    }
                )))?
                /*.queue(MoveTo(0, y_main + y_selection + 2))?
                .queue(Print(Self::produce_header()?))?*/
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?
                .queue(MoveTo(x_main, y_main + y_selection + 3))?
                .queue(Print(format!("{:^w_main$}", format!("Score: {score}"))))?
                .queue(MoveTo(x_main, y_main + y_selection + 4))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Gravity: {gravity}",)
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 5))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Lines: {}", lines_cleared)
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 6))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Pieces: {}", pieces_played.iter().sum::<u32>())
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 7))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Time: {}", fmt_duration(*game_time))
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

    fn menu_game_over(
        &mut self,
        finished_game_stats: &FinishedGameStats,
    ) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::Quit("quit after game over".to_string()),
        ];
        self.menu_game_ended(selection, false, finished_game_stats)
    }

    fn menu_game_complete(
        &mut self,
        finished_game_stats: &FinishedGameStats,
    ) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::Quit("quit after game complete".to_string()),
        ];
        self.menu_game_ended(selection, true, finished_game_stats)
    }

    fn menu_pause(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::Scores,
            Menu::About,
            Menu::Quit("quit from pause".to_string()),
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
                "Adjust Graphics...".to_string(),
                "Adjust Keybinds...".to_string(),
                "Adjust Gameplay...".to_string(),
                format!(
                    "Keep save file: {}",
                    match self.settings.save_on_exit {
                        SavefileGranularity::Nothing => "OFF*",
                        SavefileGranularity::Settings => "ON--only settings, no scores",
                        SavefileGranularity::Everything => "ON",
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
                            "(*WARNING: current data will be lost on exit)".to_string()
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
                    code: KeyCode::Enter | KeyCode::Char('e'),
                    kind: Press,
                    ..
                }) => match selected {
                    0 => break Ok(MenuUpdate::Push(Menu::AdjustGraphics)),
                    1 => break Ok(MenuUpdate::Push(Menu::AdjustKeybinds)),
                    2 => break Ok(MenuUpdate::Push(Menu::AdjustGameplay)),
                    3 => {
                        self.settings.save_on_exit = SavefileGranularity::Everything;
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
                            SavefileGranularity::Nothing => SavefileGranularity::Everything,
                            SavefileGranularity::Settings => SavefileGranularity::Nothing,
                            SavefileGranularity::Everything => SavefileGranularity::Settings,
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
                            SavefileGranularity::Settings => SavefileGranularity::Everything,
                            SavefileGranularity::Everything => SavefileGranularity::Nothing,
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
                    "".to_string()
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
                        "exited with ctrl-c".to_string(),
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
                    "".to_string()
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
                        TetrominoSource::Uniform => "Uniformly random".to_string(),
                        TetrominoSource::Stock { .. } => "Bag".to_string(),
                        TetrominoSource::Recency { .. } => "Recency".to_string(),
                        TetrominoSource::BalanceRelative { .. } =>
                            "Balance relative counts".to_string(),
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
                        "exited with ctrl-c".to_string(),
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
                    "".to_string()
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
                        "exited with ctrl-c".to_string(),
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

    fn menu_scoreboard(&mut self) -> io::Result<MenuUpdate> {
        let max_entries = 14;
        let mut scroll = 0usize;
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
            let entries = self
                .past_games
                .iter()
                .skip(scroll)
                .take(max_entries)
                .map(
                    |FinishedGameStats {
                         timestamp,
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

    fn menu_about(&mut self) -> io::Result<MenuUpdate> {
        /* FIXME: About menu. */
        self.generic_placeholder_menu(
            "About tetrs - Visit https://github.com/Strophox/tetrs",
            vec![],
        )
    }

    fn store_game(&mut self, game: &Game) -> FinishedGameStats {
        let finished_game_stats = FinishedGameStats {
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string(),
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
