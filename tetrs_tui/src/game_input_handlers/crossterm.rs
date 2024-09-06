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
    handles: Option<(Arc<AtomicBool>, JoinHandle<()>)>,
}

impl Drop for CrosstermHandler {
    fn drop(&mut self) {
        if let Some((flag, _)) = self.handles.take() {
            flag.store(false, Ordering::Release);
        }
    }
}

impl CrosstermHandler {
    pub fn new(
        button_sender: &Sender<InputOrInterrupt>,
        keybinds: &HashMap<KeyCode, Button>,
        kitty_enabled: bool,
    ) -> Self {
        let flag = Arc::new(AtomicBool::new(true));
        let join_handle = if kitty_enabled {
            Self::spawn_kitty
        } else {
            Self::spawn_standard
        }(flag.clone(), button_sender.clone(), keybinds.clone());
        CrosstermHandler {
            handles: Some((flag, join_handle)),
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
        flag: Arc<AtomicBool>,
        button_sender: Sender<InputOrInterrupt>,
        keybinds: HashMap<KeyCode, Button>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            'react_to_event: loop {
                // Maybe stop thread.
                let running = flag.load(Ordering::Acquire);
                if !running {
                    break 'react_to_event;
                };
                match event::read() {
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) => {
                        let _ = button_sender.send(Err(Interrupt::ExitProgram));
                        break 'react_to_event;
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('d'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) => {
                        let _ = button_sender.send(Err(Interrupt::ForfeitGame));
                        break 'react_to_event;
                    }
                    // Escape pressed: send pause.
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = button_sender.send(Err(Interrupt::Pause));
                        break 'react_to_event;
                    }
                    Ok(Event::Resize(..)) => {
                        let _ = button_sender.send(Err(Interrupt::WindowResize));
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
                            let _ = button_sender.send(Ok((now, button, true)));
                            let _ = button_sender.send(Ok((now, button, false)));
                        }
                    }
                    // Don't care about other events: ignore.
                    _ => {}
                };
            }
        })
    }

    fn spawn_kitty(
        flag: Arc<AtomicBool>,
        button_sender: Sender<InputOrInterrupt>,
        keybinds: HashMap<KeyCode, Button>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            'react_to_event: loop {
                // Maybe stop thread.
                let running = flag.load(Ordering::Acquire);
                if !running {
                    break 'react_to_event;
                };
                match event::poll(std::time::Duration::from_secs(1)) {
                    Ok(true) => {}
                    Ok(false) | Err(_) => continue 'react_to_event,
                }
                match event::read() {
                    // Direct interrupt.
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) => {
                        let _ = button_sender.send(Err(Interrupt::ExitProgram));
                        break 'react_to_event;
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('d'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) => {
                        let _ = button_sender.send(Err(Interrupt::ForfeitGame));
                        break 'react_to_event;
                    }
                    // Escape pressed: send pause.
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = button_sender.send(Err(Interrupt::Pause));
                        break 'react_to_event;
                    }
                    Ok(Event::Resize(..)) => {
                        let _ = button_sender.send(Err(Interrupt::WindowResize));
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
                            let _ = button_sender.send(Ok((
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
