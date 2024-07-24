use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    io::{self, Read, Write},
    num::NonZeroU32,
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
    terminal, ExecutableCommand, QueueableCommand,
};
use tetrs_engine::{Button, ButtonsPressed, Game, GameState, Gamemode, RotationSystem, Stat};

use crate::game_input_handler::{ButtonSignal, CrosstermHandler, Sig};
use crate::game_renderers::{GameScreenRenderer, UnicodeRenderer};

// NOTE: This could be more general and less ad-hoc. Count number of I-Spins, J-Spins, etc..
pub type GameRunningStats = ([u32; 5], Vec<u32>);

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GameFinishedStats {
    timestamp: String,
    actions: [u32; 5],
    score_bonuses: Vec<u32>,
    gamemode: Gamemode,
    last_state: GameState,
}

impl GameFinishedStats {
    fn was_successful(&self) -> bool {
        self.last_state.finished.is_some_and(|fin| fin.is_ok())
    }
}

#[derive(Clone, Debug)]
enum Menu {
    Title,
    NewGame,
    Game {
        game: Box<Game>,
        time_started: Instant,
        last_paused: Instant,
        total_duration_paused: Duration,
        game_running_stats: GameRunningStats,
        game_renderer: Box<UnicodeRenderer>,
    },
    GameOver(Box<GameFinishedStats>),
    GameComplete(Box<GameFinishedStats>),
    Pause,
    Settings,
    ConfigureControls,
    Scores,
    About,
    Quit(String),
}

impl std::fmt::Display for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Menu::Title => "Title Screen",
            Menu::NewGame => "New Game",
            Menu::Game { game, .. } => &format!("Game: {}", game.config().gamemode.name),
            Menu::GameOver(_) => "Game Over",
            Menu::GameComplete(_) => "Game Completed",
            Menu::Pause => "Pause",
            Menu::Settings => "Settings",
            Menu::ConfigureControls => "Configure Controls",
            Menu::Scores => "Scoreboard",
            Menu::About => "About",
            Menu::Quit(_) => "Quit",
        };
        write!(f, "{name}")
    }
}

#[derive(Clone, Debug)]
enum MenuUpdate {
    Pop,
    Push(Menu),
}

#[derive(PartialEq, Clone, Debug)]
pub struct Settings {
    pub keybinds: HashMap<KeyCode, Button>,
    pub game_fps: f64,
    pub rotation_system: RotationSystem,
}

#[derive(PartialEq, Clone, Debug)]
pub struct App<T: Write> {
    pub term: T,
    pub settings: Settings,
    custom_mode: Gamemode,
    kitty_enabled: bool,
    games_finished: Vec<GameFinishedStats>,
}

impl<T: Write> Drop for App<T> {
    fn drop(&mut self) {
        // TODO: Handle errors?
        let _ = Self::save_games(&self.games_finished);
        // Console epilogue: de-initialization.
        if self.kitty_enabled {
            let _ = self.term.execute(event::PopKeyboardEnhancementFlags);
        }
        let _ = terminal::disable_raw_mode();
        // let _ = self.term.execute(terminal::LeaveAlternateScreen); // NOTE: This is only manually done at the end of `run`, that way backtraces are not erased automatically here.
        let _ = self.term.execute(style::ResetColor);
        let _ = self.term.execute(cursor::Show);
    }
}

impl<T: Write> App<T> {
    pub const W_MAIN: u16 = 80;
    pub const H_MAIN: u16 = 24;
    pub const SAVE_FILE: &'static str = "./tetrs_terminal_scores.json";

    pub fn new(mut terminal: T, fps: u32) -> Self {
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
        let keybinds = HashMap::from([
            (KeyCode::Left, Button::MoveLeft),
            (KeyCode::Right, Button::MoveRight),
            (KeyCode::Char('a'), Button::RotateLeft),
            (KeyCode::Char('d'), Button::RotateRight),
            (KeyCode::Down, Button::DropSoft),
            (KeyCode::Up, Button::DropHard),
        ]);
        let settings = Settings {
            keybinds,
            game_fps: fps.into(),
            rotation_system: RotationSystem::Ocular,
        };
        let custom_mode = Gamemode::custom(
            "Custom Mode".to_string(),
            NonZeroU32::MIN,
            true,
            None,
            Stat::Pieces(0),
        );
        let games_finished = Self::load_games().unwrap_or(Vec::new());
        Self {
            term: terminal,
            settings,
            kitty_enabled,
            custom_mode,
            games_finished,
        }
    }

