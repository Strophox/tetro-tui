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

use crate::game_input_handlers::{InputOrInterrupt, Interrupt};

#[derive(Debug)]
pub struct CrosstermHandler {
    handle: Option<(JoinHandle<()>, Arc<AtomicBool>)>,
}

impl Drop for CrosstermHandler {
    fn drop(&mut self) {
        if let Some((_, flag)) = self.handle.take() {
            flag.store(false, Ordering::Release);
        }
    }
}

impl CrosstermHandler {
    pub fn new(
        sender: &Sender<InputOrInterrupt>,
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
            handle: Some((spawn(sender.clone(), flag.clone(), keybinds.clone()), flag)),
        }
    }

    pub fn default_keybinds() -> HashMap<KeyCode, Button> {
        HashMap::from([
            (KeyCode::Left, Button::MoveLeft),
            (KeyCode::Right, Button::MoveRight),
            (KeyCode::Char('a'), Button::RotateLeft),
            (KeyCode::Char('d'), Button::RotateRight),
            //(KeyCode::Char('s'), Button::RotateAround),
            (KeyCode::Down, Button::DropSoft),
            (KeyCode::Up, Button::DropHard),
            //(KeyCode::Char('w'), Button::DropSonic),
            (KeyCode::Char(' '), Button::Hold),
        ])
    }

    fn spawn_standard(
        sender: Sender<InputOrInterrupt>,
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
                        let _ = sender.send(Err(Interrupt::ExitProgram));
                        break;
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('d'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) => {
                        let _ = sender.send(Err(Interrupt::ForfeitGame));
                        break;
                    }
                    // Escape pressed: send pause.
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = sender.send(Err(Interrupt::Pause));
                        break;
                    }
                    Ok(Event::Resize(..)) => {
                        let _ = sender.send(Err(Interrupt::WindowResize));
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
        sender: Sender<InputOrInterrupt>,
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
                        let _ = sender.send(Err(Interrupt::ExitProgram));
                        break;
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('d'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) => {
                        let _ = sender.send(Err(Interrupt::ForfeitGame));
                        break;
                    }
                    // Escape pressed: send pause.
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = sender.send(Err(Interrupt::Pause));
                        break;
                    }
                    Ok(Event::Resize(..)) => {
                        let _ = sender.send(Err(Interrupt::WindowResize));
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
