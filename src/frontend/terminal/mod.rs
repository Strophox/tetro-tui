pub mod input_handlers;

use std::{
    collections::{HashMap, VecDeque},
    io::{self, Write},
    num::NonZeroU64,
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style, terminal, ExecutableCommand, QueueableCommand,
};
use input_handlers::{new_input_handler_crossterm, ButtonSignal, CT_Keycode, DQ_Keycode};

use crate::backend::game::{
    Button, ButtonsPressed, FeedbackEvent, Game, GameState, Gamemode, MeasureStat,
};

// TODO: Is `PartialEq` needed?
#[derive(PartialEq, Clone, Debug)]
struct Settings {
    game_fps: f64,
    keybinds: HashMap<CT_Keycode, Button>,
    // TODO: What's the information stored throughout the entire application?
}

#[derive(Debug)]
enum Menu {
    Title,
    NewGame(Gamemode),
    Game(Box<Game>, Duration, Instant),
    Pause,
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
    SetTo(Menu),
}

impl Menu {
    fn title(w: &mut dyn Write) -> io::Result<MenuUpdate> {
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

    fn newgame(w: &mut dyn Write, gamemode: &mut Gamemode) -> io::Result<MenuUpdate> {
        todo!("new game screen") // TODO:
    }

    fn game(
        w: &mut dyn Write,
        settings: &Settings,
        game: &mut Game,
        duration_paused_total: &mut Duration,
        time_paused: &mut Instant,
    ) -> io::Result<MenuUpdate> {
        let time_unpaused = Instant::now();
        *duration_paused_total += time_unpaused.saturating_duration_since(*time_paused);
        // Prepare channel with which to communicate `Button` inputs / game interrupt.
        let mut buttons_pressed = ButtonsPressed::default();
        let (tx, rx) = mpsc::channel::<ButtonSignal>();
        let _input_handler = new_input_handler_crossterm(&tx, &settings.keybinds);
        // TODO: Remove these debug structs.
        let mut feed_evt_msg_buf = VecDeque::new();
        // Game Loop
        let time_render_loop_start = Instant::now();
        let mut it = 0u32;
        let next_menu = 'render_loop: loop {
            it += 1;
            let next_frame =
                time_render_loop_start + Duration::from_secs_f64(f64::from(it) / settings.game_fps);
            let mut feedback_events = Vec::new();
            'idle_loop: loop {
                let frame_idle_remaining = next_frame - Instant::now();
                match rx.recv_timeout(frame_idle_remaining) {
                    Ok(None) => {
                        // TODO: Remove.
                        return Ok(MenuUpdate::Push(Menu::Quit(
                            "[temporary but graceful game end - goodbye]".to_string(),
                        )));
                        break 'render_loop MenuUpdate::Push(Menu::Pause);
                    }
                    Ok(Some((instant, button, button_state))) => {
                        buttons_pressed[button] = button_state;
                        let new_feedback_events =
                            game.update(Some(buttons_pressed), instant - *duration_paused_total);
                        feedback_events.extend(new_feedback_events);
                        continue 'idle_loop;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let now = Instant::now();
                        let new_feedback_events = game.update(None, now - *duration_paused_total);
                        feedback_events.extend(new_feedback_events);
                        break 'idle_loop;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // TODO: SAFETY?
                        unreachable!("game loop RecvTimeoutError::Disconnected");
                    }
                };
            }
            // TODO: Draw game.
            let GameState {
                lines_cleared,
                level,
                score,
                time_elapsed,
                board,
                active_piece,
                next_pieces,
            } = game.state();
            let mut temp_board = board.clone();
            if let Some(active_piece) = active_piece {
                for ((x, y), tile_type_id) in active_piece.tiles() {
                    temp_board[y][x] = Some(tile_type_id);
                }
            }
            w.queue(terminal::Clear(terminal::ClearType::All))?
                .queue(cursor::MoveTo(0, 0))?;
            w.queue(style::Print("   +--------------------+"))?
                .queue(cursor::MoveToNextLine(1))?;
            for (idx, line) in temp_board.iter().take(20).enumerate().rev() {
                let txt_line = format!(
                    "{idx:02} |{}|",
                    line.iter()
                        .map(|cell| {
                            cell.map_or(" .", |mino| match mino.get() {
                                1 => "OO",
                                2 => "II",
                                3 => "SS",
                                4 => "ZZ",
                                5 => "TT",
                                6 => "LL",
                                7 => "JJ",
                                _ => todo!("formatting unknown mino type"),
                            })
                        })
                        .collect::<Vec<_>>()
                        .join("")
                );
                w.queue(style::Print(txt_line))?
                    .queue(cursor::MoveToNextLine(1))?;
            }
            w.queue(style::Print("   +--------------------+"))?
                .queue(cursor::MoveToNextLine(1))?;
            w.queue(style::Print(format!("   {time_elapsed:?}")))?
                .queue(cursor::MoveToNextLine(1))?;
            // TODO: Do something with feedback events.
            for (_, feedback_event) in feedback_events {
                let str = match feedback_event {
                    FeedbackEvent::Accolade(
                        tetromino,
                        spin,
                        n_lines_cleared,
                        combo,
                        perfect_clear,
                    ) => {
                        let mut txts = Vec::new();
                        if spin {
                            txts.push(format!("{tetromino:?}-Spin"))
                        }
                        let txt_lineclear = match n_lines_cleared {
                            1 => "Single!",
                            2 => "Double!",
                            3 => "Triple!",
                            4 => "Quadruple!",
                            _ => todo!("unformatted line clear count"),
                        }
                        .to_string();
                        txts.push(txt_lineclear);
                        if combo > 1 {
                            txts.push(format!("[ x{combo} ]"));
                        }
                        if perfect_clear {
                            txts.push("PERFECT!".to_string());
                        }
                        txts.join(" ")
                    }
                    FeedbackEvent::PieceLocked(_) => continue,
                    FeedbackEvent::LineClears(_) => continue,
                    FeedbackEvent::HardDrop(_, _) => continue,
                    FeedbackEvent::Debug(s) => s,
                };
                feed_evt_msg_buf.push_front(str);
            }
            feed_evt_msg_buf.truncate(8);
            for str in feed_evt_msg_buf.iter() {
                w.queue(style::Print(str))?
                    .queue(cursor::MoveToNextLine(1))?;
            }
            // Execute draw.
            w.flush()?;
            // Exit if game ended
            if let Some(good_end) = game.finished() {
                let menu = if good_end {
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

    fn pause(w: &mut dyn Write) -> io::Result<MenuUpdate> {
        todo!("pause screen") // TODO:
    }

    fn gameover(w: &mut dyn Write) -> io::Result<MenuUpdate> {
        todo!("gameover screen") // TODO:
    }

    fn gamecomplete(w: &mut dyn Write) -> io::Result<MenuUpdate> {
        todo!("game complete screen") // TODO:
    }

    fn options(w: &mut dyn Write, settings: &mut Settings) -> io::Result<MenuUpdate> {
        todo!("options screen") // TODO:
    }

    fn configurecontrols(w: &mut dyn Write, settings: &mut Settings) -> io::Result<MenuUpdate> {
        todo!("configure controls screen") // TODO:
    }

    fn replay(w: &mut dyn Write) -> io::Result<MenuUpdate> {
        todo!("replay screen") // TODO:
    }

    fn scores(w: &mut dyn Write) -> io::Result<MenuUpdate> {
        todo!("highscores screen") // TODO:
    }
}

pub fn run(w: &mut impl Write) -> io::Result<String> {
    // Console prologue: Initializion.
    // TODO: Use kitty someday `w.execute(event::PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES))?;`.
    w.execute(cursor::Hide)?;
    w.execute(terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    // Preparing main game loop loop.
    // TODO: Store different keybind mappings somewhere and get default from there.
    let _dq_keybinds = HashMap::from([
        (DQ_Keycode::Left, Button::MoveLeft),
        (DQ_Keycode::Right, Button::MoveRight),
        (DQ_Keycode::A, Button::RotateLeft),
        (DQ_Keycode::D, Button::RotateRight),
        (DQ_Keycode::Down, Button::DropSoft),
        (DQ_Keycode::Up, Button::DropHard),
    ]);
    let ct_keybinds = HashMap::from([
        (CT_Keycode::Left, Button::MoveLeft),
        (CT_Keycode::Right, Button::MoveRight),
        (CT_Keycode::Char('a'), Button::RotateLeft),
        (CT_Keycode::Char('d'), Button::RotateRight),
        (CT_Keycode::Down, Button::DropSoft),
        (CT_Keycode::Up, Button::DropHard),
    ]);
    let mut settings = Settings {
        keybinds: ct_keybinds,
        game_fps: 24.0,
    };
    let mut menu_stack = Vec::new();
    menu_stack.push(Menu::Title);
    menu_stack.push(Menu::Game(
        Box::new(Game::with_gamemode(
            Gamemode::custom(
                "Debug".to_string(),
                NonZeroU64::new(1).unwrap(),
                true,
                None,
                MeasureStat::Pieces(0),
            ),
            Instant::now(),
        )),
        Duration::ZERO,
        Instant::now(),
    )); // TODO: Remove this once menus are navigable.
    let msg = loop {
        // Retrieve active menu, stop application if stack is empty.
        let Some(screen) = menu_stack.last_mut() else {
            break String::from("all menus exited");
        };
        // Open new menu screen, then store what it returns.
        let menu_update = match screen {
            Menu::Title => Menu::title(w),
            Menu::NewGame(gamemode) => Menu::newgame(w, gamemode),
            Menu::Game(game, total_duration_paused, last_paused) => {
                Menu::game(w, &settings, game, total_duration_paused, last_paused)
            }
            Menu::Pause => Menu::pause(w),
            Menu::GameOver => Menu::gameover(w),
            Menu::GameComplete => Menu::gamecomplete(w),
            Menu::Options => Menu::options(w, &mut settings),
            Menu::ConfigureControls => Menu::configurecontrols(w, &mut settings),
            Menu::Replay => Menu::replay(w),
            Menu::Scores => Menu::scores(w),
            Menu::Quit(string) => break string.clone(), // TODO: Optimize away `.clone()` call.
        }?;
        // Change screen session depending on what response screen gave.
        match menu_update {
            MenuUpdate::Pop => {
                menu_stack.pop();
            }
            MenuUpdate::Push(menu) => {
                menu_stack.push(menu);
            }
            MenuUpdate::SetTo(menu) => {
                menu_stack.clear();
                menu_stack.push(menu);
            }
        }
    };
    // Console epilogue: de-initialization.
    // TODO: use kitty someday `w.execute(event::PopKeyboardEnhancementFlags)?;`.
    terminal::disable_raw_mode()?;
    w.execute(terminal::LeaveAlternateScreen)?;
    w.execute(style::ResetColor)?;
    w.execute(cursor::Show)?;
    Ok(msg)
}
