use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{self, Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use tetrs_engine::Tetromino;

use crate::{
    application::{Application, Glyphset, Menu, MenuUpdate, Settings},
    fmt_helpers::fmt_tet_small,
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_adjust_graphics(&mut self) -> io::Result<MenuUpdate> {
        let if_slot_is_default_then_copy_and_switch = |settings: &mut Settings| {
            if settings.graphics_slot_active < settings.graphics_slots_that_should_not_be_changed {
                let mut n = 1;
                let new_custom_slot_name = loop {
                    let name = format!("Custom-{n}");
                    if settings.graphics_slots.iter().any(|s| s.0 == name) {
                        n += 1;
                    } else {
                        break name;
                    }
                };
                let new_slot = (new_custom_slot_name, *settings.graphics());
                settings.graphics_slots.push(new_slot);
                settings.graphics_slot_active = settings.graphics_slots.len() - 1;
            }
        };
        let selection_len = 9;
        let mut selected = 1usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "# Graphics Settings #")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw slot label.
            let slot_label = format!(
                "Slot ({}/{}): \"{}\"{}",
                self.settings.graphics_slot_active + 1,
                self.settings.graphics_slots.len(),
                self.settings.graphics_slots[self.settings.graphics_slot_active].0,
                if self.settings.graphics_slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.graphics_slot_active
                            < self.settings.graphics_slots_that_should_not_be_changed
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

            let labels = [
                format!("Glyphset: {:?}", self.settings.graphics().glyphset),
                format!(
                    "Color palette: '{}'",
                    self.settings.palette_slots[self.settings.graphics().palette_active].0
                ),
                format!(
                    "Color locked tiles: {}",
                    self.settings.graphics().palette_active_lockedtiles != 0
                ),
                format!("Show effects: {}", self.settings.graphics().show_effects),
                format!(
                    "Show ghost piece: {}",
                    self.settings.graphics().show_ghost_piece
                ),
                format!(
                    "Show button state: {}",
                    self.settings.graphics().show_button_state
                ),
                format!("Framerate: {}", self.settings.graphics().game_fps),
                format!("Show fps: {}", self.settings.graphics().show_fps),
            ];

            for (i, label) in labels.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 6 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if i + 1 == selected {
                            format!(">> {label} <<")
                        } else {
                            label
                        }
                    )))?;
            }

            self.term.queue(MoveTo(
                x_main + u16::try_from((w_main - 27) / 2).unwrap(),
                y_main + y_selection + 6 + u16::try_from(selection_len).unwrap() + 1,
            ))?;

            for tet in Tetromino::VARIANTS {
                self.term.queue(PrintStyledContent(
                    fmt_tet_small(tet).with(
                        *self
                            .settings
                            .palette()
                            .get(&tet.tiletypeid().get())
                            .unwrap_or(&style::Color::Reset),
                    ),
                ))?;
                self.term.queue(Print(' '))?;
            }

            self.term.flush()?;

            // Wait for new input.
            match event::read()? {
                // Abort program.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => break Ok(MenuUpdate::Push(Menu::Quit)),

                // Quit menu.
                Event::Key(KeyEvent {
                    code:
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace | KeyCode::Char('b'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

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

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.graphics_slot_active += 1;
                        self.settings.graphics_slot_active %= self.settings.graphics_slots.len();
                    }
                    1 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().glyphset =
                            match self.settings.graphics().glyphset {
                                Glyphset::Electronika60 => Glyphset::ASCII,
                                Glyphset::ASCII => Glyphset::Unicode,
                                Glyphset::Unicode => Glyphset::Electronika60,
                            };
                    }
                    2 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_active += 1;
                        self.settings.graphics_mut().palette_active %=
                            self.settings.palette_slots.len();
                        self.settings.graphics_mut().palette_active_lockedtiles =
                            self.settings.graphics_mut().palette_active;
                    }
                    3 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_active_lockedtiles =
                            if self.settings.graphics().palette_active_lockedtiles == 0 {
                                self.settings.graphics_mut().palette_active
                            } else {
                                0
                            };
                    }
                    4 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_effects ^= true;
                    }
                    5 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_ghost_piece ^= true;
                    }
                    6 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_button_state ^= true;
                    }
                    7 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().game_fps += 1.0;
                    }
                    8 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_fps ^= true;
                    }
                    _ => {}
                },

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.graphics_slot_active +=
                            self.settings.graphics_slots.len() - 1;
                        self.settings.graphics_slot_active %= self.settings.graphics_slots.len();
                    }
                    1 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().glyphset =
                            match self.settings.graphics().glyphset {
                                Glyphset::Electronika60 => Glyphset::Unicode,
                                Glyphset::ASCII => Glyphset::Electronika60,
                                Glyphset::Unicode => Glyphset::ASCII,
                            };
                    }
                    2 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_active +=
                            self.settings.palette_slots.len() - 1;
                        self.settings.graphics_mut().palette_active %=
                            self.settings.palette_slots.len();
                        self.settings.graphics_mut().palette_active_lockedtiles =
                            self.settings.graphics_mut().palette_active;
                    }
                    3 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_active_lockedtiles =
                            if self.settings.graphics().palette_active_lockedtiles == 0 {
                                self.settings.graphics_mut().palette_active
                            } else {
                                0
                            };
                    }
                    4 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_effects ^= true;
                    }
                    5 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_ghost_piece ^= true;
                    }
                    6 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_button_state ^= true;
                    }
                    7 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        if self.settings.graphics().game_fps > 1.0 {
                            self.settings.graphics_mut().game_fps -= 1.0;
                        }
                    }
                    8 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_fps ^= true;
                    }
                    _ => {}
                },

                // Reset graphics, or delete entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press,
                    ..
                }) => {
                    if selected == 0 {
                        // If a custom slot, then remove it (and return to the 'default' 0th slot).
                        if self.settings.graphics_slot_active
                            >= self.settings.graphics_slots_that_should_not_be_changed
                        {
                            self.settings
                                .graphics_slots
                                .remove(self.settings.graphics_slot_active);
                            self.settings.graphics_slot_active = 0;
                        }
                    }
                }

                // Other event: Just ignore.
                _ => {}
            }
            selected %= selection_len;
        }
    }
}
