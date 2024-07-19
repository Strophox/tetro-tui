mod game_input_handler;
mod game_renderer;

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
use game_input_handler::{ButtonSignal, CT_Keycode, CrosstermHandler};
use game_renderer::GameRenderer;

use crate::backend::game::{Button, ButtonsPressed, Game, Gamemode, MeasureStat};

#[derive(Debug)]
enum Menu {
    Title,
    NewGame(Gamemode),
    Game(Box<Game>, GameRenderer, Duration, Instant),
    Pause, // TODO: Add information so game stats can be displayed here.
    GameOver,
    GameComplete,
    Options,
    Replay,
    Scores,
    Quit(String),
    ConfigureControls,
}

#[derive(Debug)]
enum MenuUpdate {
    Pop,
    Push(Menu),
    Set(Menu),
}

// TODO: Derive `Default`?
#[derive(PartialEq, Clone, Debug)]
struct Settings {
    game_fps: f64,
    keybinds: HashMap<CT_Keycode, Button>,
    kitty_enabled: bool,
}

#[derive(Debug)]
pub struct TetrsTerminal<T: Write> {
    term: T,
    settings: Settings,
}

impl<T: Write> Drop for TetrsTerminal<T> {
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

impl<T: Write> TetrsTerminal<T> {
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
        menu_stack.push(Menu::Game(
            Box::new(Game::with_gamemode(
                Gamemode::custom(
                    "Debug".to_string(),
                    NonZeroU32::MIN,
                    true,
                    None,
                    MeasureStat::Pieces(0),
                ),
                Instant::now(),
            )),
            GameRenderer::default(),
            Duration::ZERO,
            Instant::now(),
        ));
        // menu_stack.push(Menu::Game(
        //     Box::new(Game::with_gamemode(Gamemode::master(), Instant::now())),
        //     Duration::ZERO,
        //     Instant::now(),
        // ));
        // Preparing main application loop.
        let msg = loop {
            // Retrieve active menu, stop application if stack is empty.
            let Some(screen) = menu_stack.last_mut() else {
                break String::from("all menus exited");
            };
            // Open new menu screen, then store what it returns.
            let menu_update = match screen {
                Menu::Title => self.menu_title(),
                Menu::NewGame(gamemode) => self.menu_newgame(gamemode),
                Menu::Game(game, renderer, total_duration_paused, last_paused) => {
                    self.menu_game(game, renderer, total_duration_paused, last_paused)
                }
                Menu::Pause => self.menu_pause(),
                Menu::GameOver => self.menu_gameover(),
                Menu::GameComplete => self.menu_gamecomplete(),
                Menu::Options => self.menu_options(),
                Menu::ConfigureControls => self.menu_configurecontrols(),
                Menu::Replay => self.menu_replay(),
                Menu::Scores => self.menu_scores(),
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

    fn menu_title(&mut self) -> io::Result<MenuUpdate> {
        todo!("title screen")
        /* TODO:
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
    }

    fn menu_newgame(&mut self, gamemode: &mut Gamemode) -> io::Result<MenuUpdate> {
        todo!("new game screen") // TODO:
    }

    fn menu_game(
        &mut self,
        game: &mut Game,
        renderer: &mut GameRenderer,
        duration_paused_total: &mut Duration,
        time_paused: &mut Instant,
    ) -> io::Result<MenuUpdate> {
        let time_unpaused = Instant::now();
        *duration_paused_total += time_unpaused.saturating_duration_since(*time_paused);
        // Prepare channel with which to communicate `Button` inputs / game interrupt.
        let mut buttons_pressed = ButtonsPressed::default();
        let (tx, rx) = mpsc::channel::<ButtonSignal>();
        let _input_handler =
            CrosstermHandler::new(&tx, &self.settings.keybinds, self.settings.kitty_enabled);
        // Game Loop
        let time_render_loop_start = Instant::now();
        let mut it = 0u32;
        let next_menu = 'render_loop: loop {
            it += 1;
            let next_frame = time_render_loop_start
                + Duration::from_secs_f64(f64::from(it) / self.settings.game_fps);
            let mut feedback_events = Vec::new();
            'idle_loop: loop {
                let frame_idle_remaining = next_frame - Instant::now();
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
                            instant - *duration_paused_total,
                            game.state().time_updated,
                        ); // Make sure button press
                           // SAFETY: We know game is not over and
                        let new_feedback_events =
                            game.update(Some(buttons_pressed), instant).unwrap();
                        feedback_events.extend(new_feedback_events);
                        continue 'idle_loop;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let now = Instant::now();
                        // TODO: SAFETY.
                        let new_feedback_events =
                            game.update(None, now - *duration_paused_total).unwrap();
                        feedback_events.extend(new_feedback_events);
                        break 'idle_loop;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // panic!("game loop RecvTimeoutError::Disconnected");
                        break 'render_loop MenuUpdate::Push(Menu::Pause);
                    }
                };
            }
            // TODO: Make this more elegantly modular.
            renderer.render_dbg(self, game, feedback_events)?;
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

    fn menu_pause(&mut self) -> io::Result<MenuUpdate> {
        todo!("pause screen") // TODO:
    }

    fn menu_gameover(&mut self) -> io::Result<MenuUpdate> {
        todo!("gameover screen") // TODO:
    }

    fn menu_gamecomplete(&mut self) -> io::Result<MenuUpdate> {
        todo!("game complete screen") // TODO:
    }

    fn menu_options(&mut self) -> io::Result<MenuUpdate> {
        todo!("options screen") // TODO:
    }

    fn menu_configurecontrols(&mut self) -> io::Result<MenuUpdate> {
        todo!("configure controls screen") // TODO:
    }

    fn menu_replay(&mut self) -> io::Result<MenuUpdate> {
        todo!("replay screen") // TODO:
    }

    fn menu_scores(&mut self) -> io::Result<MenuUpdate> {
        todo!("highscores screen") // TODO:
    }
}
