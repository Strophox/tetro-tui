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

pub use crossterm::event::KeyCode as CT_Keycode;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};

use tetrs_lib::Button;

pub type ButtonSignal = Option<(Instant, Button, bool)>;

#[derive(Debug)]
pub struct CrosstermHandler {
    handles: Option<(JoinHandle<()>, Arc<AtomicBool>)>,
}

impl Drop for CrosstermHandler {
    fn drop(&mut self) {
        if let Some((_handle, running_flag)) = self.handles.take() {
            running_flag.store(false, Ordering::Release);
        }
    }
}

impl CrosstermHandler {
    pub fn new(
        sender: &Sender<ButtonSignal>,
        keybinds: &HashMap<CT_Keycode, Button>,
        kitty_enabled: bool,
    ) -> Self {
        let spawn = if kitty_enabled {
            Self::spawn_kitty
        } else {
            Self::spawn_standard
        };
        let flag = Arc::new(AtomicBool::new(true));
        let handle = spawn(sender.clone(), flag.clone(), keybinds.clone());
        CrosstermHandler {
            handles: Some((handle, flag)),
        }
    }

    fn spawn_standard(
        sender: Sender<ButtonSignal>,
        flag: Arc<AtomicBool>,
        keybinds: HashMap<CT_Keycode, Button>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                // Maybe stop thread.
                let running = flag.load(Ordering::Acquire);
                if !running {
                    break;
                }
                let event = match event::read() {
                    Ok(event) => event,
                    // Spurious io::Error: ignore.
                    Err(_) => continue,
                };
                let instant = Instant::now();
                let button_signals = match event {
                    // Escape pressed: send interrupt.
                    Event::Key(KeyEvent {
                        code: CT_Keycode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    }) => vec![None],
                    // Candidate key pressed.
                    Event::Key(KeyEvent {
                        code: key,
                        kind: KeyEventKind::Press,
                        ..
                    }) => match keybinds.get(&key) {
                        // Binding found: send button press.
                        Some(&button) => vec![
                            Some((instant, button, true)),
                            Some((instant, button, false)),
                        ],
                        // No binding: ignore.
                        None => continue,
                    },
                    // Don't care about other events: ignore.
                    _ => continue,
                };
                for button_signal in button_signals {
                    // crossterm::QueueableCommand::queue(&mut std::io::stderr(), crossterm::style::Print(format!("ct-send: {button_signal:?}."))).unwrap();
                    // crossterm::QueueableCommand::queue(&mut std::io::stderr(), crossterm::cursor::MoveToNextLine(1)).unwrap();
                    let _ = sender.send(button_signal);
                }
            }
        })
    }

    fn spawn_kitty(
        sender: Sender<ButtonSignal>,
        flag: Arc<AtomicBool>,
        keybinds: HashMap<CT_Keycode, Button>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                // Maybe stop thread.
                let running = flag.load(Ordering::Acquire);
                if !running {
                    break;
                }
                // Receive any Crossterm event.
                // TODO: Even after game has ended this will consume one more input before seeing the flag, e.g. wasting one input of "CTRL+C"!
                let (instant, event) = match event::read() {
                    // Spurious io::Error: ignore.
                    Err(_) => continue,
                    Ok(event) => (Instant::now(), event),
                };
                // Extract possibly relevant game button signal from event.
                let button_signal = match event {
                    // Escape pressed: send pause/interrupt.
                    Event::Key(KeyEvent {
                        code: CT_Keycode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    }) => None,
                    // TTY simulated press repeat: ignore.
                    Event::Key(KeyEvent {
                        kind: KeyEventKind::Repeat,
                        ..
                    }) => continue,
                    // Candidate key actually changed.
                    Event::Key(KeyEvent { code, kind, .. }) => match keybinds.get(&code) {
                        // No binding: ignore.
                        None => continue,
                        // Binding found: send button un-/press.
                        Some(&button) => Some((instant, button, kind == KeyEventKind::Press)),
                    },
                    // Don't care about other events: ignore.
                    _ => continue,
                };
                let _ = sender.send(button_signal);
            }
        })
    }
}

/* NOTE: Archived code. Could be removed at some point.
use device_query::{CallbackGuard, DeviceEvents};
pub use device_query::keymap::Keycode as DQ_Keycode;


pub trait GameInputHandler {
    type KeycodeType;
}

impl GameInputHandler for CrosstermHandler {
    type KeycodeType = CT_Keycode;
}


struct DeviceQueryHandler<D, U> {
    _guard_key_down: CallbackGuard<D>,
    _guard_key_up: CallbackGuard<U>,
}

impl<D, U> GameInputHandler for DeviceQueryHandler<D, U> {
    type KeycodeType = DQ_Keycode;
}
#[allow(dead_code)]
pub fn new_input_handler_devicequery(
    sender: &Sender<ButtonSignal>,
    keybinds: &HashMap<DQ_Keycode, Button>,
) -> Box<dyn GameInputHandler<KeycodeType = DQ_Keycode>> {
    let sender1 = sender.clone();
    let sender2 = sender.clone();
    let keybinds1 = std::sync::Arc::new(keybinds.clone());
    let keybinds2 = keybinds1.clone();
    // Initialize callbacks which send `Button` inputs.
    let device_state = device_query::DeviceState::new();
    let _guard_key_down = device_state.on_key_down(move |key| {
        let instant = Instant::now();
        let button_signal = match key {
            // Escape pressed: send interrupt.
            DQ_Keycode::Escape => None,
            // Candidate key pressed.
            key => match keybinds1.get(key) {
                // Binding found: send button press.
                Some(&button) => Some((instant, button, true)),
                // No binding: ignore.
                None => return,
            },
        };
        let _ = sender1.send(button_signal);
    });
    let _guard_key_up = device_state.on_key_up(move |key| {
        let instant = Instant::now();
        let button_signal = match key {
            // Escape released: ignore.
            DQ_Keycode::Escape => return,
            // Candidate key pressed.
            key => match keybinds2.get(key) {
                // Binding found: send button release.
                Some(&button) => Some((instant, button, false)),
                // No binding: ignore.
                None => return,
            },
        };
        let _ = sender2.send(button_signal);
    });
    Box::new(DeviceQueryHandler {
        _guard_key_down,
        _guard_key_up,
    })
}
*/
