use std::{
    io::{self, Write},
    num::NonZeroUsize,
};

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
    fmt_helpers::BoolAsOnOff,
    tui_menus::{Menu, MenuUpdate},
    Application, Settings,
};

impl<T: Write> Application<T> {
    pub fn run_menu_adjust_graphics(&mut self) -> io::Result<MenuUpdate> {
        todo!() /*let if_unmodifiable_clone_and_switch = |s: &mut Settings| {
                    if let Some(cloned_slot_idx) = s
                        .graphics_slotmachine
                        .clone_slot_if_unmodifiable(s.graphics_picked)
                    {
                        s.graphics_picked = cloned_slot_idx;
                    }
                };

                let d_fps = 5.0;

                let mut selected = 1usize;
                loop {
                    let w_main = Self::W_MAIN.into();
                    let (x_main, y_main) = Self::viewport_offset();
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
                        self.settings.graphics_picked + 1,
                        self.settings.graphics_slotmachine.slots.len(),
                        self.settings.graphics_slotmachine.slots[self.settings.graphics_picked].0,
                        if self.settings.graphics_slotmachine.slots.len() < 2 {
                            "".to_owned()
                        } else {
                            format!(
                                " [←|{}→] ",
                                if self.settings.graphics_picked
                                    < self.settings.graphics_slotmachine.unmodifiable_slots
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
                        format!(
                            "Color palette = '{}'",
                            self.settings.palette_slotmachine.slots
                                [self.settings.graphics().palette_picked]
                                .0
                        ),
                        format!(
                            "TUI style = '{}'",
                            self.settings.tui_style_slotmachine.slots
                                [self.settings.graphics().tui_style_picked]
                                .0
                        ),
                        format!(
                            "Mino textures = '{}'",
                            self.settings.mino_textures_slotmachine.slots
                                [self.settings.graphics().mino_textures_picked]
                                .0
                        ),
                        format!(
                            "Hard drop effect = '{}'",
                            self.settings.hard_drop_effect_slotmachine.slots
                                [self.settings.graphics().hard_drop_picked]
                                .0
                        ),
                        format!(
                            "Lock effect = '{}'",
                            self.settings.lock_effect_slotmachine.slots
                                [self.settings.graphics().lock_effect_picked]
                                .0
                        ),
                        format!(
                            "Line clear effect = '{}'",
                            self.settings.line_clear_effect_slotmachine.slots
                                [self.settings.graphics().line_clear_picked]
                                .0
                        ),
                        format!(
                            "Mini tet. style = '{}'",
                            self.settings.mini_tet_style_slotmachine.slots
                                [self.settings.graphics().mini_tet_picked]
                                .0
                        ),
                        format!(
                            "Small tet. style = '{}'",
                            self.settings.small_tet_style_slotmachine.slots
                                [self.settings.graphics().small_tet_picked]
                                .0
                        ),
                        format!(
                            "Normalsize previews = {}",
                            self.settings.graphics().normalsize_preview_limit.unwrap_or(NonZeroUsize::MAX).get()
                        ),
                        format!(
                            "Frames per second = {:.02}",
                            self.settings.graphics().fps.get()
                        ),
                        format!(
                            "Display: {}",
                            [
                                ("Shadow", self.settings.graphics().show_shadow),
                                ("Spawn", self.settings.graphics().show_spawn),
                                ("Grid", self.settings.graphics().show_grid),
                                ("Boardcolor", self.settings.graphics().boardpalette_picked == 0),
                                ("HUD", self.settings.graphics().show_main_hud),
                                ("Keybinds", self.settings.graphics().show_keybinds),
                                ("Buttons", self.settings.graphics().show_buttons),
                                ("FPS", self.settings.graphics().show_fps),
                            ].map(|(name, is_on)| format!("{}{name}", if is_on { '#' } else { '_' })).join(" ")
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
                                    .get(&tet.tile_id())
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
                                self.settings.graphics_picked += 1;
                                self.settings.graphics_picked %=
                                    self.settings.graphics_slotmachine.slots.len();
                            }
                            1 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().glyphset =
                                    match self.settings.graphics().glyphset {
                                        Glyphset::Elektronika60 => Glyphset::Ascii,
                                        Glyphset::Ascii => Glyphset::Unicode,
                                        Glyphset::Unicode => Glyphset::Elektronika60,
                                    };
                            }
                            2 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().palette_picked += 1;
                                self.settings.graphics_mut().palette_picked %=
                                    self.settings.palette_slotmachine.slots.len();
                                self.settings.graphics_mut().boardpalette_picked =
                                    self.settings.graphics_mut().palette_picked;
                            }
                            3 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().boardpalette_picked =
                                    if self.settings.graphics().boardpalette_picked == 0 {
                                        self.settings.graphics_mut().palette_picked
                                    } else {
                                        0
                                    };
                            }
                            4 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().effects ^= true;
                            }
                            5 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().shadow_piece ^= true;
                            }
                            6 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().button_state ^= true;
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
                                self.settings.graphics_picked +=
                                    self.settings.graphics_slotmachine.slots.len() - 1;
                                self.settings.graphics_picked %=
                                    self.settings.graphics_slotmachine.slots.len();
                            }
                            1 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().glyphset =
                                    match self.settings.graphics().glyphset {
                                        Glyphset::Elektronika60 => Glyphset::Unicode,
                                        Glyphset::Ascii => Glyphset::Elektronika60,
                                        Glyphset::Unicode => Glyphset::Ascii,
                                    };
                            }
                            2 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().palette_picked +=
                                    self.settings.palette_slotmachine.slots.len() - 1;
                                self.settings.graphics_mut().palette_picked %=
                                    self.settings.palette_slotmachine.slots.len();
                                self.settings.graphics_mut().boardpalette_picked =
                                    self.settings.graphics_mut().palette_picked;
                            }
                            3 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().boardpalette_picked =
                                    if self.settings.graphics().boardpalette_picked == 0 {
                                        self.settings.graphics_mut().palette_picked
                                    } else {
                                        0
                                    };
                            }
                            4 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().effects ^= true;
                            }
                            5 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().shadow_piece ^= true;
                            }
                            6 => {
                                if_unmodifiable_clone_and_switch(&mut self.settings);
                                self.settings.graphics_mut().button_state ^= true;
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
                                if self.settings.graphics_picked
                                    >= self.settings.graphics_slotmachine.unmodifiable_slots
                                {
                                    self.settings
                                        .graphics_slotmachine
                                        .slots
                                        .remove(self.settings.graphics_picked);
                                    self.settings.graphics_picked = 0;
                                }
                            }
                        }

                        // Other event: Just ignore.
                        _ => {}
                    }
                    selected %= selection_len;
                }*/
    }
}
