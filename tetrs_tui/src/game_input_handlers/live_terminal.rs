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

use tetrs_engine::{Button, ButtonChange};

use super::InputSignal;

pub type Keybinds = HashMap<(KeyCode, KeyModifiers), Button>;

pub fn tetrs_default_keybinds() -> Keybinds {
    let keybinds_tetrs: [((KeyCode, KeyModifiers), Button); 8] = [
        (KeyCode::Left, Button::MoveLeft),
        (KeyCode::Right, Button::MoveRight),
        (KeyCode::Char('a'), Button::RotateLeft),
        (KeyCode::Char('d'), Button::RotateRight),
        (KeyCode::Char('s'), Button::RotateAround),
        (KeyCode::Down, Button::DropSoft),
        (KeyCode::Up, Button::DropHard),
        //(KeyCode::Char('w'), Button::DropSonic),
        (KeyCode::Char(' '), Button::HoldPiece),
    ].map(|(k,b)| ((k, KeyModifiers::NONE), b));
    HashMap::from(keybinds_tetrs)
}

pub fn vim_keybinds() -> Keybinds {
    let keybinds_vim: [((KeyCode, KeyModifiers), Button); 7] = [
        (KeyCode::Char('h'), Button::MoveLeft),
        (KeyCode::Char('l'), Button::MoveRight),
        (KeyCode::Char('a'), Button::RotateLeft),
        (KeyCode::Char('d'), Button::RotateRight),
        //(KeyCode::Char('s'), Button::RotateAround),
        (KeyCode::Char('j'), Button::DropSoft),
        (KeyCode::Char('k'), Button::DropHard),
        //(KeyCode::Char('w'), Button::DropSonic),
        (KeyCode::Char(' '), Button::HoldPiece),
    ].map(|(k,b)| ((k, KeyModifiers::NONE), b));
    HashMap::from(keybinds_vim)
}

pub fn guideline_keybinds() -> Keybinds {
    use crossterm::event::ModifierKeyCode as M;
    let keybinds_guidelinle: [((KeyCode, KeyModifiers), Button); 13] = [
        (KeyCode::Left, Button::MoveLeft),
        (KeyCode::Right, Button::MoveRight),
        (KeyCode::Char('z'), Button::RotateLeft),
        (KeyCode::Char('y'), Button::RotateLeft), // 'Branch-predicting' European keyboards.
        (KeyCode::Modifier(M::LeftControl), Button::RotateLeft),
        (KeyCode::Modifier(M::RightControl), Button::RotateLeft),
        (KeyCode::Char('x'), Button::RotateRight),
        (KeyCode::Up, Button::RotateRight),
        (KeyCode::Down, Button::DropSoft),
        (KeyCode::Char(' '), Button::DropHard),
        (KeyCode::Char('c'), Button::HoldPiece),
        (KeyCode::Modifier(M::LeftShift), Button::HoldPiece),
        (KeyCode::Modifier(M::RightShift), Button::HoldPiece),
    ].map(|(k,b)| ((k, KeyModifiers::NONE), b));
    HashMap::from(keybinds_guidelinle)
}

#[derive(Debug)]
pub struct LiveTerminalInputHandler {
    _thread_handle: JoinHandle<()>,
    running_thread_flag: Arc<AtomicBool>,
}

impl Drop for LiveTerminalInputHandler {
    fn drop(&mut self) {
        self.running_thread_flag.store(false, Ordering::Release);
    }
}

impl LiveTerminalInputHandler {
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
        LiveTerminalInputHandler {
            _thread_handle: spawn(
                running_thread_flag.clone(),
                input_sender.clone(),
                keybinds.clone(),
            ),
            running_thread_flag,
        }
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
                        let _ = input_sender.send(InputSignal::StoreSavepoint);
                    }

                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('e'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press | KeyEventKind::Repeat,
                        ..
                    })) => {
                        let _ = input_sender.send(InputSignal::StoreSeed);
                    }

                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('b'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press | KeyEventKind::Repeat,
                        ..
                    })) => {
                        let _ = input_sender.send(InputSignal::Blindfold);
                    }

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

                    Ok(Event::Key(KeyEvent {
                        code,
                        modifiers,
                        kind: KeyEventKind::Press | KeyEventKind::Repeat,
                        ..
                    })) => {
                        if let Some(&button) = keybinds.get(&(code, modifiers)) {
                            // Binding found: send button press.
                            let now = Instant::now();
                            let _ = input_sender.send(InputSignal::ButtonInput(ButtonChange::Press(button), now));
                            let _ = input_sender.send(InputSignal::ButtonInput(ButtonChange::Release(button), now));
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
                        let _ = input_sender.send(InputSignal::StoreSavepoint);
                    }

                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('e'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press | KeyEventKind::Repeat,
                        ..
                    })) => {
                        let _ = input_sender.send(InputSignal::StoreSeed);
                    }

                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char('b'),
                        modifiers,
                        kind: KeyEventKind::Press | KeyEventKind::Repeat,
                        ..
                    })) if modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::SHIFT)) => {
                        let _ = input_sender.send(InputSignal::Blindfold);
                    }

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

                    Ok(Event::Key(KeyEvent {
                        kind: KeyEventKind::Repeat,
                        ..
                    })) => {}

                    Ok(Event::Key(KeyEvent { code, modifiers, kind, .. })) => match keybinds.get(&(code, modifiers)) {
                        // No binding: ignore.
                        None => {}
                        // Binding found: send button un-/press.
                        Some(&button) => {
                            let wrap = if kind == KeyEventKind::Press { ButtonChange::Press } else { ButtonChange::Release };
                            // FIXME: This module could be refactored by handling all the `let _ = input_sender.send(..)` lines and automatically stopping the thread, possibly removing the need for a synchronized run_thread flag in the first place.
                            let _ = input_sender.send(InputSignal::ButtonInput(
                                wrap(button),
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
