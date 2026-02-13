use std::{
    io::{self, Write},
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
use tetrs_engine::{ExtNonNegF64, RotationSystem, TetrominoGenerator};

use crate::application::{Application, Menu, MenuUpdate, Settings};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_adjust_gameplay(&mut self) -> io::Result<MenuUpdate> {
        let if_slot_is_default_then_copy_and_switch = |settings: &mut Settings| {
            if settings.gameplay_slot_active < settings.gameplay_slots_that_should_not_be_changed {
                let mut n = 1;
                let new_custom_slot_name = loop {
                    let name = format!("Custom-{n}");
                    if settings.gameplay_slots.iter().any(|s| s.0 == name) {
                        n += 1;
                    } else {
                        break name;
                    }
                };
                let new_slot = (new_custom_slot_name, settings.gameplay().clone());
                settings.gameplay_slots.push(new_slot);
                settings.gameplay_slot_active = settings.gameplay_slots.len() - 1;
            }
        };

        let d_das = Duration::from_millis(1);
        let d_arr = Duration::from_millis(1);
        let d_sdf = ExtNonNegF64::new(0.2).unwrap();
        let d_lcd = Duration::from_millis(5);
        let d_are = Duration::from_millis(5);

        let selection_len = 11;
        let mut selected = 1usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 7;

            // Draw menu title.
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    "= Gameplay Configurations (apply on New Game) ="
                )))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;

            // Draw slot label.
            let slot_label = format!(
                "Slot ({}/{}): \"{}\"{}",
                self.settings.gameplay_slot_active + 1,
                self.settings.gameplay_slots.len(),
                self.settings.gameplay_slots[self.settings.gameplay_slot_active].0,
                if self.settings.gameplay_slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.gameplay_slot_active
                            < self.settings.gameplay_slots_that_should_not_be_changed
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
                    "Rotation system: {:?}",
                    self.settings.gameplay().rotation_system
                ),
                format!(
                    "Tetromino generator: {}",
                    match &self.settings.gameplay().tetromino_generator {
                        TetrominoGenerator::Uniform => "Uniformly random".to_owned(),
                        TetrominoGenerator::Stock { .. } => "Bag".to_owned(),
                        TetrominoGenerator::Recency { .. } => "Recency".to_owned(),
                        TetrominoGenerator::BalanceRelative { .. } =>
                            "Balance relative counts".to_owned(),
                    }
                ),
                format!(
                    "Piece preview count: {}",
                    self.settings.gameplay().piece_preview_count
                ),
                format!(
                    "Delayed auto shift (DAS): {:?} *",
                    self.settings.gameplay().delayed_auto_shift
                ),
                format!(
                    "Auto repeat rate (ARR): {:?} *",
                    self.settings.gameplay().auto_repeat_rate
                ),
                format!(
                    "Soft drop factor (SDF): {} *",
                    self.settings.gameplay().soft_drop_factor.get()
                ),
                format!(
                    "Line clear duration (LCD): {:?}",
                    self.settings.gameplay().line_clear_duration
                ),
                format!(
                    "Spawn delay (ARE): {:?}",
                    self.settings.gameplay().spawn_delay
                ),
                format!(
                    "Allow pre-spawn rotation/hold (IRS/IHS): {:?} *",
                    self.settings.gameplay().allow_prespawn_actions
                ),
                format!(
                    "/!\\ Override - assume enhanced-key-events: {} *",
                    self.runtime_data.kitty_assumed
                ),
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
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 6 + u16::try_from(selection_len).unwrap(),
                ))?
                .queue(PrintStyledContent(
                    format!(
                        "{:^w_main$}",
                        if self.runtime_data.kitty_detected {
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
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => break Ok(MenuUpdate::Push(Menu::Quit)),
                Event::Key(KeyEvent {
                    code:
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace | KeyCode::Char('b'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                // Reset config, or delete entire slot.
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press,
                    ..
                }) => {
                    if selected == 0 {
                        // If a custom slot, then remove it (and return to the 'default' 0th slot).
                        if self.settings.gameplay_slot_active
                            >= self.settings.gameplay_slots_that_should_not_be_changed
                        {
                            self.settings
                                .gameplay_slots
                                .remove(self.settings.gameplay_slot_active);
                            self.settings.gameplay_slot_active = 0;
                        }
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
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.gameplay_slot_active += 1;
                        self.settings.gameplay_slot_active %= self.settings.gameplay_slots.len();
                    }
                    1 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().rotation_system =
                            match self.settings.gameplay().rotation_system {
                                RotationSystem::Ocular => RotationSystem::Classic,
                                RotationSystem::Classic => RotationSystem::Super,
                                RotationSystem::Super => RotationSystem::Ocular,
                            };
                    }
                    2 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().tetromino_generator =
                            match self.settings.gameplay().tetromino_generator {
                                TetrominoGenerator::Uniform => TetrominoGenerator::bag(),
                                TetrominoGenerator::Stock { .. } => TetrominoGenerator::recency(),
                                TetrominoGenerator::Recency { .. } => {
                                    TetrominoGenerator::balance_relative()
                                }
                                TetrominoGenerator::BalanceRelative { .. } => {
                                    TetrominoGenerator::uniform()
                                }
                            };
                    }
                    3 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().piece_preview_count += 1;
                    }
                    4 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().delayed_auto_shift += d_das;
                    }
                    5 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().auto_repeat_rate += d_arr;
                    }
                    6 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().soft_drop_factor += d_sdf;
                    }
                    7 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().line_clear_duration += d_lcd;
                    }
                    8 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().spawn_delay += d_are;
                    }
                    9 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().allow_prespawn_actions ^= true;
                    }
                    10 => {
                        self.runtime_data.kitty_assumed ^= true;
                    }
                    _ => {}
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    ..
                }) => match selected {
                    0 => {
                        self.settings.gameplay_slot_active +=
                            self.settings.gameplay_slots.len() - 1;
                        self.settings.gameplay_slot_active %= self.settings.gameplay_slots.len();
                    }
                    1 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().rotation_system =
                            match self.settings.gameplay().rotation_system {
                                RotationSystem::Ocular => RotationSystem::Super,
                                RotationSystem::Super => RotationSystem::Classic,
                                RotationSystem::Classic => RotationSystem::Ocular,
                            };
                    }
                    2 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().tetromino_generator =
                            match self.settings.gameplay().tetromino_generator {
                                TetrominoGenerator::Uniform => {
                                    TetrominoGenerator::balance_relative()
                                }
                                TetrominoGenerator::Stock { .. } => TetrominoGenerator::uniform(),
                                TetrominoGenerator::Recency { .. } => TetrominoGenerator::bag(),
                                TetrominoGenerator::BalanceRelative { .. } => {
                                    TetrominoGenerator::recency()
                                }
                            };
                    }
                    3 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().piece_preview_count = self
                            .settings
                            .gameplay()
                            .piece_preview_count
                            .saturating_sub(1);
                    }
                    4 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().delayed_auto_shift = self
                            .settings
                            .gameplay()
                            .delayed_auto_shift
                            .saturating_sub(d_das);
                    }
                    5 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().auto_repeat_rate = self
                            .settings
                            .gameplay()
                            .auto_repeat_rate
                            .saturating_sub(d_arr);
                    }
                    6 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().soft_drop_factor = self
                            .settings
                            .gameplay_mut()
                            .soft_drop_factor
                            .saturating_sub(d_sdf)
                    }
                    7 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().line_clear_duration = self
                            .settings
                            .gameplay()
                            .line_clear_duration
                            .saturating_sub(d_lcd);
                    }
                    8 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().spawn_delay =
                            self.settings.gameplay().spawn_delay.saturating_sub(d_are);
                    }
                    9 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.gameplay_mut().allow_prespawn_actions ^= true;
                    }
                    10 => {
                        self.runtime_data.kitty_assumed ^= true;
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
