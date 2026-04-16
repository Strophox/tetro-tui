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
use falling_tetromino_engine::Button;

use crate::{
    fmt_helpers::fmt_button_keybinds,
    tui_menus::{title_bar, Menu, MenuUpdate},
    tui_settings::GameKeybinds,
    Application, Settings,
};

impl<T: Write> Application<T> {
    pub fn run_menu_adjust_keybinds(&mut self) -> io::Result<MenuUpdate> {
        let if_unmodifiable_clone_and_switch = |s: &mut Settings| {
            if let Some(cloned_slot_idx) = s
                .keybinds_slotmachine
                .clone_slot_if_unmodifiable(s.keybinds_picked)
            {
                s.keybinds_picked = cloned_slot_idx;
            }
        };

        let buttons_available = Button::VARIANTS;
        // +1 for available slot selection.
        let selection_len = 1 + buttons_available.len();
        // Go to actual keybind selection on menu entry.
        let mut selected = 1usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();
            let y_selection = (Self::H_MAIN / 5).saturating_sub(2);
            // Draw menu title.
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "@ Keybinds @").bold(),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", title_bar(&self.settings))))?;

            // Draw slot label.
            let slot_label = format!(
                "Slot {}/{}: '{}'{}",
                self.settings.keybinds_picked + 1,
                self.settings.keybinds_slotmachine.slots.len(),
                self.settings
                    .keybinds_slotmachine
                    .grab(self.settings.keybinds_picked)
                    .0,
                if self.settings.keybinds_slotmachine.slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.keybinds_picked
                            < self.settings.keybinds_slotmachine.unmodifiable_slots
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
                .queue(Print(format!("{:^w_main$}", title_bar(&self.settings))))?;

            // Draw keybinds selection.
            let button_names = buttons_available.iter().map(|&button| {
                format!(
                    "{button:?}: {}",
                    fmt_button_keybinds(button, self.settings.keybinds())
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
                // Exit program.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c' | 'C'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => break Ok(MenuUpdate::Push(Menu::Quit)),

                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q' | 'Q') | KeyCode::Backspace,
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                // Modify keybind.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e' | 'E'),
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
                        if self.temp_data.kitty_assumed {
                            let f = Self::GAME_KEYBOARD_ENHANCEMENT_FLAGS;
                            // NOTE: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm minds if we do this on Windows.
                            let _v = self.term.execute(event::PushKeyboardEnhancementFlags(f));
                        }
                        loop {
                            if let Event::Key(KeyEvent {
                                code,
                                modifiers,
                                kind: Press,
                                ..
                            }) = event::read()?
                            {
                                // Add key pressed unless it's [Esc] or [Ctrl+C].
                                if matches!(
                                    (code, modifiers),
                                    (KeyCode::Char('c' | 'C'), KeyModifiers::CONTROL)
                                ) {
                                    return Ok(MenuUpdate::Push(Menu::Quit));
                                } else if !matches!(code, KeyCode::Esc) {
                                    if_unmodifiable_clone_and_switch(&mut self.settings);
                                    self.settings.keybinds_mut().unstable_access().insert(
                                        GameKeybinds::normalize((code, modifiers)),
                                        current_button,
                                    );
                                }
                                break;
                            }
                        }
                        // Console epilogue: De-initialization.
                        if self.temp_data.kitty_assumed {
                            // NOTE: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm minds if we do this on Windows.
                            let _v = self.term.execute(event::PopKeyboardEnhancementFlags);
                        }
                    }
                }

                // Delete keybind, or entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d' | 'D'),
                    kind: Press,
                    ..
                }) => {
                    if selected == 0 {
                        // If a custom slot, then remove it (and return to the 'default' 0th slot).
                        if self.settings.keybinds_picked
                            >= self.settings.keybinds_slotmachine.unmodifiable_slots
                        {
                            self.settings
                                .keybinds_slotmachine
                                .slots
                                .remove(self.settings.keybinds_picked);
                            self.settings.keybinds_picked = 0;
                        }
                    } else {
                        // Trying to modify a default slot: create copy of slot to allow safely modifying that.
                        if let Some(cloned_slot_idx) = self
                            .settings
                            .keybinds_slotmachine
                            .clone_slot_if_unmodifiable(self.settings.keybinds_picked)
                        {
                            self.settings.keybinds_picked = cloned_slot_idx;
                        }
                        // Remove all keys bound to the selected action button.
                        let button_selected = buttons_available[selected - 1];
                        self.settings
                            .keybinds_mut()
                            .unstable_access()
                            .retain(|_code, button| *button != button_selected);
                    }
                }

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k' | 'K'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += selection_len - 1;
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j' | 'J'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    selected += 1;
                }

                // Reload from savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('l' | 'L'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.loadfile_result = self.load_from_savefile();
                }

                // Cycle slot to right.
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 0 {
                        self.settings.keybinds_picked += 1;
                        self.settings.keybinds_picked %=
                            self.settings.keybinds_slotmachine.slots.len();
                    }
                }

                // Cycle slot to right.
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == 0 {
                        self.settings.keybinds_picked +=
                            self.settings.keybinds_slotmachine.slots.len() - 1;
                        self.settings.keybinds_picked %=
                            self.settings.keybinds_slotmachine.slots.len();
                    }
                }

                // Other IO event: no action.
                _ => {}
            }
            selected %= selection_len;
        }
    }
}
