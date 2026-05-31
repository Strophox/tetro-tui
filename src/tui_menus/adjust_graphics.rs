use std::{
    io::{self, Write},
    num::NonZeroUsize,
};

use crossterm::{
    QueueableCommand,
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
    core_game_engine::{Tetromino, TileType},
    fmt_helpers::BoolAsOnOff,
    tui_menus::{Menu, MenuUpdate, heading_line},
};

impl<W: Write> Application<W> {
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
            self.term
                // .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "# Graphics Settings #")
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
                        " [←/{}→]",
                        if self.settings.graphics_selected
                            < self.settings.graphics_slotmachine.unmodifiable_slots
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

            let labels1 = [
                format!(
                    "Tile colors = {}",
                    self.settings
                        .tile_coloring_slotmachine
                        .grab(self.settings.graphics().tile_coloring_selected)
                        .0
                ),
                format!(
                    "Tile/Tetromino symbols = {} {}{}{}",
                    self.settings
                        .tile_symbols_slotmachine
                        .grab(self.settings.graphics().tile_symbols_selected)
                        .0,
                    self.settings
                        .tile_symbols()
                        .locked(TileType::Tet(Tetromino::S))
                        .0
                        .map(|ch| ch.to_string())
                        .join(""),
                    self.settings
                        .tile_symbols()
                        .player(Tetromino::S)
                        .0
                        .map(|ch| ch.to_string())
                        .join(""),
                    self.settings
                        .tile_symbols()
                        .shadow
                        .0
                        .map(|ch| ch.to_string())
                        .join(""),
                ),
                format!(
                    "Use dual-colored tiles = {}",
                    self.settings
                        .graphics()
                        .use_primary_col_as_tile_bg_secondary_as_fg
                        .on_off(),
                ),
                format!(
                    "UI colors = {}",
                    self.settings
                        .tui_coloring_slotmachine
                        .grab(self.settings.graphics().tui_coloring_selected)
                        .0
                ),
                format!(
                    "UI symbols = {}",
                    self.settings
                        .tui_symbols_slotmachine
                        .grab(self.settings.graphics().tui_symbols_selected)
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
                    "Small tet. symbols = {} {}",
                    self.settings
                        .small_tetromino_symbols_slotmachine
                        .grab(self.settings.graphics().small_tetromino_symbols_selected)
                        .0,
                    self.settings.small_tetromino_symbols().tets[Tetromino::S as usize]
                ),
                format!(
                    "Mini tet. symbols = {} {}",
                    self.settings
                        .mini_tetromino_symbols_slotmachine
                        .grab(self.settings.graphics().mini_tetromino_symbols_selected)
                        .0,
                    self.settings
                        .mini_tetromino_symbols()
                        .tets
                        .map(|ch| ch.to_string())
                        .join("")
                ),
                format!(
                    "Normalsized tet. previews = {}",
                    self.settings
                        .graphics()
                        .normalsize_preview_limit
                        .map(|x| x.to_string())
                        .unwrap_or("unlimited".to_owned())
                ),
                format!(
                    "Frames rendered per second = {:.1}",
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
                        (self.settings.graphics().uniform_locked_tiles).then_some("Unif-tiles"),
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
                ("Grid", self.settings.graphics().show_grid),
                ("Piece shadow", self.settings.graphics().show_shadow),
                (
                    "Upcoming spawn preview (if stack high)",
                    self.settings.graphics().show_spawn,
                ),
                (
                    "Uniform locked tiles",
                    self.settings.graphics().uniform_locked_tiles,
                ),
                ("Main HUD", self.settings.graphics().show_main_hud),
                (
                    "Include basic keybinds HUD",
                    self.settings.graphics().show_keybinds,
                ),
                (
                    "Show active/held buttons",
                    self.settings.graphics().show_buttons,
                ),
                (
                    "Lock delay visualizer",
                    self.settings.graphics().show_lockdelay,
                ),
                ("FPS counter", self.settings.graphics().show_fps),
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
                        .queue(PrintStyledContent(
                            format!(
                                "{:^w_main$}",
                                if 1 + i == selected {
                                    format!(
                                        "{} {label} {}",
                                        self.settings.tui_symbols().menu_pointers[0],
                                        self.settings.tui_symbols().menu_pointers[1]
                                    )
                                } else {
                                    label.clone()
                                }
                            )
                            .with(self.settings.tui_coloring().fg_tui)
                            .on(self.settings.tui_coloring().bg_tui),
                        ))?;
                }
            } else {
                self.term
                    .queue(MoveTo(x_main, y_main + y_selection + 6))?
                    .queue(PrintStyledContent(
                        format!("{:^w_main$}", "...")
                            .with(self.settings.tui_coloring().fg_tui)
                            .on(self.settings.tui_coloring().bg_tui),
                    ))?;
                for (i, label) in labels2.into_iter().enumerate() {
                    self.term
                        .queue(MoveTo(
                            x_main,
                            y_main + y_selection + 6 + 1 + u16::try_from(i).unwrap(),
                        ))?
                        .queue(PrintStyledContent(
                            format!(
                                "{:^w_main$}",
                                if 1 + labels1.len() + i == selected {
                                    format!(
                                        "{} {label} {}",
                                        self.settings.tui_symbols().menu_pointers[0],
                                        self.settings.tui_symbols().menu_pointers[1]
                                    )
                                } else {
                                    label
                                }
                            )
                            .with(self.settings.tui_coloring().fg_tui)
                            .on(self.settings.tui_coloring().bg_tui),
                        ))?;
                }
            }

