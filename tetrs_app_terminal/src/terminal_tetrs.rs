use std::{
    collections::HashMap,
    io::{self, Write},
    num::NonZeroU32,
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style, terminal, ExecutableCommand, QueueableCommand,
};
use tetrs_lib::{Button, ButtonsPressed, Game, Gamemode, MeasureStat};

use crate::game_screen_renderers::{DebugRenderer, GameScreenRenderer, UnicodeRenderer};
use crate::input_handler::{ButtonSignal, CT_Keycode, CrosstermHandler};

// TODO: #[derive(Debug)]
enum Menu {
    Title,
    NewGame(Gamemode),
    Game {
        game: Box<Game>,
        game_screen_renderer: UnicodeRenderer,
        total_duration_paused: Duration,
        last_paused: Instant,
    },
    GameOver,
    GameComplete,
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
    Set(Menu),
}

// TODO: Derive `Default`?
#[derive(PartialEq, Clone, Debug)]
pub struct Settings {
    pub game_fps: f64,
    pub keybinds: HashMap<CT_Keycode, Button>,
    kitty_enabled: bool,
}

#[derive(Debug)]
pub struct TerminalTetrs<T: Write> {
    pub term: T,
    pub settings: Settings,
}

impl<T: Write> Drop for TerminalTetrs<T> {
    fn drop(&mut self) {
        // Console epilogue: de-initialization.
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
    pub fn new(mut terminal: T) -> Self {
        // Console prologue: Initializion.
        let _ = terminal.execute(terminal::SetTitle("Tetrs"));
        let _ = terminal.execute(cursor::Hide);
        let _ = terminal.execute(terminal::EnterAlternateScreen);
        let _ = terminal::enable_raw_mode();
        let mut kitty_enabled =
            crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false);
        if kitty_enabled {
            if let Err(_) = terminal.execute(event::PushKeyboardEnhancementFlags(
                event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
            )) {
                kitty_enabled = false;
            }
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
            game_fps: 24.0,
            kitty_enabled,
        };
        Self {
            term: terminal,
            settings,
        }
    }

