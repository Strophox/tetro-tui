use std::{
    io::{self, Write},
    num::NonZeroU32,
    time::Duration,
};

use crossterm::{
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use falling_tetromino_engine::{ExtNonNegF64, RotationSystem, TetrominoGenerator};

use crate::{
    application::{
        menus::{Menu, MenuUpdate},
        Application, Settings,
    },
    fmt_helpers::FmtBool,
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_adjust_gameplay(&mut self) -> io::Result<MenuUpdate> {
        let if_unmodifiable_clone_and_switch = |s: &mut Settings| {
            if let Some(cloned_slot_idx) = s
                .gameplay_slotmachine
                .clone_slot_if_unmodifiable(s.gameplay_pick)
            {
                s.gameplay_pick = cloned_slot_idx;
            }
        };

        let d_das = Duration::from_millis(1);
        let d_arr = Duration::from_millis(1);
        let d_sdf = ExtNonNegF64::new(0.25).unwrap();
        let d_lcd = Duration::from_millis(5);
        let d_are = Duration::from_millis(5);

        let d_tmf = Duration::from_millis(5);

        let mut selected = 1usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = (Self::H_MAIN / 5).saturating_sub(2);

            // Draw menu title.
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        "= Gameplay Configuration (apply on New Game) ="
                    )
                    .bold(),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw slot label.
            let slot_label = format!(
                "Slot {}/{}: '{}'{}",
                self.settings.gameplay_pick + 1,
                self.settings.gameplay_slotmachine.slots.len(),
                self.settings.gameplay_slotmachine.slots[self.settings.gameplay_pick].0,
                if self.settings.gameplay_slotmachine.slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.gameplay_pick
                            < self.settings.gameplay_slotmachine.unmodifiable
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

            // Draw config selection.
            let labels = [
                format!(
                    "Piece rotation system = {:?}",
                    self.settings.gameplay().rotsys
                ),
                format!(
                    "Piece randomization = {}",
                    match &self.settings.gameplay().randomizer {
                        TetrominoGenerator::Uniform => "Completely random".to_owned(),
                        TetrominoGenerator::Stock {
                            tets_stocked: _,
                            restock_multiplicity,
                        } => format!("{}-Bag", restock_multiplicity.get() * 7),
                        TetrominoGenerator::Recency {
                            tets_last_emitted: _,
                            factor,
                            is_base_not_exp,
                        } => format!(
                            "Recency ({})",
                            if *is_base_not_exp {
                                format!("{:.01}^", factor.get())
                            } else {
                                format!("^{:.01}", factor.get())
                            }
                        ),
                        TetrominoGenerator::BalanceOut {
                            tets_relative_counts: _,
                        } => "Balance out".to_owned(),
                    }
                ),
                format!("Piece preview count = {}", self.settings.gameplay().preview),
                format!(
                    "Delayed auto move (DAS) = {:?} *",
                    self.settings.gameplay().das
                ),
                format!(
                    "Auto move rate (ARR) = {:?} *",
                    self.settings.gameplay().arr
                ),
                format!(
                    "Soft drop speedup (SDF) = {}x *",
                    self.settings.gameplay().sdf.get()
                ),
                format!(
                    "Line clear duration (LCD) = {:?}",
                    self.settings.gameplay().lcd
                ),
                format!("Spawn delay (ARE) = {:?}", self.settings.gameplay().are),
                format!(
                    "Allow initial rotation/hold (IRS/IHS) = {} *",
                    self.settings.gameplay().initsys.fmt_on_off()
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
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if i + 1 == selected {
                            format!(">> {label} <<")
                        } else {
                            label
                        }
                    )))?;
            }
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 6 + u16::try_from(selection_len).unwrap(),
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        if self.temp_data.kitty_detected {
                            "(*Should apply, since terminal seems to support enhanced-key-events)"
                        } else {
                            "(*Unlikely to apply, enhanced-key-events seem unsupported by terminal)"
                        },
                    )
                    .italic(),
                ))?;

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
                }) => {
                    if selected == 0 {
                        // If a custom slot, then remove it (and return to the 'default' 0th slot).
                        if self.settings.gameplay_pick
                            >= self.settings.gameplay_slotmachine.unmodifiable
                        {
                            self.settings
                                .gameplay_slotmachine
                                .slots
                                .remove(self.settings.gameplay_pick);
                            self.settings.gameplay_pick = 0;
                        }
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
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.gameplay_pick += 1;
                        self.settings.gameplay_pick %=
                            self.settings.gameplay_slotmachine.slots.len();
                    }
                    1 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().rotsys = match self.settings.gameplay().rotsys
                        {
                            RotationSystem::Raw => RotationSystem::Ocular, // Set to Ocular.
                            RotationSystem::Ocular => RotationSystem::ClassicL,
                            RotationSystem::ClassicL => RotationSystem::ClassicR,
                            RotationSystem::ClassicR => RotationSystem::Super,
                            RotationSystem::Super => RotationSystem::Ocular,
                        };
                    }
                    2 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        if modifiers.contains(KeyModifiers::ALT) {
                            match &mut self.settings.gameplay_mut().randomizer {
                                TetrominoGenerator::Uniform => {}
                                TetrominoGenerator::Stock {
                                    tets_stocked: _,
                                    restock_multiplicity,
                                } => {
                                    *restock_multiplicity = restock_multiplicity.saturating_add(1);
                                }
                                TetrominoGenerator::Recency {
                                    tets_last_emitted: _,
                                    factor,
                                    is_base_not_exp,
                                } => {
                                    if *is_base_not_exp {
                                        *factor += ExtNonNegF64::new(0.1).unwrap();
                                    } else {
                                        *is_base_not_exp ^= true;
                                    }
                                }
                                TetrominoGenerator::BalanceOut {
                                    tets_relative_counts: _,
                                } => {}
                            };
                        } else {
                            self.settings.gameplay_mut().randomizer =
                                match self.settings.gameplay().randomizer {
                                    TetrominoGenerator::Uniform => TetrominoGenerator::bag(),
                                    TetrominoGenerator::Stock { .. } => {
                                        TetrominoGenerator::snappy_recency()
                                    }
                                    TetrominoGenerator::Recency { .. } => {
                                        TetrominoGenerator::balance_out()
                                    }
                                    TetrominoGenerator::BalanceOut { .. } => {
                                        TetrominoGenerator::Uniform
                                    }
                                };
                        }
                    }
                    3 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().preview += 1;
                    }
                    4 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().das += d_das;
                    }
                    5 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().arr += d_arr;
                    }
                    6 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().sdf += d_sdf;
                    }
                    7 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().lcd += d_lcd;
                    }
                    8 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().are += d_are;
                    }
                    9 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().initsys ^= true;
                    }
                    10 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().dtapfinesse = Some(
                            self.settings.gameplay_mut().dtapfinesse.unwrap_or_default() + d_tmf,
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
                        self.settings.gameplay_pick +=
                            self.settings.gameplay_slotmachine.slots.len() - 1;
                        self.settings.gameplay_pick %=
                            self.settings.gameplay_slotmachine.slots.len();
                    }
                    1 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().rotsys = match self.settings.gameplay().rotsys
                        {
                            RotationSystem::Raw => RotationSystem::Ocular, // Set to Ocular.
                            RotationSystem::Ocular => RotationSystem::Super,
                            RotationSystem::Super => RotationSystem::ClassicR,
                            RotationSystem::ClassicR => RotationSystem::ClassicL,
                            RotationSystem::ClassicL => RotationSystem::Ocular,
                        };
                    }
                    2 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        if modifiers.contains(KeyModifiers::ALT) {
                            match &mut self.settings.gameplay_mut().randomizer {
                                TetrominoGenerator::Uniform => {}
                                TetrominoGenerator::Stock {
                                    tets_stocked: _,
                                    restock_multiplicity,
                                } => {
                                    *restock_multiplicity =
                                        NonZeroU32::new(restock_multiplicity.get() - 1)
                                            .unwrap_or(NonZeroU32::MIN);
                                }
                                TetrominoGenerator::Recency {
                                    tets_last_emitted: _,
                                    factor,
                                    is_base_not_exp,
                                } => {
                                    if *is_base_not_exp {
                                        *is_base_not_exp ^= true;
                                    } else {
                                        *factor =
                                            factor.saturating_sub(ExtNonNegF64::new(0.1).unwrap());
                                    }
                                }
                                TetrominoGenerator::BalanceOut {
                                    tets_relative_counts: _,
                                } => {}
                            };
                        } else {
                            self.settings.gameplay_mut().randomizer =
                                match self.settings.gameplay().randomizer {
                                    TetrominoGenerator::Uniform => {
                                        TetrominoGenerator::balance_out()
                                    }
                                    TetrominoGenerator::Stock { .. } => TetrominoGenerator::Uniform,
                                    TetrominoGenerator::Recency { .. } => TetrominoGenerator::bag(),
                                    TetrominoGenerator::BalanceOut { .. } => {
                                        TetrominoGenerator::snappy_recency()
                                    }
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
                        self.settings.gameplay_mut().sdf =
                            self.settings.gameplay_mut().sdf.saturating_sub(d_sdf)
                    }
                    7 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().lcd =
                            self.settings.gameplay().lcd.saturating_sub(d_lcd);
                    }
                    8 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().are =
                            self.settings.gameplay().are.saturating_sub(d_are);
                    }
                    9 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().initsys ^= true;
                    }
                    10 => {
                        if_unmodifiable_clone_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().dtapfinesse = self
                            .settings
                            .gameplay()
                            .dtapfinesse
                            .unwrap_or_default()
                            .checked_sub(d_tmf);
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
