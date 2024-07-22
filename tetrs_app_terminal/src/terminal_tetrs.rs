use std::{
    collections::HashMap,
    io::{self, Write},
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
    style::{self, Print},
    terminal, ExecutableCommand, QueueableCommand,
};
use tetrs_lib::{Button, ButtonsPressed, Game, GameState, Gamemode, MeasureStat};

use crate::game_input_handler::{ButtonSignal, CT_Keycode, CrosstermHandler};
use crate::game_screen_renderers::{GameScreenRenderer, UnicodeRenderer};

#[derive(Debug)]
enum Menu {
    Title,
    NewGame(Gamemode),
    Game {
        game: Box<Game>,
        game_screen_renderer: UnicodeRenderer,
        total_duration_paused: Duration,
        last_paused: Instant,
    },
    GameOver(Box<GameState>),
    GameComplete(Box<GameState>),
    Pause, // TODO: Add information so game stats can be displayed here.
    Options,
    ConfigureControls,
    Scores,
    About,
    Quit(String),
}

// TODO: #[derive(Debug)]
enum MenuUpdate {
    Pop,
    Push(Menu),
}

// TODO: Derive `Default`?
#[derive(PartialEq, Clone, Debug)]
pub struct Settings {
    pub game_fps: f64,
    pub keybinds: HashMap<CT_Keycode, Button>,
    cache_custom_game: Gamemode,
    kitty_enabled: bool,
}

impl std::fmt::Display for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Menu::Title => "Title Screen",
            Menu::NewGame(_) => "New Game",
            Menu::Game { game, .. } => &format!("Game: {}", game.config().gamemode.name),
            Menu::GameOver(_) => "Game Over",
            Menu::GameComplete(_) => "Game Completed",
            Menu::Pause => "Pause",
            Menu::Options => "Options",
            Menu::ConfigureControls => "Configure Controls",
            Menu::Scores => "Scoreboard",
            Menu::About => "About",
            Menu::Quit(_) => "Quit",
        };
        write!(f, "{name}")
    }
}

#[derive(Debug)]
pub struct TerminalTetrs<T: Write> {
    pub term: T,
    pub settings: Settings,
}

impl<T: Write> Drop for TerminalTetrs<T> {
    fn drop(&mut self) {
        // Console epilogue: de-initialization.
        // TODO FIXME BUG: There's this horrible bug where the keyboard flags pop incorrectly: if I press escape in the pause menu, it resumes the game, but when I release escape during the game immediately after it interprets this as a "Press" as well, pausing again.
        if self.settings.kitty_enabled {
            let _ = self.term.execute(event::PopKeyboardEnhancementFlags);
        }
        let _ = terminal::disable_raw_mode();
        // let _ = self.term.execute(terminal::LeaveAlternateScreen); // NOTE: This is only manually done at the end of `run`, that way backtraces are not erased automatically here.
        let _ = self.term.execute(style::ResetColor);
        let _ = self.term.execute(cursor::Show);
    }
}

impl<T: Write> TerminalTetrs<T> {
    pub fn new(mut terminal: T, fps: u32) -> Self {
        // Console prologue: Initializion.
        let _ = terminal.execute(terminal::EnterAlternateScreen);
        let _ = terminal.execute(terminal::SetTitle("Tetrs"));
        let _ = terminal.execute(cursor::Hide);
        let _ = terminal::enable_raw_mode();
        let kitty_enabled = terminal::supports_keyboard_enhancement().unwrap_or(false);
        if kitty_enabled {
            // TODO: This is kinda iffy. Do we need all flags? What undesirable effects might there be?
            let _ = terminal.execute(event::PushKeyboardEnhancementFlags(
                event::KeyboardEnhancementFlags::all(),
            ));
        }
        // TODO: Store different keybind mappings somewhere and get default from there.
        let ct_keybinds = HashMap::from([
            (CT_Keycode::Left, Button::MoveLeft),
            (CT_Keycode::Right, Button::MoveRight),
            (CT_Keycode::Char('a'), Button::RotateLeft),
            (CT_Keycode::Char('d'), Button::RotateRight),
            (CT_Keycode::Down, Button::DropSoft),
            (CT_Keycode::Up, Button::DropHard),
        ]);
        let settings = Settings {
            keybinds: ct_keybinds,
            game_fps: fps.into(),
            cache_custom_game: Gamemode::custom(
                "Custom, Unnamed".to_string(),
                NonZeroU32::MIN,
                true,
                None,
                MeasureStat::Time(Duration::ZERO),
            ),
            kitty_enabled,
        };
        Self {
            term: terminal,
            settings,
        }
    }

