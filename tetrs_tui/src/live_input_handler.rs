use std::{
    sync::mpsc::{SendError, Sender},
    thread::{self, JoinHandle},
    time::Instant,
};

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};

use tetrs_engine::Button;

use crate::keybinds_presets::{normalize, Keybinds};

pub enum LiveTermSignal {
    RecognizedButton(Button, KeyEventKind),
    RawEvent(Event),
}

pub fn spawn(
    input_sender: Sender<(LiveTermSignal, Instant)>,
    keybinds: Keybinds,
) -> JoinHandle<()> {
    thread::spawn(move || {
        'detect_events: loop {
            // Read event.
            match event::read() {
                Ok(event) => {
                    let timestamp = Instant::now();

                    let mut stop_thread = false;

                    let signal = match event {
                        Event::Key(KeyEvent {
                            code,
                            modifiers,
                            kind,
                            ..
                        }) => {
                            let is_press_or_repeat = matches!(
                                kind,
                                event::KeyEventKind::Press | event::KeyEventKind::Repeat
                            );
                            // FIXME: What about forfeiting a game with [Ctrl+D]?
                            let escape = matches!(code, event::KeyCode::Esc);
                            let ctrl_c = matches!(code, event::KeyCode::Char('c'))
                                && matches!(modifiers, event::KeyModifiers::CONTROL);

                            if is_press_or_repeat && (escape || ctrl_c) {
                                stop_thread = true;
                            }

                            match keybinds.get(&normalize((code, modifiers))) {
                                // No binding: Just send directly transmit whatever the event was.
                                None => LiveTermSignal::RawEvent(event),

                                // Binding found: send button un-/press.
                                Some(&button) => LiveTermSignal::RecognizedButton(button, kind),
                            }
                        }

                        // Not a key event, just send directly.
                        _ => LiveTermSignal::RawEvent(event),
                    };

                    // Send signal.
                    match input_sender.send((signal, timestamp)) {
                        Ok(()) => {}
                        Err(SendError(_event_which_failed_to_transmit)) => {
                            break 'detect_events;
                        }
                    }

                    if stop_thread {
                        break 'detect_events;
                    }
                }

                // FIXME: Handle io::Error? If not, why not?
                Err(_e) => {}
            }
        }
    })
}
