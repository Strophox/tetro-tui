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

pub type Keybinds = std::collections::HashMap<crossterm::event::KeyCode, tetrs_engine::Button>;

#[derive(Debug)]
pub struct TerminalInputHandler {
    _thread_handle: JoinHandle<()>,
    running_thread_flag: Arc<AtomicBool>,
}

impl Drop for TerminalInputHandler {
    fn drop(&mut self) {
        self.running_thread_flag.store(false, Ordering::Release);
    }
}

impl TerminalInputHandler {
    pub fn new(
        input_sender: &Sender<InputSignal>,
        keybinds: &Keybinds,
        kitty_enabled: bool,
    ) -> Self {
        let running_thread_flag = Arc::new(AtomicBool::new(true));
        let spawn = if kitty_enabled {
            Self::spawn_kitty
        } else {
            Self::spawn_standard
        };
        TerminalInputHandler {
            _thread_handle: spawn(
                running_thread_flag.clone(),
                input_sender.clone(),
                keybinds.clone(),
            ),
            running_thread_flag,
        }
    }

    pub fn tetrs_default_keybinds() -> Keybinds {
        HashMap::from([
            (KeyCode::Left, Button::MoveLeft),
            (KeyCode::Right, Button::MoveRight),
            (KeyCode::Char('a'), Button::RotateLeft),
            (KeyCode::Char('d'), Button::RotateRight),
            (KeyCode::Char('s'), Button::RotateAround),
            (KeyCode::Down, Button::DropSoft),
            (KeyCode::Up, Button::DropHard),
            //(KeyCode::Char('w'), Button::DropSonic),
            (KeyCode::Char(' '), Button::HoldPiece),
        ])
    }

    pub fn vim_keybinds() -> Keybinds {
        HashMap::from([
            (KeyCode::Char('h'), Button::MoveLeft),
            (KeyCode::Char('l'), Button::MoveRight),
            (KeyCode::Char('a'), Button::RotateLeft),
            (KeyCode::Char('d'), Button::RotateRight),
            //(KeyCode::Char('s'), Button::RotateAround),
            (KeyCode::Char('j'), Button::DropSoft),
            (KeyCode::Char('k'), Button::DropHard),
            //(KeyCode::Char('w'), Button::DropSonic),
            (KeyCode::Char(' '), Button::HoldPiece),
        ])
    }

    pub fn guideline_keybinds() -> Keybinds {
        use crossterm::event::ModifierKeyCode as M;
        HashMap::from([
            (KeyCode::Left, Button::MoveLeft),
            (KeyCode::Right, Button::MoveRight),
            (KeyCode::Char('z'), Button::RotateLeft),
            (KeyCode::Char('y'), Button::RotateLeft), // 'Branch-predicting' European keyboards.
            (KeyCode::Modifier(M::LeftControl), Button::RotateLeft),
            (KeyCode::Modifier(M::RightControl), Button::RotateLeft),
            (KeyCode::Char('x'), Button::RotateRight),
            (KeyCode::Up, Button::RotateRight),
            (KeyCode::Down, Button::DropSoft),
            (KeyCode::Char('c'), Button::HoldPiece),
            (KeyCode::Modifier(M::LeftShift), Button::HoldPiece),
            (KeyCode::Modifier(M::RightShift), Button::HoldPiece),
        ])
    }

    fn spawn_standard(
        run_thread_flag: Arc<AtomicBool>,
        input_sender: Sender<InputSignal>,
        keybinds: Keybinds,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            'react_to_event: loop {
                // Maybe stop thread.
                let true = run_thread_flag.load(Ordering::Acquire) else {
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
                        kind: KeyEventKind::Press | KeyEventKind::Repeat,
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
        keybinds: Keybinds,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            'react_to_event: loop {
                // Maybe stop thread.
                let true = run_thread_flag.load(Ordering::Acquire) else {
                    break 'react_to_event;
                };
                // FIXME(Strophox): I think this code is obsolete. But reminds me of the issue where Kitty's "release" event is not captured by the game if one pauses during a press.
                // match event::poll(std::time::Duration::from_secs(1)) {
                //     Ok(true) => {}
                //     Ok(false) | Err(_) => continue 'react_to_event,
                // }
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
                        kind: KeyEventKind::Press | KeyEventKind::Repeat,
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
                            // FIXME: This module could be refactored by handling all the `let _ = input_sender.send(..)` lines and automatically stopping the thread, possibly removing the need for a synchronized run_thread flag in the first place.
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
