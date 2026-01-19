use std::io::{self, Write};

use crossterm::{
    cursor::{self, MoveTo},
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
    ExecutableCommand, QueueableCommand,
};
use tetrs_engine::Button;

use crate::{
    application::{Application, Menu, MenuUpdate, Settings},
    utils::fmt_keybinds,
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_adjust_keybinds(&mut self) -> io::Result<MenuUpdate> {
        // "Trying to modify a default slot: create copy of slot to allow safely modifying that."
        let if_slot_is_default_then_copy_and_switch = |settings: &mut Settings| {
            if settings.keybinds_slot_active < settings.keybinds_slots_that_should_not_be_changed {
                let mut n = 1;
                let new_custom_slot_name = loop {
                    let name = format!("custom_{n}");
                    if settings.keybinds_slots.iter().any(|s| s.0 == name) {
                        n += 1;
                    } else {
                        break name;
                    }
                };
                let new_slot = (new_custom_slot_name, settings.keybinds().clone());
                settings.keybinds_slots.push(new_slot);
                settings.keybinds_slot_active = settings.keybinds_slots.len() - 1;
            }
        };
        let buttons_available = Button::VARIANTS;
        // +1 for available slot selection.
        let selection_len = 1 + buttons_available.len();
        // Go to actual keybind selection on menu entry.
        let mut selected = 1usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            // Draw menu title.
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "@ Keybinds @")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw slot label.
            let slot_label = format!(
                "Slot ({}/{}): \"{}\"{}",
                self.settings.keybinds_slot_active + 1,
                self.settings.keybinds_slots.len(),
                self.settings.keybinds_slots[self.settings.keybinds_slot_active].0,
                if self.settings.keybinds_slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.keybinds_slot_active
                            < self.settings.keybinds_slots_that_should_not_be_changed
                        {
                            ""
                        } else {
                            "Del|"
                        }
                    )
                }
            );
            self.term
                .queue(MoveTo(x_main, y_main + y_selection + 3))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == 0 {
                        format!(">> {slot_label} <<")
                    } else {
                        slot_label
                    }
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 4))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw keybinds selection.
            let button_names = buttons_available.iter().map(|&button| {
                format!(
                    "{button:?}: {}",
                    fmt_keybinds(button, self.settings.keybinds())
                )
            });
            for (i, name) in button_names.enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 6 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        // +1 because the first button is Slot selection.
                        if i + 1 == selected {
                            format!(">> {name} <<")
                        } else {
                            name
                        }
                    )))?;
            }

            // Draw footer legend.
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 6 + u16::try_from(buttons_available.len()).unwrap() + 1,
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        "(Controls: [Enter]=add [Esc]=cancel [Del]=clear)",
                    )
                    .italic(),
                ))?;
            self.term.flush()?;

            // Wait for new input.
            match event::read()? {
                // Abort program.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => {
                    break Ok(MenuUpdate::Push(Menu::Quit(
                        "exited with ctrl-c".to_owned(),
                    )))
                }

                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                // Modify keybind.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e'),
                    kind: Press,
                    ..
                }) => {
                    // `> 0` because 0 is slot selection.
                    if selected > 0 {
                        let current_button = buttons_available[selected - 1];
                        self.term
                            .execute(MoveTo(
                                x_main,
                                y_main
                                    + y_selection
                                    + 4
                                    + u16::try_from(selection_len).unwrap()
                                    + 2,
                            ))?
                            .execute(PrintStyledContent(
                                format!(
                                    "{:^w_main$}",
                                    format!("Press a key for {current_button:?}..."),
                                )
                                .italic(),
                            ))?
                            .execute(cursor::MoveToNextLine(1))?
                            .execute(Clear(ClearType::CurrentLine))?;
                        // Wait until appropriate keypress detected.
                        if self.runtime_data.kitty_assumed {
                            // FIXME: Kinda iffy. Do we need all flags? What undesirable effects might there be?
                            let _ = self.term.execute(event::PushKeyboardEnhancementFlags(
                                event::KeyboardEnhancementFlags::all(),
                                // event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
                            ));
                        }
                        loop {
                            if let Event::Key(KeyEvent {
                                code, kind: Press, ..
                            }) = event::read()?
                            {
                                // Add key pressed unless it's `Esc`.
                                if code != KeyCode::Esc {
                                    if_slot_is_default_then_copy_and_switch(&mut self.settings);
                                    self.settings.keybinds_mut().insert(code, current_button);
                                }
                                break;
                            }
                        }
                        // Console epilogue: De-initialization.
                        if self.runtime_data.kitty_assumed {
                            let _ = self.term.execute(event::PopKeyboardEnhancementFlags);
                        }
                    }
                }

                // Delete keybind, or entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press,
                    ..
                }) => {
                    if selected == 0 {
                        // If a custom slot, then remove it (and return to the 'default' 0th slot).
                        if self.settings.keybinds_slot_active
                            >= self.settings.keybinds_slots_that_should_not_be_changed
                        {
                            self.settings
                                .keybinds_slots
                                .remove(self.settings.keybinds_slot_active);
                            self.settings.keybinds_slot_active = 0;
                        }
                    } else {
                        // Trying to modify a default slot: create copy of slot to allow safely modifying that.
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        // Remove all keys bound to the selected action button.
                        let button_selected = buttons_available[selected - 1];
                        self.settings
                            .keybinds_mut()
                            .retain(|_code, button| *button != button_selected);
                    }
                }

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += selection_len - 1;
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += 1;
                }

                // Cycle slot to right.
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 0 {
                        self.settings.keybinds_slot_active += 1;
                        self.settings.keybinds_slot_active %= self.settings.keybinds_slots.len();
                    }
                }

                // Cycle slot to right.
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 0 {
                        self.settings.keybinds_slot_active +=
                            self.settings.keybinds_slots.len() - 1;
                        self.settings.keybinds_slot_active %= self.settings.keybinds_slots.len();
                    }
                }

                // Other IO event: no action.
                _ => {}
            }
            selected %= selection_len;
        }
    }
}
