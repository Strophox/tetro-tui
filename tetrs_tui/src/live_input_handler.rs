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
            let event_read_result = event::read();

            match event_read_result {
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
                            let is_press = matches!(
                                kind,
                                event::KeyEventKind::Press | event::KeyEventKind::Repeat
                            );
                            let escape = matches!(code, event::KeyCode::Esc);
                            let ctrl_c = matches!(code, event::KeyCode::Char('c'))
                                && matches!(modifiers, event::KeyModifiers::CONTROL);
                            if is_press && (escape || ctrl_c) {
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

                    let send_signal_result = input_sender.send((signal, timestamp));

                    match send_signal_result {
                        Ok(()) => {}
                        Err(SendError(_event_which_failed_to_transmit)) => {
                            break 'detect_events;
                        }
                    }

                    if stop_thread {
                        break 'detect_events;
                    }
                }

                Err(_e) => {
                    // FIXME: Handle io::Error? If not, why not?
                }
            };
        }
    })
}
