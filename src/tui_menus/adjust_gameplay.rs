use std::{
    io::{self, Write},
    num::NonZeroU32,
    time::Duration,
};

use crate::core_game_engine::{
    BalanceOutGen, ExtNonNegF64, MiscPceRots, MiscTetGens, RecencyGen, RerollGen, StockGen,
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
use either::Either;

use crate::{
    Application, Settings,
    fmt_helpers::BoolAsOnOff,
    settings::GameplayPreferences,
    tui_menus::{Menu, MenuUpdate, heading_line},
};

impl<W: Write> Application<W> {
    pub fn run_menu_adjust_gameplay(&mut self) -> io::Result<MenuUpdate> {
        let if_unmodifiable_clone_and_switch = |s: &mut Settings| {
            if let Some(cloned_slot_idx) = s
                .gameplay_slotmachine
                .clone_slot_if_unmodifiable(s.gameplay_selected)
            {
                s.gameplay_selected = cloned_slot_idx;
            }
        };

        let d_das = Duration::from_millis(1);
        let d_arr = Duration::from_millis(1);
        let d_dsd = Duration::from_millis(5);
        let d_factor_sdf = ExtNonNegF64::new(0.5).unwrap();
        let d_upperbound_sdf = Duration::from_millis(5).into();
        let maxval_factor_sdf = ExtNonNegF64::from(40);
        let d_lcd = Duration::from_millis(5);
        let d_are = Duration::from_millis(5);
        let d_dtapfinesse = Duration::from_millis(5);

        let mut selected = 1usize;
        loop {
            if self.settings.tui_coloring().bg_tui == Color::Reset {
                self.term.queue(Clear(ClearType::All))?;
            } else {
                self.term.queue(MoveTo(0, 0))?.queue(PrintStyledContent({
                    let (w, h) = terminal::size()?;
                    " ".repeat((w * h) as usize)
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
                    format!(
                        "{:^w_main$}",
                        "= Gameplay Preferences (apply on New Game) ="
                    )
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
                self.settings.gameplay_selected + 1,
                self.settings.gameplay_slotmachine.slots.len(),
                self.settings
                    .gameplay_slotmachine
                    .grab(self.settings.gameplay_selected)
                    .0,
                if self.settings.gameplay_slotmachine.slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←/{}→]",
                        if self.settings.gameplay_selected
                            < self.settings.gameplay_slotmachine.unmodifiable_slots
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

            // Draw config selection.
            let warning_star = if self.temp_data.kitty_detected {
                ""
            } else {
                " *"
            };
            let labels = [
                format!("Piece rotation = {:?}", self.settings.gameplay().rotsys),
                format!(
                    "Piece randomization = {}",
                    match &self.settings.gameplay().tetgen {
                        MiscTetGens::Reroll(RerollGen {
                            tet_last_emitted: _,
                            aversion_to_last: 0,
                        }) => "Uniformly random".to_owned(),
                        MiscTetGens::Reroll(RerollGen {
                            tet_last_emitted: _,
                            aversion_to_last: 1,
                        }) => "Classic (Reroll 1x)".to_owned(),
                        MiscTetGens::Reroll(RerollGen {
                            tet_last_emitted: _,
                            aversion_to_last: n,
                        }) => format!("Reroll {n}x"),
                        MiscTetGens::Stock(StockGen {
                            tets_stocked: _,
                            restock_multiplicity,
                        }) => format!("{}-Bag", restock_multiplicity.get() * 7),
                        MiscTetGens::Recency(RecencyGen {
                            tets_last_emitted: _,
                            factor,
                            is_base_not_exp,
                        }) => format!(
                            "Recency ({})",
                            if *is_base_not_exp {
                                format!("{:.01}^#", factor.get())
                            } else {
                                format!("#^{:.01}", factor.get())
                            }
                        ),
                        MiscTetGens::BalanceOut(BalanceOutGen {
                            tets_relative_tallies: _,
                        }) => "Balance out".to_owned(),
                    }
                ),
                format!("Piece preview = {}", self.settings.gameplay().preview),
                format!(
                    "Delayed auto move (DAS) = {:?}{warning_star}",
                    self.settings.gameplay().das
                ),
                format!(
                    "Auto repeat rate (ARR) = {:?}{warning_star}",
                    self.settings.gameplay().arr
                ),
                format!(
                    "Delayed soft drop = {:?}{warning_star}",
                    self.settings.gameplay().dsd
                ),
                format!(
                    "Soft drop rate (SDF) = {}{warning_star}",
                    match self.settings.gameplay().sdr {
                        Either::Left(factor) => format!("{:.01}x gravity", factor.get()),
                        Either::Right(upperbound) =>
                            format!("raise gravity to {:.01} Hz", upperbound.as_hertz().get()),
                    }
                ),
                format!(
                    "Line clear duration (LCD) = {:?}",
                    self.settings.gameplay().lcd
                ),
                format!("Spawn delay (ARE) = {:?}", self.settings.gameplay().are),
                format!(
                    "Allow spawn manipulation (hold-IRS/IHS/IMS/ITS) = {}{warning_star}",
                    self.settings.gameplay().initsys.on_off()
                ),
                format!(
                    "Convert double-tap to teleport = {:?}",
                    self.settings.gameplay().dtapfinesse
                ),
            ];

            // For slot, +1
            let selection_len = labels.len() + 1;

            for (i, label) in labels.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 6 + u16::try_from(i).unwrap(),
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            if i + 1 == selected {
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
            if !self.temp_data.kitty_detected {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 6 + u16::try_from(selection_len).unwrap(),
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            "(*Unlikely to work; Enhanced-key-events seem unsupported by terminal)"
                        )
                        .italic()
                        .with(self.settings.tui_coloring().fg_tui)
                        .on(self.settings.tui_coloring().bg_tui),
                    ))?;
            }

            self.term.flush()?;
            // Wait for new input.
            match event::read()? {
                // Quit menu.
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
                    let client_menu_name = "Gameplay Preferences menu";
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
                                (
                                    "Alt+←/→ Alt+h/l",
                                    "Finely adjust value of Piece randomizer, toggle Soft drop speedup mechanic",
                                ),
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

                // Reset config, or delete entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d' | 'D'),
                    kind: Press,
                    ..
                }) if selected == 0
                    // If a custom slot, then remove it (and return to the 'default' 0th slot).
                    && self.settings.gameplay_selected
                        >= self.settings.gameplay_slotmachine.unmodifiable_slots =>
                {
                    self.settings
                        .gameplay_slotmachine
                        .slots
                        .remove(self.settings.gameplay_selected);
                    self.settings.gameplay_selected = 0;
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
                    self.temp_data.load_savefile_result = self.savefile_load();
                }

                // Store to savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('s' | 'S'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.store_savefile_result = self.savefile_store();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.gameplay_selected += 1;
                        self.settings.gameplay_selected %=
                            self.settings.gameplay_slotmachine.slots.len();
                    }
                    1 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().rotsys = match self.settings.gameplay().rotsys
                        {
                            MiscPceRots::Ocular => MiscPceRots::ClassicL,
                            MiscPceRots::ClassicL => MiscPceRots::ClassicR,
                            MiscPceRots::ClassicR => MiscPceRots::Super,
                            MiscPceRots::Super => MiscPceRots::Ocular,
                        };
                    }
                    2 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        if modifiers.contains(KeyModifiers::ALT) {
                            match &mut self.settings.gameplay_mut().tetgen {
                                MiscTetGens::Reroll(RerollGen {
                                    tet_last_emitted: _,
                                    aversion_to_last,
                                }) => {
                                    *aversion_to_last = aversion_to_last.saturating_add(1);
                                }
                                MiscTetGens::Stock(StockGen {
                                    tets_stocked: _,
                                    restock_multiplicity,
                                }) => {
                                    *restock_multiplicity = restock_multiplicity.saturating_add(1);
                                }
                                MiscTetGens::Recency(RecencyGen {
                                    tets_last_emitted: _,
                                    factor,
                                    is_base_not_exp,
                                }) => {
                                    if *is_base_not_exp {
                                        *factor += ExtNonNegF64::new(0.1).unwrap();
                                    } else {
                                        *is_base_not_exp ^= true;
                                    }
                                }
                                MiscTetGens::BalanceOut(BalanceOutGen {
                                    tets_relative_tallies: _,
                                }) => {}
                            };
                        } else {
                            self.settings.gameplay_mut().tetgen =
                                match self.settings.gameplay().tetgen {
                                    MiscTetGens::Reroll(RerollGen {
                                        aversion_to_last: 0,
                                        ..
                                    }) => MiscTetGens::classic(),
                                    MiscTetGens::Reroll(_) => MiscTetGens::bag(),
                                    MiscTetGens::Stock(_) => MiscTetGens::balance_out(),
                                    MiscTetGens::BalanceOut(_) => MiscTetGens::snappy(),
                                    MiscTetGens::Recency(_) => MiscTetGens::uniform(),
                                };
                        }
                    }
                    3 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().preview += 1;
                    }
                    4 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().das =
                            self.settings.gameplay().das.saturating_add(d_das);
                    }
                    5 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().arr =
                            self.settings.gameplay().arr.saturating_add(d_arr);
                    }
                    6 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().dsd = Some(
                            self.settings
                                .gameplay_mut()
                                .dsd
                                .unwrap_or_default()
                                .saturating_add(d_dsd),
                        );
                    }
                    7 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        if modifiers.contains(KeyModifiers::ALT) {
                            self.settings.gameplay_mut().sdr = match self.settings.gameplay().sdr {
                                Either::Left(_) => GameplayPreferences::default().sdr,
                                Either::Right(_) => GameplayPreferences::guideline().sdr,
                            }
                        } else {
                            match self.settings.gameplay_mut().sdr {
                                Either::Left(ref mut factor) => {
                                    *factor += d_factor_sdf;
                                    if *factor > maxval_factor_sdf {
                                        *factor = ExtNonNegF64::MAX;
                                    }
                                }
                                Either::Right(ref mut upperbound) => {
                                    *upperbound = upperbound.saturating_sub(d_upperbound_sdf);
                                }
                            }
                        }
                    }
                    8 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().lcd += d_lcd;
                    }
                    9 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().are =
                            self.settings.gameplay().are.saturating_add(d_are);
                    }
                    10 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().initsys ^= true;
                    }
                    11 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().dtapfinesse = Some(
                            self.settings
                                .gameplay_mut()
                                .dtapfinesse
                                .unwrap_or_default()
                                .saturating_add(d_dtapfinesse),
                        );
                    }
                    _ => {}
                },

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.gameplay_selected +=
                            self.settings.gameplay_slotmachine.slots.len() - 1;
                        self.settings.gameplay_selected %=
                            self.settings.gameplay_slotmachine.slots.len();
                    }
                    1 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().rotsys = match self.settings.gameplay().rotsys
                        {
                            MiscPceRots::Ocular => MiscPceRots::Super,
                            MiscPceRots::Super => MiscPceRots::ClassicR,
                            MiscPceRots::ClassicR => MiscPceRots::ClassicL,
                            MiscPceRots::ClassicL => MiscPceRots::Ocular,
                        };
                    }
                    2 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        if modifiers.contains(KeyModifiers::ALT) {
                            match &mut self.settings.gameplay_mut().tetgen {
                                MiscTetGens::Reroll(RerollGen {
                                    tet_last_emitted: _,
                                    aversion_to_last,
                                }) => {
                                    *aversion_to_last = aversion_to_last.saturating_sub(1);
                                }
                                MiscTetGens::Stock(StockGen {
                                    tets_stocked: _,
                                    restock_multiplicity,
                                }) => {
                                    *restock_multiplicity =
                                        NonZeroU32::new(restock_multiplicity.get() - 1)
                                            .unwrap_or(NonZeroU32::MIN);
                                }
                                MiscTetGens::Recency(RecencyGen {
                                    tets_last_emitted: _,
                                    factor,
                                    is_base_not_exp,
                                }) => {
                                    if *is_base_not_exp {
                                        *is_base_not_exp ^= true;
                                    } else {
                                        *factor =
                                            factor.saturating_sub(ExtNonNegF64::new(0.1).unwrap());
                                    }
                                }
                                MiscTetGens::BalanceOut(BalanceOutGen {
                                    tets_relative_tallies: _,
                                }) => {}
                            };
                        } else {
                            self.settings.gameplay_mut().tetgen =
                                match self.settings.gameplay().tetgen {
                                    MiscTetGens::Reroll(RerollGen {
                                        aversion_to_last: 0,
                                        ..
                                    }) => MiscTetGens::snappy(),
                                    MiscTetGens::Reroll(_) => MiscTetGens::uniform(),
                                    MiscTetGens::Stock(_) => MiscTetGens::classic(),
                                    MiscTetGens::BalanceOut(_) => MiscTetGens::bag(),
                                    MiscTetGens::Recency(_) => MiscTetGens::balance_out(),
                                };
                        }
                    }
                    3 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().preview =
                            self.settings.gameplay().preview.saturating_sub(1);
                    }
                    4 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().das =
                            self.settings.gameplay().das.saturating_sub(d_das);
                    }
                    5 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().arr =
                            self.settings.gameplay().arr.saturating_sub(d_arr);
                    }
                    6 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().dsd = self
                            .settings
                            .gameplay()
                            .dsd
                            .unwrap_or_default()
                            .checked_sub(d_dsd);
                    }
                    7 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        if modifiers.contains(KeyModifiers::ALT) {
                            self.settings.gameplay_mut().sdr = match self.settings.gameplay().sdr {
                                Either::Left(_) => GameplayPreferences::default().sdr,
                                Either::Right(_) => GameplayPreferences::guideline().sdr,
                            }
                        } else {
                            match self.settings.gameplay_mut().sdr {
                                Either::Left(ref mut factor) => {
                                    if *factor > maxval_factor_sdf {
                                        *factor = maxval_factor_sdf;
                                    } else {
                                        *factor = factor.saturating_sub(d_factor_sdf)
                                    }
                                }
                                Either::Right(ref mut upperbound) => {
                                    *upperbound += d_upperbound_sdf;
                                }
                            }
                        }
                    }
                    8 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().lcd =
                            self.settings.gameplay().lcd.saturating_sub(d_lcd);
                    }
                    9 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().are =
                            self.settings.gameplay().are.saturating_sub(d_are);
                    }
                    10 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().initsys ^= true;
                    }
                    11 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().dtapfinesse = self
                            .settings
                            .gameplay()
                            .dtapfinesse
                            .unwrap_or_default()
                            .checked_sub(d_dtapfinesse);
                    }
                    _ => {}
                },
                // Other event: don't care.
                _ => {}
            }
            selected %= selection_len;
        }
    }
}