            if in_section_1 {
                self.term.queue(MoveTo(
                    x_main + u16::try_from((w_main - (2 + 6 * (2 + 2))) / 2).unwrap(),
                    y_main
                        + y_selection
                        + 6
                        + u16::try_from(labels1.len() + labels1_add.len()).unwrap()
                        + 1,
                ))?;

                for tet in Tetromino::VARIANTS {
                    for ch in self.settings.tile_symbols().locked(tet.into()).0 {
                        let (primcol, secndcol) =
                            self.settings.tile_coloring().tile_col(tet.into(), 0);
                        let (fg, bg) = if self
                            .settings
                            .graphics()
                            .use_primary_col_as_tile_bg_secondary_as_fg
                        {
                            (secndcol, primcol)
                        } else {
                            (primcol, self.settings.tui_coloring().bg_tui)
                        };
                        self.term.queue(PrintStyledContent(ch.with(fg).on(bg)))?;
                    }
                    self.term.queue(PrintStyledContent(
                        "  ".on(self.settings.tui_coloring().bg_tui),
                    ))?;
                }
            } else {
                self.term.queue(MoveTo(
                    x_main
                        + u16::try_from(
                            (w_main - (2 + 1 + 4 + 1 + 3 + 1 + 3 + 1 + 3 + 1 + 3 + 1 + 3)) / 2,
                        )
                        .unwrap(),
                    y_main
                        + y_selection
                        + 3
                        + u16::try_from(labels1.len() + labels1_add.len()).unwrap()
                        + 1,
                ))?;

                for tet in Tetromino::VARIANTS {
                    self.term.queue(PrintStyledContent(
                        self.settings.small_tetromino_symbols().tets[tet as usize]
                            .clone()
                            .with(
                                self.settings
                                    .tile_coloring()
                                    .simplified_tile_col(tet.into(), 0),
                            )
                            .on(self.settings.tui_coloring().bg_tui),
                    ))?;
                    self.term.queue(PrintStyledContent(
                        ' '.on(self.settings.tui_coloring().bg_tui),
                    ))?;
                }
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
                    let client_menu_name = "Graphics Settings menu";
                    let legend = vec![
                        (
                            "Normal keybinds".to_owned(),
                            [
                                ("Escape Backspace q", "Exit menu"),
                                ("Delete d", "Delete slot"),
                                ("↓/↑ j/k", "Navigate down/up"),
                                ("←/→ h/l", "Change slot, adjust value"),
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
                        self.settings.graphics_mut().tile_coloring_selected += 1;
                        self.settings.graphics_mut().tile_coloring_selected %=
                            self.settings.tile_coloring_slotmachine.slots.len();
                    }
                    2 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().tile_symbols_selected += 1;
                        self.settings.graphics_mut().tile_symbols_selected %=
                            self.settings.tile_symbols_slotmachine.slots.len();
                    }
                    3 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings
                            .graphics_mut()
                            .use_primary_col_as_tile_bg_secondary_as_fg ^= true;
                    }
                    4 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().tui_coloring_selected += 1;
                        self.settings.graphics_mut().tui_coloring_selected %=
                            self.settings.tui_coloring_slotmachine.slots.len();
                    }
                    5 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().tui_symbols_selected += 1;
                        self.settings.graphics_mut().tui_symbols_selected %=
                            self.settings.tui_symbols_slotmachine.slots.len();
                    }
                    6 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().hard_drop_selected += 1;
                        self.settings.graphics_mut().hard_drop_selected %=
                            self.settings.hard_drop_effect_slotmachine.slots.len();
                    }
                    7 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().lock_effect_selected += 1;
                        self.settings.graphics_mut().lock_effect_selected %=
                            self.settings.lock_effect_slotmachine.slots.len();
                    }
                    8 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().line_clear_selected += 1;
                        self.settings.graphics_mut().line_clear_selected %=
                            self.settings.line_clear_effect_slotmachine.slots.len();
                    }
                    9 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings
                            .graphics_mut()
                            .small_tetromino_symbols_selected += 1;
                        self.settings
                            .graphics_mut()
                            .small_tetromino_symbols_selected %= self
                            .settings
                            .small_tetromino_symbols_slotmachine
                            .slots
                            .len();
                    }
                    10 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().mini_tetromino_symbols_selected += 1;
                        self.settings.graphics_mut().mini_tetromino_symbols_selected %=
                            self.settings.mini_tetromino_symbols_slotmachine.slots.len();
                    }
                    11 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().normalsize_preview_limit =
                            if let Some(limit) = self.settings.graphics().normalsize_preview_limit {
                                Some(limit.saturating_add(1))
                            } else {
                                Some(NonZeroUsize::MIN)
                            }
                    }
                    12 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().fps += d_fps;
                    }
                    13 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_grid ^= true;
                    }
                    14 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_shadow ^= true;
                    }
                    15 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_spawn ^= true;
                    }
                    16 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().uniform_locked_tiles ^= true;
                    }
                    17 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_main_hud ^= true;
                    }
                    18 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_keybinds ^= true;
                    }
                    19 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_buttons ^= true;
                    }
                    20 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_lockdelay ^= true;
                    }
                    21 => {
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
                        self.settings.graphics_mut().tile_coloring_selected +=
                            self.settings.tile_coloring_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().tile_coloring_selected %=
                            self.settings.tile_coloring_slotmachine.slots.len();
                    }
                    2 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().tile_symbols_selected +=
                            self.settings.tile_symbols_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().tile_symbols_selected %=
                            self.settings.tile_symbols_slotmachine.slots.len();
                    }
                    3 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings
                            .graphics_mut()
                            .use_primary_col_as_tile_bg_secondary_as_fg ^= true;
                    }
                    4 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().tui_coloring_selected +=
                            self.settings.tui_coloring_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().tui_coloring_selected %=
                            self.settings.tui_coloring_slotmachine.slots.len();
                    }
                    5 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().tui_symbols_selected +=
                            self.settings.tui_symbols_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().tui_symbols_selected %=
                            self.settings.tui_symbols_slotmachine.slots.len();
                    }
                    6 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().hard_drop_selected +=
                            self.settings.hard_drop_effect_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().hard_drop_selected %=
                            self.settings.hard_drop_effect_slotmachine.slots.len();
                    }
                    7 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().lock_effect_selected +=
                            self.settings.lock_effect_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().lock_effect_selected %=
                            self.settings.lock_effect_slotmachine.slots.len();
                    }
                    8 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().line_clear_selected +=
                            self.settings.line_clear_effect_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().line_clear_selected %=
                            self.settings.line_clear_effect_slotmachine.slots.len();
                    }
                    9 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings
                            .graphics_mut()
                            .small_tetromino_symbols_selected += self
                            .settings
                            .small_tetromino_symbols_slotmachine
                            .slots
                            .len()
                            - 1;
                        self.settings
                            .graphics_mut()
                            .small_tetromino_symbols_selected %= self
                            .settings
                            .small_tetromino_symbols_slotmachine
                            .slots
                            .len();
                    }
                    10 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().mini_tetromino_symbols_selected +=
                            self.settings.mini_tetromino_symbols_slotmachine.slots.len() - 1;
                        self.settings.graphics_mut().mini_tetromino_symbols_selected %=
                            self.settings.mini_tetromino_symbols_slotmachine.slots.len();
                    }
                    11 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().normalsize_preview_limit = if let Some(limit) =
                            self.settings.graphics().normalsize_preview_limit
                        {
                            NonZeroUsize::try_from(limit.get() - 1).ok()
                        } else {
                            None
                        };
                    }
                    12 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        if self.settings.graphics().fps > d_fps {
                            self.settings.graphics_mut().fps =
                                self.settings.graphics().fps.saturating_sub(d_fps);
                        }
                    }
                    13 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_grid ^= true;
                    }
                    14 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_shadow ^= true;
                    }
                    15 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_spawn ^= true;
                    }
                    16 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().uniform_locked_tiles ^= true;
                    }
                    17 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_main_hud ^= true;
                    }
                    18 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_keybinds ^= true;
                    }
                    19 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_buttons ^= true;
                    }
                    20 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.graphics_mut().show_lockdelay ^= true;
                    }
                    21 => {
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
                }) if selected == 0
                    // If a custom slot, then remove it (and return to the 'default' 0th slot).
                    && self.settings.graphics_selected
                        >= self.settings.graphics_slotmachine.unmodifiable_slots =>
                {
                    self.settings
                        .graphics_slotmachine
                        .slots
                        .remove(self.settings.graphics_selected);
                    self.settings.graphics_selected = 0;
                }

                // Other event: Just ignore.
                _ => {}
            }
            selected %= selection_len;
        }
    }
}
