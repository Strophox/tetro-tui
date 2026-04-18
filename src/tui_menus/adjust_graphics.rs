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
    tui_menus::{title_bar, Menu, MenuUpdate},
    Application, Settings,
};

impl<T: Write> Application<T> {
    pub fn run_menu_adjust_graphics(&mut self) -> io::Result<MenuUpdate> {
        let if_unmodifiable_clone_and_switch = |s: &mut Settings| {
            if let Some(cloned_slot_idx) = s
                .graphics_slotmachine
                .clone_slot_if_unmodifiable(s.graphics_selected)
            {
                s.graphics_selected = cloned_slot_idx;
            }
        };

        let d_fps = 5.0.try_into().unwrap();

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
                .queue(Print(format!("{:^w_main$}", title_bar(&self.settings))))?;

            // Draw slot label.
            let slot_label = format!(
                "Slot {}/{}: '{}'{}",
                self.settings.graphics_selected + 1,
                self.settings.graphics_slotmachine.slots.len(),
                self.settings
                    .graphics_slotmachine
                    .grab(self.settings.graphics_selected)
                    .0,
                if self.settings.graphics_slotmachine.slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.graphics_selected
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
                .queue(Print(format!("{:^w_main$}", title_bar(&self.settings))))?;

            let labels1 = [
                format!(
                    "Color palette = {}",
                    self.settings
                        .palette_slotmachine
                        .grab(self.settings.graphics().palette_selected)
                        .0
                ),
                format!(
                    "TUI style = {}",
                    self.settings
                        .tui_style_slotmachine
                        .grab(self.settings.graphics().tui_style_selected)
                        .0
                ),
                format!(
                    "Mino textures = {}",
                    self.settings
                        .mino_textures_slotmachine
                        .grab(self.settings.graphics().mino_textures_selected)
                        .0
                ),
                format!(
                    "Hard drop effect = {}",
                    self.settings
                        .hard_drop_effect_slotmachine
                        .grab(self.settings.graphics().hard_drop_selected)
                        .0
                ),
                format!(
                    "Lock effect = {}",
                    self.settings
                        .lock_effect_slotmachine
                        .grab(self.settings.graphics().lock_effect_selected)
                        .0
                ),
                format!(
                    "Line clear effect = {}",
                    self.settings
                        .line_clear_effect_slotmachine
                        .grab(self.settings.graphics().line_clear_selected)
                        .0
                ),
                format!(
                    "Mini tet. style = {}",
                    self.settings
                        .mini_tet_style_slotmachine
                        .grab(self.settings.graphics().mini_tet_selected)
                        .0
                ),
                format!(
                    "Small tet. style = {}",
                    self.settings
                        .small_tet_style_slotmachine
                        .grab(self.settings.graphics().small_tet_selected)
                        .0
                ),
                format!(
                    "Normalsize previews = {}",
                    self.settings
                        .graphics()
                        .normalsize_preview_limit
                        .map(|x| x.to_string())
                        .unwrap_or("unlimited".to_owned())
                ),
                format!(
                    "Frames per second = {:.1}",
                    self.settings.graphics().fps.get()
                ),
            ];

            let labels1_add = [
                format!(
                    "Show... {{{}}}",
                    [
                        self.settings.graphics().show_grid.then_some("Grid"),
                        self.settings.graphics().show_shadow.then_some("Shadow"),
                        self.settings.graphics().show_spawn.then_some("Spawn"),
                        (self.settings.graphics().boardpalette_selected != 0)
                            .then_some("Col'board"),
                        self.settings.graphics().show_main_hud.then_some("HUD"),
                        self.settings.graphics().show_keybinds.then_some("Keybinds"),
                        self.settings.graphics().show_buttons.then_some("Buttons"),
                        self.settings
                            .graphics()
                            .show_lockdelay
                            .then_some("Lockdelay"),
                        self.settings.graphics().show_fps.then_some("FPS"),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(",")
                ),
                // format!(
                //     "...{}]",
                //     [
                //         self.settings.graphics().show_main_hud.then_some("HUD"),
                //         self.settings.graphics().show_keybinds.then_some("Keybinds"),
                //         self.settings.graphics().show_buttons.then_some("Buttons"),
                //         self.settings.graphics().show_fps.then_some("FPS"),
                //     ].into_iter().filter_map(std::convert::identity).collect::<Vec<_>>().join(",")
                // ),
            ];

            let labels2 = [
                ("Show grid", self.settings.graphics().show_grid),
                ("Show piece shadow", self.settings.graphics().show_shadow),
                (
                    "Preview spawn when stack high",
                    self.settings.graphics().show_spawn,
                ),
                (
                    "Color board tiles",
                    self.settings.graphics().boardpalette_selected != 0,
                ),
                ("Show left HUD", self.settings.graphics().show_main_hud),
                (
                    "Show keybinds legend",
                    self.settings.graphics().show_keybinds,
                ),
                ("Show active buttons", self.settings.graphics().show_buttons),
                (
                    "Show lock delay countdown",
                    self.settings.graphics().show_lockdelay,
                ),
                ("Show FPS counter", self.settings.graphics().show_fps),
            ]
            .map(|(name, is_on)| format!("{name} = {}", is_on.on_off()));

            // +1 For slot.
            let selection_len = 1 + labels1.len() + labels2.len();

            let in_section_1 = selected < 1 + labels1.len();

            if selected < 1 + labels1.len() {
                for (i, label) in labels1.iter().chain(labels1_add.iter()).enumerate() {
                    self.term
                        .queue(MoveTo(
                            x_main,
                            y_main + y_selection + 6 + u16::try_from(i).unwrap(),
                        ))?
                        .queue(Print(format!(
                            "{:^w_main$}",
                            if 1 + i == selected {
                                format!(">> {label} <<")
                            } else {
                                label.clone()
                            }
                        )))?;
                }
            } else {
                self.term
                    .queue(MoveTo(x_main, y_main + y_selection + 6))?
                    .queue(Print(format!("{:^w_main$}", "...")))?;
                for (i, label) in labels2.into_iter().enumerate() {
                    self.term
                        .queue(MoveTo(
                            x_main,
                            y_main + y_selection + 6 + 1 + u16::try_from(i).unwrap(),
                        ))?
                        .queue(Print(format!(
                            "{:^w_main$}",
                            if 1 + labels1.len() + i == selected {
                                format!(">> {label} <<")
                            } else {
                                label
                            }
                        )))?;
                }
            }

            if in_section_1 {
                self.term.queue(MoveTo(
                    x_main + u16::try_from((w_main - 27) / 2).unwrap(),
                    y_main
                        + y_selection
                        + 6
                        + u16::try_from(labels1.len() + labels1_add.len()).unwrap()
                        + 1,
                ))?;

                for tet in Tetromino::VARIANTS {
                    self.term.queue(PrintStyledContent(
                        self.settings.small_tet_style().tets[tet as usize]
                            .clone()
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

                // Reload from savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('l' | 'L'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.loadfile_result = self.savefile_load();
                }

                // Store to savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('s' | 'S'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.storefile_result = self.savefile_store();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.graphics_selected += 1;
                        self.settings.graphics_selected %=
                            self.settings.graphics_slotmachine.slots.len();
                    }
                    1 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_selected += 1;
                        self.settings.graphics_mut().palette_selected %=
                            self.settings.palette_slotmachine.slots.len();
                        self.settings.graphics_mut().boardpalette_selected =
                            self.settings.graphics_mut().palette_selected;
                    }
                    2 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().tui_style_selected += 1;
                        self.settings.graphics_mut().tui_style_selected %=
                            self.settings.tui_style_slotmachine.slots.len();
                    }
                    3 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().mino_textures_selected += 1;
                        self.settings.graphics_mut().mino_textures_selected %=
                            self.settings.mino_textures_slotmachine.slots.len();
                    }
                    4 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().hard_drop_selected += 1;
                        self.settings.graphics_mut().hard_drop_selected %=
                            self.settings.hard_drop_effect_slotmachine.slots.len();
                    }
                    5 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().lock_effect_selected += 1;
                        self.settings.graphics_mut().lock_effect_selected %=
                            self.settings.lock_effect_slotmachine.slots.len();
                    }
                    6 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().line_clear_selected += 1;
                        self.settings.graphics_mut().line_clear_selected %=
                            self.settings.line_clear_effect_slotmachine.slots.len();
                    }
                    7 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().mini_tet_selected += 1;
                        self.settings.graphics_mut().mini_tet_selected %=
                            self.settings.mini_tet_style_slotmachine.slots.len();
                    }
                    8 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().small_tet_selected += 1;
                        self.settings.graphics_mut().small_tet_selected %=
                            self.settings.small_tet_style_slotmachine.slots.len();
                    }
                    9 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().normalsize_preview_limit =
                            if let Some(limit) = self.settings.graphics().normalsize_preview_limit {
                                Some(limit.saturating_add(1))
                            } else {
                                Some(NonZeroUsize::MIN)
                            }
                    }
                    10 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().fps += d_fps;
                    }
                    11 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_grid ^= true;
                    }
                    12 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_shadow ^= true;
                    }
                    13 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_spawn ^= true;
                    }
                    14 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().boardpalette_selected =
                            if self.settings.graphics().boardpalette_selected == 0 {
                                self.settings.graphics_mut().palette_selected
                            } else {
                                0
                            };
                    }
                    15 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_main_hud ^= true;
                    }
                    16 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_keybinds ^= true;
                    }
                    17 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_buttons ^= true;
                    }
                    18 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_lockdelay ^= true;
                    }
                    19 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_fps ^= true;
                    }
                    _ => {}
                },

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.graphics_selected +=
                            self.settings.graphics_slotmachine.slots.len() - 1;
                        self.settings.graphics_selected %=
                            self.settings.graphics_slotmachine.slots.len();
                    }
                    1 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().palette_selected +=
                            self.settings.palette_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().palette_selected %=
                            self.settings.palette_slotmachine.slots.len();
                        self.settings.graphics_mut().boardpalette_selected =
                            self.settings.graphics_mut().palette_selected;
                    }
                    2 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().tui_style_selected +=
                            self.settings.tui_style_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().tui_style_selected %=
                            self.settings.tui_style_slotmachine.slots.len();
                    }
                    3 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().mino_textures_selected +=
                            self.settings.mino_textures_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().mino_textures_selected %=
                            self.settings.mino_textures_slotmachine.slots.len();
                    }
                    4 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().hard_drop_selected +=
                            self.settings.hard_drop_effect_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().hard_drop_selected %=
                            self.settings.hard_drop_effect_slotmachine.slots.len();
                    }
                    5 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().lock_effect_selected +=
                            self.settings.lock_effect_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().lock_effect_selected %=
                            self.settings.lock_effect_slotmachine.slots.len();
                    }
                    6 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().line_clear_selected +=
                            self.settings.line_clear_effect_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().line_clear_selected %=
                            self.settings.line_clear_effect_slotmachine.slots.len();
                    }
                    7 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().mini_tet_selected +=
                            self.settings.mini_tet_style_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().mini_tet_selected %=
                            self.settings.mini_tet_style_slotmachine.slots.len();
                    }
                    8 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().small_tet_selected +=
                            self.settings.small_tet_style_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().small_tet_selected %=
                            self.settings.small_tet_style_slotmachine.slots.len();
                    }
                    9 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().normalsize_preview_limit = if let Some(limit) =
                            self.settings.graphics().normalsize_preview_limit
                        {
                            NonZeroUsize::try_from(limit.get() - 1).ok()
                        } else {
                            None
                        };
                    }
                    10 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        if self.settings.graphics().fps > d_fps {
                            self.settings.graphics_mut().fps =
                                self.settings.graphics().fps.saturating_sub(d_fps);
                        }
                    }
                    11 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_grid ^= true;
                    }
                    12 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_shadow ^= true;
                    }
                    13 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_spawn ^= true;
                    }
                    14 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().boardpalette_selected =
                            if self.settings.graphics().boardpalette_selected == 0 {
                                self.settings.graphics_mut().palette_selected
                            } else {
                                0
                            };
                    }
                    15 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_main_hud ^= true;
                    }
                    16 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_keybinds ^= true;
                    }
                    17 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_buttons ^= true;
                    }
                    18 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_lockdelay ^= true;
                    }
                    19 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_fps ^= true;
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
                        if self.settings.graphics_selected
                            >= self.settings.graphics_slotmachine.unmodifiable_slots
                        {
                            self.settings
                                .graphics_slotmachine
                                .slots
                                .remove(self.settings.graphics_selected);
                            self.settings.graphics_selected = 0;
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
