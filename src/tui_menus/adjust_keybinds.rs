use std::io::{self, Write};

use crate::core_game_engine::Button;
use crossterm::{
    ExecutableCommand, QueueableCommand,
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{Color, PrintStyledContent, Stylize},
    terminal::{self, Clear, ClearType},
};

use crate::{
    Application, Settings,
    fmt_helpers::{fmt_button_keybinds, fmt_key_with_keymods},
    settings::GameKeybinds,
    tui_menus::{Menu, MenuUpdate, heading_line},
};

impl<W: Write> Application<W> {
    pub fn run_menu_adjust_keybinds(&mut self) -> io::Result<MenuUpdate> {
        let if_unmodifiable_clone_and_switch = |s: &mut Settings| {
            if let Some(cloned_slot_idx) = s
                .keybinds_slotmachine
                .clone_slot_if_unmodifiable(s.keybinds_selected)
            {
                s.keybinds_selected = cloned_slot_idx;
            }
        };

        let buttons_available = Button::VARIANTS;
        // +1 for available slot selection.
        let selection_len = 1 + buttons_available.len();
        // Go to actual keybind selection on menu entry.
        let mut selected = 1usize;
        loop {
            if self.settings.tui_coloring().bg_tui == Color::Reset {
                self.term.queue(Clear(ClearType::All))?;
            } else {
                self.term.queue(MoveTo(0, 0))?.queue(PrintStyledContent({
                    let (w, h) = terminal::size()?;
                    " ".repeat(w as usize * h as usize)
                        .on(self.settings.tui_coloring().bg_tui)
                }))?;
            }
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();
            let y_selection = (Self::H_MAIN / 5).saturating_sub(2);
            // Draw menu title.
            self.term
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "@ Game Keybinds @")
                        .bold()
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", heading_line(&self.settings))
                        .with(self.settings.tui_coloring().fg_accent)
                        .on(self.settings.tui_coloring().bg_tui),
                ))?;

            // Draw slot label.
            let slot_label = format!(
                "Slot {}/{}: '{}'{}",
                self.settings.keybinds_selected + 1,
                self.settings.keybinds_slotmachine.slots.len(),
                self.settings
                    .keybinds_slotmachine
                    .grab(self.settings.keybinds_selected)
                    .0,
                if self.settings.keybinds_slotmachine.slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←/{}→]",
                        if self.settings.keybinds_selected
                            < self.settings.keybinds_slotmachine.unmodifiable_slots
                        {
                            ""
                        } else {
                            "Del/"
                        }
                    )
                }
            );
            self.term
                .queue(MoveTo(x_main, y_main + y_selection + 3))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        if selected == 0 {
                            format!(
                                "{} {slot_label} {}",
                                self.settings.tui_symbols().menu_pointers[0],
                                self.settings.tui_symbols().menu_pointers[1]
                            )
                        } else {
                            slot_label
                        }
                    )
                    .with(self.settings.tui_coloring().fg_tui)
                    .on(self.settings.tui_coloring().bg_tui),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 4))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", heading_line(&self.settings))
                        .with(self.settings.tui_coloring().fg_accent)
                        .on(self.settings.tui_coloring().bg_tui),
                ))?;

            // Draw keybinds selection.
            let button_names = buttons_available.iter().map(|&button| {
                format!(
                    "{button:?}: {}",
                    fmt_button_keybinds(button, self.settings.keybinds(), ",")
                )
            });
            for (i, name) in button_names.enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 6 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            // +1 because the first button is Slot selection.
                            if i + 1 == selected {
                                format!(
                                    "{} {name} {}",
                                    self.settings.tui_symbols().menu_pointers[0],
                                    self.settings.tui_symbols().menu_pointers[1]
                                )
                            } else {
                                name
                            }
                        )
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui),
                    ))?;
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
                        "[Enter]=add [Esc]=cancel during add [Del]=clear",
                    )
                    .italic()
                    .with(self.settings.tui_coloring().fg_tui)
                    .on(self.settings.tui_coloring().bg_tui),
                ))?;
            let dangerous_keybinds: Vec<_> = self
                .settings
                .keybinds()
                .iter()
                .filter_map(|((kc, km), _b)| {
                    matches!(kc, KeyCode::Modifier(_)).then_some(fmt_key_with_keymods((*kc, *km)))
                })
                .collect();
            if !dangerous_keybinds.is_empty() && !self.temp_data.kitty_detected {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 6
                            + u16::try_from(buttons_available.len()).unwrap()
                            + 2,
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            "*Enhanced-key-events seem unsupported by terminal,",
                        )
                        .italic()
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui),
                    ))?
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 6
                            + u16::try_from(buttons_available.len()).unwrap()
                            + 3,
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            format!(
                                "keybinds unlikely to work: {}",
                                dangerous_keybinds.join(",")
                            ),
                        )
                        .italic()
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui),
                    ))?;
            }

            self.term.flush()?;

            // Wait for new input.
            match event::read()? {
                // Quit program.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c' | 'C'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => break Ok(MenuUpdate::Push(Menu::Quit)),

                // Keybinds help menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('?'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    let client_menu_name = "Game Keybinds menu";
                    let legend = vec![
                        (
                            "Normal keybinds".to_owned(),
                            [
                                ("Enter e", "Add new keybind to selected action"),
                                (
                                    "Escape Backspace q",
                                    "Exit menu or cancel when adding new keybind",
                                ),
                                (
                                    "Delete d",
                                    "Delete slot or reset keybinds for selected action",
                                ),
                                ("↓/↑ j/k", "Navigate down/up"),
                                ("←/→ h/l", "Change slot"),
                                ("?", "Open Keybinds overview"),
                            ]
                            .into_iter()
                            .map(|(lhs, rhs)| (lhs.to_owned(), rhs.to_owned()))
                            .collect(),
                        ),
                        (
                            "Special keybinds".to_owned(),
                            [
                                ("Ctrl+C", "Quit program (respects save preferences)"),
                                (
                                    "Ctrl+Alt+S",
                                    "Perform savefile store (respects save preferences)",
                                ),
                                (
                                    "Ctrl+Alt+L",
                                    "Reload app from savefile (overwrites current data!)",
                                ),
                            ]
                            .into_iter()
                            .map(|(lhs, rhs)| (lhs.to_owned(), rhs.to_owned()))
                            .collect(),
                        ),
                    ];

                    break Ok(MenuUpdate::Push(Menu::KeybindsOverview {
                        client_menu_name,
                        legend,
                    }));
                }

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
                })
                    // `> 0` because 0 is slot selection.
                    if selected > 0 => {
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
                                .italic().with(self.settings.tui_coloring().fg_tui).on(self.settings.tui_coloring().bg_tui),
                            ))?;
                        // Wait until appropriate keypress detected.
                        if self.temp_data.kitty_assumed {
                            let f = Self::GAME_KEYBOARD_ENHANCEMENT_FLAGS;
                            // NOTE: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm minds if we do this on Windows.
                            let _r: Result<&mut W, io::Error> = self.term.execute(event::PushKeyboardEnhancementFlags(f));
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
                            let _r = self.term.execute(event::PopKeyboardEnhancementFlags);
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
                        if self.settings.keybinds_selected
                            >= self.settings.keybinds_slotmachine.unmodifiable_slots
                        {
                            self.settings
                                .keybinds_slotmachine
                                .slots
                                .remove(self.settings.keybinds_selected);
                            self.settings.keybinds_selected = 0;
                        }
                    } else {
                        // Trying to modify a default slot: create copy of slot to allow safely modifying that.
                        if let Some(cloned_slot_idx) = self
                            .settings
                            .keybinds_slotmachine
                            .clone_slot_if_unmodifiable(self.settings.keybinds_selected)
                        {
                            self.settings.keybinds_selected = cloned_slot_idx;
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
                    self.temp_data.load_savefile_result = self.savefile_read();
                }

                // Store to savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('s' | 'S'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.store_savefile_result = self.savefile_write();
                }

                // Cycle slot to right.
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: Press | Repeat,
                    ..
                })
                    if selected == 0 => {
                        self.settings.keybinds_selected += 1;
                        self.settings.keybinds_selected %=
                            self.settings.keybinds_slotmachine.slots.len();
                    }

                // Cycle slot to right.
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    kind: Press | Repeat,
                    ..
                })
                    if selected == 0 => {
                        self.settings.keybinds_selected +=
                            self.settings.keybinds_slotmachine.slots.len() - 1;
                        self.settings.keybinds_selected %=
                            self.settings.keybinds_slotmachine.slots.len();
                    }

                // Other IO event: no action.
                _ => {}
            }
            selected %= selection_len;
        }
    }
}