    pub fn run(&mut self) -> io::Result<String> {
        let mut menu_stack = vec![Menu::Title];
        // TODO: Remove this once menus are navigable.
        // menu_stack.push(Menu::NewGame(Gamemode::custom(
        //     "Unnamed Custom".to_string(),
        //     NonZeroU32::MIN,
        //     true,
        //     Some(MeasureStat::Pieces(100)),
        //     MeasureStat::Score(0),
        // )));
        // menu_stack.push(Menu::Game {
        //     game: Box::new(Game::with_gamemode(
        //         Gamemode::custom(
        //             "Debug".to_string(),
        //             NonZeroU32::try_from(10).unwrap(),
        //             true,
        //             None,
        //             MeasureStat::Pieces(0),
        //         ),
        //         Instant::now(),
        //     )),
        //     game_screen_renderer: Default::default(),
        //     total_duration_paused: Duration::ZERO,
        //     last_paused: Instant::now(),
        // });
        // menu_stack.push(Menu::Game {
        //     game: Box::new(Game::with_gamemode(Gamemode::marathon(), Instant::now())),
        //     game_screen_renderer: Default::default(),
        //     total_duration_paused: Duration::ZERO,
        //     last_paused: Instant::now(),
        // });
        // menu_stack.push(Menu::Game {
        //     game: Box::new(Game::with_gamemode(Gamemode::master(), Instant::now())),
        //     game_screen_renderer: Default::default(),
        //     total_duration_paused: Duration::ZERO,
        //     last_paused: Instant::now(),
        // });
        // Preparing main application loop.
        let msg = loop {
            // Retrieve active menu, stop application if stack is empty.
            let Some(screen) = menu_stack.last_mut() else {
                break String::from("all menus exited");
            };
            // Open new menu screen, then store what it returns.
            let menu_update = match screen {
                Menu::Title => self.title(),
                Menu::NewGame(gamemode) => self.newgame(gamemode),
                Menu::Game {
                    game,
                    game_screen_renderer: renderer,
                    total_duration_paused,
                    last_paused,
                } => self.game(game, renderer, total_duration_paused, last_paused),
                Menu::Pause => self.pause(),
                Menu::GameOver(gamestate) => self.gameover(gamestate),
                Menu::GameComplete(gamestate) => self.gamecomplete(gamestate),
                Menu::Scores => self.scores(),
                Menu::About => self.about(),
                Menu::Options => self.options(),
                Menu::ConfigureControls => self.configurecontrols(),
                Menu::Quit(string) => break string.clone(),
            }?;
            // Change screen session depending on what response screen gave.
            match menu_update {
                MenuUpdate::Pop => {
                    menu_stack.pop();
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
        // TODO: This is done here manually for debug reasons in case the application still crashes somehow, c.f. note in `Drop::drop(self)`.
        let _ = self.term.execute(terminal::LeaveAlternateScreen);
        Ok(msg)
    }

    fn generic_placeholder_widget(
        &mut self,
        current_menu_name: &str,
        selection: Vec<Menu>,
    ) -> io::Result<MenuUpdate> {
        let mut selected = 0usize;
        loop {
            // Draw menu.
            let (console_width, console_height) = terminal::size()?;
            let (w_x, w_y) = (
                console_width.saturating_sub(80) / 2,
                console_height.saturating_sub(24) / 2,
            );
            let names = selection
                .iter()
                .map(|menu| menu.to_string())
                .collect::<Vec<_>>();
            let menu_y = 24 / 3;
            if current_menu_name.is_empty() {
                self.term
                    .queue(terminal::Clear(terminal::ClearType::All))?
                    .queue(MoveTo(w_x, w_y + menu_y))?
                    .queue(Print(format!("{:^80}", "▀█▀ ██ ▀█▀ █▀▀ ▄█▀")))?
                    .queue(MoveTo(w_x, w_y + menu_y + 1))?
                    .queue(Print(format!("{:^80}", "    █▄▄▄▄▄▄       ")))?;
            } else {
                self.term
                    .queue(terminal::Clear(terminal::ClearType::All))?
                    .queue(MoveTo(w_x, w_y + menu_y))?
                    .queue(Print(format!(
                        "{:^80}",
                        format!("[ {} ]", current_menu_name.to_ascii_uppercase())
                    )))?
                    .queue(MoveTo(w_x, w_y + menu_y + 2))?
                    .queue(Print(format!("{:^80}", "──────────────────────────")))?;
            }
            if names.is_empty() {
                self.term
                    .queue(MoveTo(w_x, w_y + menu_y + 5))?
                    .queue(Print(format!(
                        "{:^80}",
                        "There isn't anything interesting here... (yet)"
                    )))?;
            } else {
                for (i, name) in names.into_iter().enumerate() {
                    self.term
                        .queue(MoveTo(w_x, w_y + menu_y + 4 + u16::try_from(i).unwrap()))?
                        .queue(Print(format!(
                            "{:^80}",
                            if i == selected {
                                format!(">>> {name} <<<")
                            } else {
                                name
                            }
                        )))?;
                }
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
                    code: CT_Keycode::Esc,
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),
                // Select next menu.
                Event::Key(KeyEvent {
                    code: CT_Keycode::Enter,
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
                    code: CT_Keycode::Up | CT_Keycode::Left,
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !selection.is_empty() {
                        selected += selection.len() - 1;
                    }
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: CT_Keycode::Down | CT_Keycode::Right,
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
        /* TODO: Title menu.
        Title
            -> { Quit }
        Title
            -> { NewGame Options Scores About }
            [color="#007FFF"]
        */
        let selection = vec![
            Menu::NewGame(self.settings.cache_custom_game.clone()),
            Menu::Options,
            Menu::Scores,
            Menu::About,
        ];
        self.generic_placeholder_widget("", selection)
    }

    fn newgame(&mut self, gamemode: &mut Gamemode) -> io::Result<MenuUpdate> {
        /* TODO: Newgame menu.
        NewGame
            -> { Game }
        NewGame
            -> { Options }
            [color="#007FFF"]

        MenuUpdate::Pop
        */
        let now = Instant::now();
        let selection = vec![
            Menu::Game {
                game: Box::new(Game::with_gamemode(Gamemode::marathon(), now)),
                game_screen_renderer: Default::default(),
                total_duration_paused: Duration::ZERO,
                last_paused: now,
            },
            Menu::Game {
                game: Box::new(Game::with_gamemode(
                    Gamemode::sprint(NonZeroU32::try_from(10).unwrap()),
                    now,
                )),
                game_screen_renderer: Default::default(),
                total_duration_paused: Duration::ZERO,
                last_paused: now,
            },
            Menu::Game {
                game: Box::new(Game::with_gamemode(
                    Gamemode::ultra(NonZeroU32::try_from(10).unwrap()),
                    now,
                )),
                game_screen_renderer: Default::default(),
                total_duration_paused: Duration::ZERO,
                last_paused: now,
            },
            Menu::Game {
                game: Box::new(Game::with_gamemode(Gamemode::master(), now)),
                game_screen_renderer: Default::default(),
                total_duration_paused: Duration::ZERO,
                last_paused: now,
            },
            Menu::Game {
                game: Box::new(Game::with_gamemode(Gamemode::endless(), now)),
                game_screen_renderer: Default::default(),
                total_duration_paused: Duration::ZERO,
                last_paused: now,
            },
        ];
        self.generic_placeholder_widget("Start New Game", selection)
    }

    fn game(
        &mut self,
        game: &mut Game,
        game_screen_renderer: &mut impl GameScreenRenderer,
        total_duration_paused: &mut Duration,
        time_paused: &mut Instant,
    ) -> io::Result<MenuUpdate> {
        /* TODO: Game menu.
        Game
            -> { GameOver GameComplete }
        Game
            -> { Pause }
            [color="#007FFF"]
        */
        // Prepare channel with which to communicate `Button` inputs / game interrupt.
        let mut buttons_pressed = ButtonsPressed::default();
        let (tx, rx) = mpsc::channel::<ButtonSignal>();
        let _input_handler =
            CrosstermHandler::new(&tx, &self.settings.keybinds, self.settings.kitty_enabled);
        // Game Loop
        let time_game_resumed = Instant::now();
        *total_duration_paused += time_game_resumed.saturating_duration_since(*time_paused);
        let mut f = 0u32;
        let next_menu = 'render_loop: loop {
            // Exit if game ended
            if let Some(good_end) = game.finished() {
                let menu = if good_end.is_ok() {
                    Menu::GameComplete
                } else {
                    Menu::GameOver
                }(Box::new(game.state().clone()));
                // TODO: Temporary writing current game to file.
                let mut file = std::fs::File::create("./tetrs_last_game.txt")?;
                let _ = file.write(format!("{game:#?}").as_bytes());
                break MenuUpdate::Push(menu);
            }
            // Start next frame
            f += 1;
            let next_frame_at =
                time_game_resumed + Duration::from_secs_f64(f64::from(f) / self.settings.game_fps);
            let mut new_feedback_events = Vec::new();
            'idle_loop: loop {
                let frame_idle_remaining = next_frame_at - Instant::now();
                match rx.recv_timeout(frame_idle_remaining) {
                    Ok(None) => {
                        break 'render_loop MenuUpdate::Push(Menu::Pause);
                    }
                    Ok(Some((instant, button, button_state))) => {
                        buttons_pressed[button] = button_state;
                        let instant = std::cmp::max(
                            instant - *total_duration_paused,
                            game.state().last_updated,
                        );
                        if let Ok(evts) = game.update(Some(buttons_pressed), instant) {
                            new_feedback_events.extend(evts);
                        }
                        continue 'idle_loop;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if let Ok(evts) = game.update(None, Instant::now() - *total_duration_paused)
                        {
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
            game_screen_renderer.render(self, game, new_feedback_events)?;
        };
        *time_paused = Instant::now();
        Ok(next_menu)
    }

    fn gameover(&mut self, gamestate: &GameState) -> io::Result<MenuUpdate> {
        /* TODO: Gameover menu.
        GameOver
            -> { Quit }
        GameOver
            -> { NewGame Scores }
            [color="#007FFF"]
        */
        let selection = vec![
            Menu::NewGame(self.settings.cache_custom_game.clone()),
            Menu::Scores,
            Menu::Quit("quit after gameover".to_string()),
        ];
        self.generic_placeholder_widget("Game Over", selection)
    }

    fn gamecomplete(&mut self, gamestate: &GameState) -> io::Result<MenuUpdate> {
        /* TODO: Gamecomplete menu.
        GameComplete
            -> { Quit }
        GameComplete
            -> { NewGame Scores }
            [color="#007FFF"]
        */
        let selection = vec![
            Menu::NewGame(self.settings.cache_custom_game.clone()),
            Menu::Scores,
            Menu::Quit("quit after game complete".to_string()),
        ];
        self.generic_placeholder_widget("Game Completed", selection)
    }

    fn pause(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: Pause menu.
        Pause
            -> { Quit }
        Pause
            -> { NewGame Scores Options About }
            [color="#007FFF"]

        MenuUpdate::Pop
        */
        let selection = vec![
            Menu::NewGame(self.settings.cache_custom_game.clone()),
            Menu::Scores,
            Menu::Options,
            Menu::About,
            Menu::Quit("quit from pause".to_string()),
        ];
        self.generic_placeholder_widget("Paused", selection)
    }

    fn options(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: Options menu.
        Options
            -> { ConfigureControls }
            [color="#007FFF"]

        MenuUpdate::Pop
        */
        self.generic_placeholder_widget("Settings", vec![Menu::ConfigureControls])
    }

    fn configurecontrols(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: Configurecontrols menu.

        MenuUpdate::Pop
        */
        self.generic_placeholder_widget("Configure Controls", vec![])
    }

    fn scores(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: Scores menu.

        MenuUpdate::Pop
        */
        self.generic_placeholder_widget("Highscores", vec![])
    }

    fn about(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: About menu.

        MenuUpdate::Pop
        */
        self.generic_placeholder_widget("About Tetrs", vec![])
    }
}
