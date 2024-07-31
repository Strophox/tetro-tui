use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use tetrs_engine::Button;

pub type ButtonOrSignal = Result<(Instant, Button, bool), Signal>;

pub enum Signal {
    WindowResize,
    Pause,
    ForfeitGame,
    ExitProgram,
}

#[derive(Debug)]
pub struct CrosstermHandler {
    _handle: Option<(JoinHandle<()>, Arc<AtomicBool>)>,
}

impl Drop for CrosstermHandler {
    fn drop(&mut self) {
        if let Some((_, flag)) = self._handle.take() {
            flag.store(false, Ordering::Release);
        }
    }
}

impl CrosstermHandler {
    pub fn new(
        sender: &Sender<ButtonOrSignal>,
        keybinds: &HashMap<KeyCode, Button>,
        kitty_enabled: bool,
    ) -> Self {
        let spawn = if kitty_enabled {
            Self::spawn_kitty
        } else {
            Self::spawn_standard
        };
        let flag = Arc::new(AtomicBool::new(true));
        CrosstermHandler {
            _handle: Some((spawn(sender.clone(), flag.clone(), keybinds.clone()), flag)),
        }
    }

    pub fn default_keybinds() -> HashMap<KeyCode, Button> {
        HashMap::from([
            (KeyCode::Left, Button::MoveLeft),
            (KeyCode::Right, Button::MoveRight),
            (KeyCode::Char('a'), Button::RotateLeft),
            (KeyCode::Char('d'), Button::RotateRight),
            (KeyCode::Down, Button::DropSoft),
            (KeyCode::Up, Button::DropHard),
        ])
    }

    fn spawn_standard(
        sender: Sender<ButtonOrSignal>,
        flag: Arc<AtomicBool>,
        keybinds: HashMap<KeyCode, Button>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                // Maybe stop thread.
                let running = flag.load(Ordering::Acquire);
                if !running {
                    break;
                };
                match event::read() {
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) => {
                        let _ = sender.send(Err(Signal::ExitProgram));
                        break;
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('d'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) => {
                        let _ = sender.send(Err(Signal::ForfeitGame));
                        break;
                    }
                    // Escape pressed: send pause.
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = sender.send(Err(Signal::Pause));
                        break;
                    }
                    Ok(Event::Resize(..)) => {
                        let _ = sender.send(Err(Signal::WindowResize));
                    }
                    // Candidate key pressed.
                    Ok(Event::Key(KeyEvent {
                        code: key,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        if let Some(&button) = keybinds.get(&key) {
                            // Binding found: send button press.
                            let now = Instant::now();
                            let _ = sender.send(Ok((now, button, true)));
                            let _ = sender.send(Ok((now, button, false)));
                        }
                    }
                    // Don't care about other events: ignore.
                    _ => {}
                };
            }
        })
    }

    fn spawn_kitty(
        sender: Sender<ButtonOrSignal>,
        flag: Arc<AtomicBool>,
        keybinds: HashMap<KeyCode, Button>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                // Maybe stop thread.
                let running = flag.load(Ordering::Acquire);
                if !running {
                    break;
                };
                match event::read() {
                    // Direct interrupt.
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) => {
                        let _ = sender.send(Err(Signal::ExitProgram));
                        break;
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('d'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) => {
                        let _ = sender.send(Err(Signal::ForfeitGame));
                        break;
                    }
                    // Escape pressed: send pause.
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = sender.send(Err(Signal::Pause));
                        break;
                    }
                    Ok(Event::Resize(..)) => {
                        let _ = sender.send(Err(Signal::WindowResize));
                    }
                    // TTY simulated press repeat: ignore.
                    Ok(Event::Key(KeyEvent {
                        kind: KeyEventKind::Repeat,
                        ..
                    })) => {}
                    // Candidate key actually changed.
                    Ok(Event::Key(KeyEvent { code, kind, .. })) => match keybinds.get(&code) {
                        // No binding: ignore.
                        None => {}
                        // Binding found: send button un-/press.
                        Some(&button) => {
                            let _ = sender.send(Ok((
                                Instant::now(),
                                button,
                                kind == KeyEventKind::Press,
                            )));
                        }
                    },
                    // Don't care about other events: ignore.
                    _ => {}
                };
            }
        })
    }
}