    fn save_games(games_finished: &[GameFinishedStats]) -> io::Result<()> {
        let relevant_games_finished = games_finished
            .iter()
            .filter(|game_finished_stats| {
                game_finished_stats.was_successful()
                    || match game_finished_stats.gamemode.optimize {
                        Stat::Time(dur) => dur > Duration::from_secs(20),
                        Stat::Pieces(pcs) => pcs > 10,
                        Stat::Lines(lns) => lns > 0,
                        Stat::Level(lvl) => lvl.get() > 1,
                        Stat::Score(pts) => pts > 0,
                    }
            })
            .collect::<Vec<_>>();
        let save_str = serde_json::to_string(&relevant_games_finished)?;
        let mut file = File::create(Self::SAVE_FILE)?;
        // TODO: Handle error?
        let _ = file.write(save_str.as_bytes())?;
        Ok(())
    }

    fn load_games() -> io::Result<Vec<GameFinishedStats>> {
        let mut file = File::open(Self::SAVE_FILE)?;
        let mut save_str = String::new();
        file.read_to_string(&mut save_str)?;
        let games_finished = serde_json::from_str(&save_str)?;
        Ok(games_finished)
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
                    game_running_stats,
                    game_renderer,
                } => self.game(
                    game,
                    time_started,
                    last_paused,
                    total_duration_paused,
                    game_running_stats,
                    game_renderer.as_mut(),
                ),
                Menu::Pause => self.pause(),
                Menu::GameOver(game_finished_stats) => self.gameover(game_finished_stats),
                Menu::GameComplete(game_finished_stats) => self.gamecomplete(game_finished_stats),
                Menu::Scores => self.scores(),
                Menu::About => self.about(),
                Menu::Settings => self.settings(),
                Menu::ConfigureControls => self.configurecontrols(),
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
        // NOTE: This is done here manually (instead of `Drop::drop(self)`) so debug is not wiped in case the application crashes before reaching this point.
        let _ = self.term.execute(terminal::LeaveAlternateScreen);
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
                        format!("[ {} ]", current_menu_name.to_ascii_uppercase())
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
                        y_main + y_selection + 4 + u16::try_from(n_names).unwrap() + 3,
                    ))?
                    .queue(PrintStyledContent(
                        format!("{:^w_main$}", "Use [←] [→] [↑] [↓] [Esc] [Enter].",).italic(),
                    ))?;
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
            Gamemode::marathon(),
            Gamemode::sprint(NonZeroU32::try_from(5).unwrap()),
            Gamemode::ultra(NonZeroU32::try_from(5).unwrap()),
            Gamemode::master(),
        ];
        let (d_time, d_score, d_pieces, d_lines, d_level) = (Duration::from_secs(5), 200, 10, 5, 1);
        let mut selected = 0usize;
        let mut selected_custom = 0usize;
        // There are the preset gamemodes + custom gamemode.
        let selected_cnt = preset_gamemodes.len() + 1;
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
                .queue(Print(format!("{:^w_main$}", "Start New Game")))?
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
                            "▓▓> Custom Mode: (*change 'limit' by pressing right repeatedly)"
                        } else {
                            "  > Custom Mode: (*change 'limit' by pressing right repeatedly)"
                        }
                    } else {
                        "Custom Mode..."
                    }
                )))?;
            // Render custom mode stuff.
            if selected == selected_cnt - 1 {
                let stats_str = [
                    (1, format!("level start: {}", self.custom_mode.start_level)),
                    (
                        2,
                        format!("level increment: {}", self.custom_mode.increment_level),
                    ),
                    (3, format!("limit: {:?}", self.custom_mode.limit)),
                ]
                .map(|(j, stat_str)| {
                    if j == selected_custom {
                        format!("▓▓{stat_str}")
                    } else {
                        stat_str
                    }
                })
                .join("    ");
                self.term
                    .queue(MoveTo(
                        x_main + 16,
                        y_main + y_selection + 4 + 2 * u16::try_from(selected_cnt).unwrap(),
                    ))?
                    .queue(Print(stats_str))?;
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
                    let mode = if selected < selected_cnt - 1 {
                        // SAFETY: Index is valid.
                        preset_gamemodes.into_iter().nth(selected).unwrap()
                    } else {
                        self.custom_mode.clone()
                    };
                    let mut game = Game::with_gamemode(mode);
                    game.config_mut().rotation_system = self.settings.rotation_system;
                    let now = Instant::now();
                    break Ok(MenuUpdate::Push(Menu::Game {
                        game: Box::new(game),
                        time_started: now,
                        last_paused: now,
                        total_duration_paused: Duration::ZERO,
                        game_running_stats: GameRunningStats::default(),
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
                                self.custom_mode.start_level =
                                    self.custom_mode.start_level.saturating_add(d_level);
                            }
                            2 => {
                                self.custom_mode.increment_level =
                                    !self.custom_mode.increment_level;
                            }
                            3 => {
                                match self.custom_mode.limit {
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
                                self.custom_mode.start_level = NonZeroU32::try_from(
                                    self.custom_mode.start_level.get() - d_level,
                                )
                                .unwrap_or(NonZeroU32::MIN);
                            }
                            2 => {
                                self.custom_mode.increment_level =
                                    !self.custom_mode.increment_level;
                            }
                            3 => {
                                match self.custom_mode.limit {
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
                            self.custom_mode.limit = match self.custom_mode.limit {
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
        game_running_stats: &mut GameRunningStats,
        game_renderer: &mut impl GameScreenRenderer,
    ) -> io::Result<MenuUpdate> {
        // Update rotation system manually.
        game.config_mut().rotation_system = self.settings.rotation_system;
        // Prepare channel with which to communicate `Button` inputs / game interrupt.
        let mut buttons_pressed = ButtonsPressed::default();
        let (tx, rx) = mpsc::channel::<ButtonSignal>();
        let _input_handler =
            CrosstermHandler::new(&tx, &self.settings.keybinds, self.kitty_enabled);
        // Game Loop
        let session_resumed = Instant::now();
        *total_duration_paused += session_resumed.saturating_duration_since(*last_paused);
        let mut clean_screen = true;
        let mut f = 0u32;
        let menu_update = 'render_loop: loop {
            // Exit if game ended
            if game.is_finished() {
                let game_finished_stats = self.store_game(game, game_running_stats);
                let menu = if game_finished_stats.was_successful() {
                    Menu::GameComplete
                } else {
                    Menu::GameOver
                }(Box::new(game_finished_stats));
                break 'render_loop MenuUpdate::Push(menu);
            }
            // Start next frame
            f += 1;
            let next_frame_at =
                session_resumed + Duration::from_secs_f64(f64::from(f) / self.settings.game_fps);
            let mut new_feedback_events = Vec::new();
            'idle_loop: loop {
                let frame_idle_remaining = next_frame_at - Instant::now();
                match rx.recv_timeout(frame_idle_remaining) {
                    Ok(Err(Sig::ExitProgram)) => {
                        self.store_game(game, game_running_stats);
                        break 'render_loop MenuUpdate::Push(Menu::Quit(
                            "exited with ctrl-c".to_string(),
                        ));
                    }
                    Ok(Err(Sig::ForfeitGame)) => {
                        let game_finished_stats = self.store_game(game, game_running_stats);
                        break 'render_loop MenuUpdate::Push(Menu::GameOver(Box::new(
                            game_finished_stats,
                        )));
                    }
                    Ok(Err(Sig::Pause)) => {
                        *last_paused = Instant::now();
                        break 'render_loop MenuUpdate::Push(Menu::Pause);
                    }
                    Ok(Err(Sig::WindowResize)) => {
                        clean_screen = true;
                        continue 'idle_loop;
                    }
                    Ok(Ok((instant, button, button_state))) => {
                        buttons_pressed[button] = button_state;
                        let game_time_userinput = instant.saturating_duration_since(*time_started)
                            - *total_duration_paused;
                        let game_now = std::cmp::max(game_time_userinput, game.state().game_time);
                        if let Ok(evts) = game.update(Some(buttons_pressed), game_now) {
                            new_feedback_events.extend(evts);
                        }
                        continue 'idle_loop;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let game_time_now = Instant::now().saturating_duration_since(*time_started)
                            - *total_duration_paused;
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
            // TODO: Make this more elegantly modular.
            game_renderer.render(
                self,
                game,
                game_running_stats,
                new_feedback_events,
                clean_screen,
            )?;
            clean_screen = false;
        };
        Ok(menu_update)
    }

    fn store_game(
        &mut self,
        game: &Game,
        game_running_stats: &mut GameRunningStats,
    ) -> GameFinishedStats {
        let mut gamemode = game.config().gamemode.clone();
        gamemode.optimize = match gamemode.optimize {
            Stat::Time(_) => Stat::Time(game.state().game_time),
            Stat::Pieces(_) => Stat::Pieces(game.state().pieces_played.iter().sum()),
            Stat::Lines(_) => Stat::Lines(game.state().lines_cleared.len()),
            Stat::Level(_) => Stat::Level(game.state().level),
            Stat::Score(_) => Stat::Score(game.state().score),
        };
        let game_finished_stats = GameFinishedStats {
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string(),
            actions: game_running_stats.0,
            score_bonuses: game_running_stats.1.clone(),
            gamemode,
            last_state: game.state().clone(),
        };
        self.games_finished.push(game_finished_stats.clone());
        self.games_finished
            .sort_by(|game_finished_stats_1, game_finished_stats_2| {
                let (gm1, gm2) = (
                    &game_finished_stats_1.gamemode,
                    &game_finished_stats_2.gamemode,
                );
                gm1.limit.cmp(&gm2.limit).then_with(|| {
                    // NOW: Same limit.
                    let sort_descendingly = if std::mem::discriminant(&gm1.optimize)
                        == std::mem::discriminant(&gm1.optimize)
                    {
                        // NOW: Same optimization type.
                        if let Some(gm1_limit) = gm1.limit {
                            gm1.optimize > gm1_limit
                        } else {
                            // TODO: This would have to change if `Stat::Finesse` were introduced, since we want to sort that ascendingly/minimize on scoreboard.
                            true
                        }
                    } else {
                        // "Whatever" case.
                        false
                    };
                    if sort_descendingly {
                        gm1.optimize.cmp(&gm2.optimize).reverse()
                    } else {
                        gm1.optimize.cmp(&gm2.optimize)
                    }
                })
            });
        game_finished_stats
    }

    fn generic_game_finished(
        &mut self,
        selection: Vec<Menu>,
        success: bool,
        game_finished_stats: &GameFinishedStats,
    ) -> io::Result<MenuUpdate> {
        let GameFinishedStats {
            timestamp: _,
            actions,
            score_bonuses,
            gamemode,
            last_state,
        } = game_finished_stats;
        let GameState {
            game_time,
            finished: _,
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
                .queue(terminal::Clear(terminal::ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!(
                        "Game {}! - {}",
                        if success { "Completed" } else { "Over" },
                        gamemode.name.to_ascii_uppercase()
                    )
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?
                .queue(MoveTo(x_main, y_main + y_selection + 4))?
                .queue(Print(format!("{:^w_main$}", format!("Score: {score}"))))?
                .queue(MoveTo(x_main, y_main + y_selection + 5))?
                .queue(Print(format!("{:^w_main$}", format!("Level: {level}",))))?
                .queue(MoveTo(x_main, y_main + y_selection + 6))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    format!("Lines: {}", lines_cleared.len())
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

    fn gameover(&mut self, game_finished_stats: &GameFinishedStats) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Scores,
            Menu::Settings,
            Menu::Quit("quit after game over".to_string()),
        ];
        self.generic_game_finished(selection, false, game_finished_stats)
    }

    fn gamecomplete(&mut self, game_finished_stats: &GameFinishedStats) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Scores,
            Menu::Settings,
            Menu::Quit("quit after game complete".to_string()),
        ];
        self.generic_game_finished(selection, true, game_finished_stats)
    }

    fn pause(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Scores,
            Menu::Settings,
            Menu::About,
            Menu::Quit("quit from pause".to_string()),
        ];
        self.generic_placeholder_widget("Paused", selection)
    }

    fn settings(&mut self) -> io::Result<MenuUpdate> {
        let selection_len = 3;
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
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(0).unwrap(),
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == 0 {
                        ">>> Configure Controls <<<"
                    } else {
                        "Configure Controls"
                    }
                )))?
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(1).unwrap(),
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == 1 {
                        format!(">>> FPS: {} <<<", self.settings.game_fps)
                    } else {
                        format!("FPS: {}", self.settings.game_fps)
                    }
                )))?
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(2).unwrap(),
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == 2 {
                        format!(
                            ">>> Rotation System: '{:?}' <<<",
                            self.settings.rotation_system
                        )
                    } else {
                        format!("Rotation System: '{:?}'", self.settings.rotation_system)
                    }
                )))?
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 4 + u16::try_from(selection_len).unwrap() + 3,
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
                    if selected == 0 {
                        break Ok(MenuUpdate::Push(Menu::ConfigureControls));
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
                }) => {
                    if selected == 1 {
                        self.settings.game_fps += 1.0;
                    } else if selected == 2 {
                        self.settings.rotation_system = match self.settings.rotation_system {
                            RotationSystem::Ocular => RotationSystem::Classic,
                            RotationSystem::Classic => RotationSystem::Super,
                            RotationSystem::Super => RotationSystem::Ocular,
                        };
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 1 && self.settings.game_fps > 0.0 {
                        self.settings.game_fps -= 1.0;
                    } else if selected == 2 {
                        self.settings.rotation_system = match self.settings.rotation_system {
                            RotationSystem::Ocular => RotationSystem::Super,
                            RotationSystem::Classic => RotationSystem::Ocular,
                            RotationSystem::Super => RotationSystem::Classic,
                        };
                    }
                }
                // Other event: don't care.
                _ => {}
            }
            selected = selected.rem_euclid(selection_len);
        }
    }

    fn configurecontrols(&mut self) -> io::Result<MenuUpdate> {
        let button_selection = [
            Button::MoveLeft,
            Button::MoveRight,
            Button::RotateLeft,
            Button::RotateRight,
            Button::RotateAround,
            Button::DropSoft,
            Button::DropHard,
        ];
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(terminal::Clear(terminal::ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "Configure Controls")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let button_names = button_selection
                .iter()
                .map(|&button| {
                    format!(
                        "{button:?}: {}",
                        format_keybinds(button, &self.settings.keybinds)
                    )
                })
                .collect::<Vec<_>>();
            let n_buttons = button_names.len();
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
                    y_main + y_selection + 4 + u16::try_from(n_buttons).unwrap() + 3,
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
                    let current_button = button_selection[selected];
                    self.term
                        .execute(MoveTo(
                            x_main,
                            y_main + y_selection + 4 + u16::try_from(n_buttons).unwrap() + 3,
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
                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += button_selection.len() - 1;
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
            selected = selected.rem_euclid(button_selection.len());
        }
    }

    fn scores(&mut self) -> io::Result<MenuUpdate> {
        let max_entries = 16;
        let mut scroll = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(terminal::Clear(terminal::ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "* Highscores *")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            let entries = self
                .games_finished
                .iter()
                .skip(scroll)
                .take(max_entries)
                .map(
                    |gfs @ GameFinishedStats {
                         timestamp,
                         actions: _,
                         score_bonuses: _,
                         gamemode,
                         last_state,
                     }| {
                        format!(
                            "{}: {} ({} limit{}) @{}",
                            gamemode.name,
                            match gamemode.optimize {
                                Stat::Lines(_) =>
                                    format!("{} lines", last_state.lines_cleared.len()),
                                Stat::Level(_) => format!("{} levels", last_state.level),
                                Stat::Score(_) => format!("{} points", last_state.score),
                                Stat::Pieces(_) => format!(
                                    "{} pieces",
                                    last_state.pieces_played.iter().sum::<u32>()
                                ),
                                Stat::Time(_) => format_duration(last_state.game_time),
                            },
                            match gamemode.limit {
                                None => "no".to_string(),
                                Some(Stat::Lines(lns)) => format!("{lns} ln"),
                                Some(Stat::Level(lvl)) => format!("{lvl} lvl"),
                                Some(Stat::Score(pts)) => format!("{pts} pts"),
                                Some(Stat::Pieces(pcs)) => format!("{pcs} pcs"),
                                Some(Stat::Time(dur)) => format_duration(dur),
                            },
                            if gfs.was_successful() {
                                ""
                            } else {
                                " *not fin."
                            },
                            timestamp,
                        )
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
            let entries_left = self
                .games_finished
                .len()
                .saturating_sub(max_entries + scroll);
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

    fn about(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: About menu. */
        self.generic_placeholder_widget(
            "About Tetrs - See https://github.com/Strophox/tetrs",
            vec![],
        )
    }
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