    pub fn run(&mut self) -> io::Result<String> {
        let mut menu_stack = Vec::new();
        menu_stack.push(Menu::Title);
        // TODO: Remove this once menus are navigable.
        menu_stack.push(Menu::Game {
            game: Box::new(Game::with_gamemode(
                Gamemode::custom(
                    "Debug".to_string(),
                    NonZeroU32::try_from(10).unwrap(),
                    true,
                    None,
                    MeasureStat::Pieces(0),
                ),
                Instant::now(),
            )),
            game_screen_renderer: Default::default(),
            total_duration_paused: Duration::ZERO,
            last_paused: Instant::now(),
        });
        menu_stack.push(Menu::Game {
            game: Box::new(Game::with_gamemode(Gamemode::marathon(), Instant::now())),
            game_screen_renderer: Default::default(),
            total_duration_paused: Duration::ZERO,
            last_paused: Instant::now(),
        });
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
                Menu::GameOver => self.gameover(),
                Menu::GameComplete => self.gamecomplete(),
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
                    menu_stack.push(menu);
                }
                MenuUpdate::Set(menu) => {
                    menu_stack.clear();
                    menu_stack.push(menu);
                }
            }
        };
        // NOTE: This is done here manually for debug reasons in case the application still crashes somehow, c.f. note in `Drop::drop(self)`.
        let _ = self.term.execute(terminal::LeaveAlternateScreen);
        Ok(msg)
    }

    fn title(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: Title menu.
        Title
            -> { Quit }
        Title
            -> { NewGame Options Scores About }
            [color="#007FFF"]

        while event::poll(Duration::from_secs(0))? {
            match event::read()? {
                // Abort
                Event::Key(KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        state: _}) => {
                    break 'update_loop
                }
                // Handle common key inputs
                Event::Key(KeyEvent) => {
                    // TODO: handle key inputs!
                }
                Event::Resize(cols, rows) => {
                    // TODO: handle resize
                }
                // Console lost focus: Pause, re-enter update loop
                Event::FocusLost => {
                    // TODO: actively UNfocus application (requires flag)?
                    if let Screen::Gaming(_) = screen {
                        active_screens.push(Screen::Options);
                        continue 'update_loop
                    }
                }
                // Console gained focus: Do nothing, just let player continue
                Event::FocusGained => { }
                // NOTE We do not handle mouse events (yet?)
                Event::Mouse(MouseEvent) => { }
                // Ignore pasted text
                Event::Paste(String) => { }
            }
        }*/
        todo!("title menu")
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
        todo!("newgame menu")
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
        let time_unpaused = Instant::now();
        *total_duration_paused += time_unpaused.saturating_duration_since(*time_paused);
        // Prepare channel with which to communicate `Button` inputs / game interrupt.
        let mut buttons_pressed = ButtonsPressed::default();
        let (tx, rx) = mpsc::channel::<ButtonSignal>();
        let _input_handler =
            CrosstermHandler::new(&tx, &self.settings.keybinds, self.settings.kitty_enabled);
        // Game Loop
        let time_render_loop_start = Instant::now();
        let mut f = 0u32;
        let next_menu = 'render_loop: loop {
            f += 1;
            let next_frame_at = time_render_loop_start
                + Duration::from_secs_f64(f64::from(f) / self.settings.game_fps);
            let mut new_feedback_events = Vec::new();
            'idle_loop: loop {
                let frame_idle_remaining = next_frame_at - Instant::now();
                match rx.recv_timeout(frame_idle_remaining) {
                    Ok(None) => {
                        // TODO: Game pause directly quits: Remove this as soon as pause menu works.
                        return Ok(MenuUpdate::Push(Menu::Quit(
                            "[temporary but graceful game end - goodbye]".to_string(),
                        )));
                        break 'render_loop MenuUpdate::Push(Menu::Pause);
                    }
                    Ok(Some((instant, button, button_state))) => {
                        buttons_pressed[button] = button_state;
                        let instant = std::cmp::max(
                            instant - *total_duration_paused,
                            game.state().time_updated,
                        ); // Make sure button press
                           // SAFETY: We know game is not over and
                        new_feedback_events
                            .extend(game.update(Some(buttons_pressed), instant).unwrap());
                        continue 'idle_loop;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let now = Instant::now();
                        // TODO: SAFETY.
                        new_feedback_events
                            .extend(game.update(None, now - *total_duration_paused).unwrap());
                        break 'idle_loop;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // panic!("game loop RecvTimeoutError::Disconnected");
                        break 'render_loop MenuUpdate::Push(Menu::Pause);
                    }
                };
            }
            // TODO: Make this more elegantly modular.
            game_screen_renderer.render(self, game, new_feedback_events)?;
            // Exit if game ended
            if let Some(good_end) = game.finished() {
                let menu = if good_end.is_ok() {
                    Menu::GameComplete
                } else {
                    Menu::GameOver
                };
                break MenuUpdate::Push(menu);
            }
        };
        *time_paused = Instant::now();
        Ok(next_menu)
    }

    fn gameover(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: Gameover menu.
        GameOver
            -> { Quit }
        GameOver
            -> { NewGame Scores }
            [color="#007FFF"]
        */
        todo!("gameover menu")
    }

    fn gamecomplete(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: Gamecomplete menu.
        GameComplete
            -> { Quit }
        GameComplete
            -> { NewGame Scores }
            [color="#007FFF"]
        */
        todo!("game complete menu")
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
        todo!("pause menu")
    }

    fn options(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: Options menu.
        Options
            -> { ConfigureControls }
            [color="#007FFF"]

        MenuUpdate::Pop
        */
        todo!("options menu")
    }

    fn configurecontrols(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: Configurecontrols menu.

        MenuUpdate::Pop
        */
        todo!("configure controls menu")
    }

    fn scores(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: Scores menu.

        MenuUpdate::Pop
        */
        todo!("highscores menu")
    }

    fn about(&mut self) -> io::Result<MenuUpdate> {
        /* TODO: About menu.

        MenuUpdate::Pop
        */
        todo!("About menu")
    }
}
