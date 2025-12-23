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

use super::InputSignal;

#[derive(Debug)]
pub struct CrosstermHandler {
    handle: Option<(Arc<AtomicBool>, JoinHandle<()>)>,
}

impl Drop for CrosstermHandler {
    fn drop(&mut self) {
        if let Some((run_thread_flag, _)) = self.handle.take() {
            run_thread_flag.store(false, Ordering::Release);
        }
    }
}

impl CrosstermHandler {
    pub fn new(
        input_sender: &Sender<InputSignal>,
        keybinds: &HashMap<KeyCode, Button>,
        kitty_enabled: bool,
    ) -> Self {
        let run_thread_flag = Arc::new(AtomicBool::new(true));
        let join_handle = if kitty_enabled {
            Self::spawn_kitty
        } else {
            Self::spawn_standard
        }(
            run_thread_flag.clone(),
            input_sender.clone(),
            keybinds.clone(),
        );
        CrosstermHandler {
            handle: Some((run_thread_flag, join_handle)),
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
            (KeyCode::Char(' '), Button::HoldPiece),
        ])
    }

    pub fn vim_keybinds() -> HashMap<KeyCode, Button> {
        HashMap::from([
            (KeyCode::Char('h'), Button::DropSoft),
            (KeyCode::Char('j'), Button::MoveLeft),
            (KeyCode::Char('k'), Button::RotateRight),
            (KeyCode::Char('l'), Button::MoveRight),
            (KeyCode::Char(' '), Button::DropHard),
            (KeyCode::Char('c'), Button::HoldPiece),
            (KeyCode::Char('z'), Button::RotateLeft),
        ])
    }

    fn spawn_standard(
        run_thread_flag: Arc<AtomicBool>,
        input_sender: Sender<InputSignal>,
        keybinds: HashMap<KeyCode, Button>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            'react_to_event: loop {
                // Maybe stop thread.
                let run_thread = run_thread_flag.load(Ordering::Acquire);
                if !run_thread {
                    break 'react_to_event;
                };
                match event::read() {
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press | KeyEventKind::Repeat,
                        ..
                    })) => {
                        let _ = input_sender.send(InputSignal::AbortProgram);
                        break 'react_to_event;
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('d'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = input_sender.send(InputSignal::ForfeitGame);
                        break 'react_to_event;
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('s'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = input_sender.send(InputSignal::TakeSnapshot);
                    }
                    // Escape pressed: send pause.
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = input_sender.send(InputSignal::Pause);
                        break 'react_to_event;
                    }
                    Ok(Event::Resize(..)) => {
                        let _ = input_sender.send(InputSignal::WindowResize);
                    }
                    // Candidate key pressed.
                    Ok(Event::Key(KeyEvent {
                        code: key,
                        kind: KeyEventKind::Press | KeyEventKind::Repeat,
                        ..
                    })) => {
                        if let Some(&button) = keybinds.get(&key) {
                            // Binding found: send button press.
                            let now = Instant::now();
                            let _ = input_sender.send(InputSignal::ButtonInput(button, true, now));
                            let _ = input_sender.send(InputSignal::ButtonInput(button, false, now));
                        }
                    }
                    // Don't care about other events: ignore.
                    _ => {}
                };
            }
        })
    }

    fn spawn_kitty(
        run_thread_flag: Arc<AtomicBool>,
        input_sender: Sender<InputSignal>,
        keybinds: HashMap<KeyCode, Button>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            'react_to_event: loop {
                // Maybe stop thread.
                let run_thread = run_thread_flag.load(Ordering::Acquire);
                if !run_thread {
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
                        kind: KeyEventKind::Press | KeyEventKind::Repeat,
                        ..
                    })) => {
                        let _ = input_sender.send(InputSignal::AbortProgram);
                        break 'react_to_event;
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('d'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = input_sender.send(InputSignal::ForfeitGame);
                        break 'react_to_event;
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('s'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = input_sender.send(InputSignal::TakeSnapshot);
                    }
                    // Escape pressed: send pause.
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    })) => {
                        let _ = input_sender.send(InputSignal::Pause);
                        break 'react_to_event;
                    }
                    Ok(Event::Resize(..)) => {
                        let _ = input_sender.send(InputSignal::WindowResize);
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
                            let _ = input_sender.send(InputSignal::ButtonInput(
                                button,
                                kind == KeyEventKind::Press,
                                Instant::now(),
                            ));
                        }
                    },
                    // Don't care about other events: ignore.
                    _ => {}
                };
            }
        })
    }
}
