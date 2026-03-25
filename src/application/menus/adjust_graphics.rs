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
use falling_tetromino_engine::Tetromino;

use crate::{
    application::{
        menus::{Menu, MenuUpdate},
        Application, Glyphset, Settings,
    },
    fmt_helpers::{FmtBool, FmtTetromino},
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_adjust_graphics(&mut self) -> io::Result<MenuUpdate> {
        let if_unmodifiable_clone_and_switch = |s: &mut Settings| {
            if let Some(cloned_slot_idx) = s
                .graphics_slotmachine
                .clone_slot_if_unmodifiable(s.graphics_pick)
            {
                s.graphics_pick = cloned_slot_idx;
            }
        };

        let d_fps = 5.0;

        let mut selected = 1usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = (Self::H_MAIN / 5).saturating_sub(2);
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "# Graphics Settings #").bold(),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw slot label.
            let slot_label = format!(
                "Slot {}/{}: '{}'{}",
                self.settings.graphics_pick + 1,
                self.settings.graphics_slotmachine.slots.len(),
                self.settings.graphics_slotmachine.slots[self.settings.graphics_pick].0,
                if self.settings.graphics_slotmachine.slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.graphics_pick
                            < self.settings.graphics_slotmachine.unmodifiable
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
                format!("Glyphset = {:?}", self.settings.graphics().glyphset),
                format!(
                    "Color palette = '{}'",
                    self.settings.palette_slotmachine.slots[self.settings.graphics().palette_pick]
                        .0
                ),
                format!(
                    "Color locked tiles = {}",
                    (self.settings.graphics().lockpalette_pick != 0).fmt_on_off()
                ),
                format!(
                    "Show effects = {}",
                    self.settings.graphics().show_effects.fmt_on_off()
                ),
                format!(
                    "Show shadow piece = {}",
                    self.settings.graphics().show_shadow_piece.fmt_on_off()
                ),
                format!(
                    "Show button state = {}",
                    self.settings.graphics().show_button_state.fmt_on_off()
                ),
                format!("Max framerate = {}", self.settings.graphics().game_fps),
                format!(
                    "Show FPS = {}",
                    self.settings.graphics().show_fps.fmt_on_off()
                ),
                format!(
                    "(WIP) lineclear style = {}",
                    self.settings.graphics().lineclear_style
                ),
            ];

            // +1 For slot.
            let selection_len = labels.len() + 1;

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
                    if self.settings.graphics().glyphset == Glyphset::Unicode {
                        tet.linestr()
                    } else {
                        tet.linestr_ascii()
                    }
                    .with(
                        *self
                            .settings
                            .palette()
                            .get(&tet.tiletypeid())
                            .unwrap_or(&style::Color::Reset),
                    ),
                ))?;
                self.term.queue(Print(' '))?;
            }

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

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.graphics_pick += 1;
                        self.settings.graphics_pick %=
                            self.settings.graphics_slotmachine.slots.len();
                    }
                    1 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().glyphset =
                            match self.settings.graphics().glyphset {
                                Glyphset::Elektronika_60 => Glyphset::ASCII,
                                Glyphset::ASCII => Glyphset::Unicode,
                                Glyphset::Unicode => Glyphset::Elektronika_60,
                            };
                    }
                    2 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_pick += 1;
                        self.settings.graphics_mut().palette_pick %=
                            self.settings.palette_slotmachine.slots.len();
                        self.settings.graphics_mut().lockpalette_pick =
                            self.settings.graphics_mut().palette_pick;
                    }
                    3 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().lockpalette_pick =
                            if self.settings.graphics().lockpalette_pick == 0 {
                                self.settings.graphics_mut().palette_pick
                            } else {
                                0
                            };
                    }
                    4 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_effects ^= true;
                    }
                    5 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_shadow_piece ^= true;
                    }
                    6 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_button_state ^= true;
                    }
                    7 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().game_fps += d_fps;
                    }
                    8 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_fps ^= true;
                    }
                    9 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().lineclear_style += 1;
                        self.settings.graphics_mut().lineclear_style %= 2;
                    }
                    _ => {}
                },

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.graphics_pick +=
                            self.settings.graphics_slotmachine.slots.len() - 1;
                        self.settings.graphics_pick %=
                            self.settings.graphics_slotmachine.slots.len();
                    }
                    1 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().glyphset =
                            match self.settings.graphics().glyphset {
                                Glyphset::Elektronika_60 => Glyphset::Unicode,
                                Glyphset::ASCII => Glyphset::Elektronika_60,
                                Glyphset::Unicode => Glyphset::ASCII,
                            };
                    }
                    2 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_pick +=
                            self.settings.palette_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().palette_pick %=
                            self.settings.palette_slotmachine.slots.len();
                        self.settings.graphics_mut().lockpalette_pick =
                            self.settings.graphics_mut().palette_pick;
                    }
                    3 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().lockpalette_pick =
                            if self.settings.graphics().lockpalette_pick == 0 {
                                self.settings.graphics_mut().palette_pick
                            } else {
                                0
                            };
                    }
                    4 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_effects ^= true;
                    }
                    5 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_shadow_piece ^= true;
                    }
                    6 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_button_state ^= true;
                    }
                    7 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        if self.settings.graphics().game_fps > d_fps {
                            self.settings.graphics_mut().game_fps -= d_fps;
                        }
                    }
                    8 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_fps ^= true;
                    }
                    9 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().lineclear_style += 1;
                        self.settings.graphics_mut().lineclear_style %= 2;
                    }
                    _ => {}
                },

                // Reset graphics, or delete entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d' | 'D'),
                    kind: Press,
                    ..
                }) => {
                    if selected == 0 {
                        // If a custom slot, then remove it (and return to the 'default' 0th slot).
                        if self.settings.graphics_pick
                            >= self.settings.graphics_slotmachine.unmodifiable
                        {
                            self.settings
                                .graphics_slotmachine
                                .slots
                                .remove(self.settings.graphics_pick);
                            self.settings.graphics_pick = 0;
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
