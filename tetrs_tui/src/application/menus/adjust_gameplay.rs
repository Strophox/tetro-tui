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
use tetrs_engine::{RotationSystem, TetrominoSource};

use crate::application::{Application, Menu, MenuUpdate, Settings};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_adjust_gameplay(&mut self) -> io::Result<MenuUpdate> {
        let if_slot_is_default_then_copy_and_switch = |settings: &mut Settings| {
            if settings.config_slot_active < settings.config_slots_that_should_not_be_changed {
                let mut n = 1;
                let new_custom_slot_name = loop {
                    let name = format!("custom_{n}");
                    if settings.config_slots.iter().any(|s| s.0 == name) {
                        n += 1;
                    } else {
                        break name;
                    }
                };
                let new_slot = (new_custom_slot_name, settings.config().clone());
                settings.config_slots.push(new_slot);
                settings.config_slot_active = settings.config_slots.len() - 1;
            }
        };
        let selection_len = 10;
        let mut selected = 1usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;

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
                self.settings.config_slot_active + 1,
                self.settings.config_slots.len(),
                self.settings.config_slots[self.settings.config_slot_active].0,
                if self.settings.config_slots.len() < 2 {
                    "".to_owned()
                } else {
                    format!(
                        " [←|{}→] ",
                        if self.settings.config_slot_active
                            < self.settings.config_slots_that_should_not_be_changed
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
                    self.settings.config().rotation_system
                ),
                format!(
                    "Piece generation: {}",
                    match &self.settings.config().tetromino_generator {
                        TetrominoSource::Uniform => "Uniformly random".to_owned(),
                        TetrominoSource::Stock { .. } => "Bag".to_owned(),
                        TetrominoSource::Recency { .. } => "Recency".to_owned(),
                        TetrominoSource::BalanceRelative { .. } =>
                            "Balance relative counts".to_owned(),
                        TetrominoSource::Cycle { pattern, index: _ } =>
                            format!("Cycling pattern {pattern:?}"),
                    }
                ),
                format!("Preview size: {}", self.settings.config().preview_count),
                format!(
                    "Delayed auto shift: {:?} *",
                    self.settings.config().delayed_auto_shift
                ),
                format!(
                    "Auto repeat rate: {:?} *",
                    self.settings.config().auto_repeat_rate
                ),
                format!(
                    "Soft drop factor: {} *",
                    self.settings.config().soft_drop_factor
                ),
                format!(
                    "Line clear delay: {:?}",
                    self.settings.config().line_clear_delay
                ),
                format!(
                    "Appearance delay: {:?}",
                    self.settings.config().appearance_delay
                ),
                format!(
                    "(/!\\ Override) Assume enhanced-key-events: {} *",
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
                            "(*Should apply, since enhanced-key-events seem available)"
                        } else {
                            "(*Might NOT apply since enhanced-key-events seem unavailable)"
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
                }) => {
                    break Ok(MenuUpdate::Push(Menu::Quit(
                        "exited with ctrl-c".to_owned(),
                    )))
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
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
                        if self.settings.config_slot_active
                            >= self.settings.config_slots_that_should_not_be_changed
                        {
                            self.settings
                                .config_slots
                                .remove(self.settings.config_slot_active);
                            self.settings.config_slot_active = 0;
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
                        self.settings.config_slot_active += 1;
                        self.settings.config_slot_active %= self.settings.config_slots.len();
                    }
                    1 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().rotation_system =
                            match self.settings.config().rotation_system {
                                RotationSystem::Ocular => RotationSystem::Classic,
                                RotationSystem::Classic => RotationSystem::Super,
                                RotationSystem::Super => RotationSystem::Ocular,
                            };
                    }
                    2 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().tetromino_generator = match self
                            .settings
                            .config()
                            .tetromino_generator
                        {
                            TetrominoSource::Uniform => TetrominoSource::bag(),
                            TetrominoSource::Stock { .. } => TetrominoSource::recency(),
                            TetrominoSource::Recency { .. } => TetrominoSource::balance_relative(),
                            TetrominoSource::BalanceRelative { .. } => TetrominoSource::uniform(),
                            TetrominoSource::Cycle { .. } => TetrominoSource::uniform(),
                        };
                    }
                    3 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().preview_count += 1;
                    }
                    4 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().delayed_auto_shift += Duration::from_millis(1);
                    }
                    5 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().auto_repeat_rate += Duration::from_millis(1);
                    }
                    6 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().soft_drop_factor += 0.5;
                    }
                    7 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().line_clear_delay += Duration::from_millis(10);
                    }
                    8 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().appearance_delay += Duration::from_millis(10);
                    }
                    9 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
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
                        self.settings.config_slot_active += self.settings.config_slots.len() - 1;
                        self.settings.config_slot_active %= self.settings.config_slots.len();
                    }
                    1 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().rotation_system =
                            match self.settings.config().rotation_system {
                                RotationSystem::Ocular => RotationSystem::Super,
                                RotationSystem::Super => RotationSystem::Classic,
                                RotationSystem::Classic => RotationSystem::Ocular,
                            };
                    }
                    2 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().tetromino_generator =
                            match self.settings.config().tetromino_generator {
                                TetrominoSource::Uniform => TetrominoSource::balance_relative(),
                                TetrominoSource::Stock { .. } => TetrominoSource::uniform(),
                                TetrominoSource::Recency { .. } => TetrominoSource::bag(),
                                TetrominoSource::BalanceRelative { .. } => {
                                    TetrominoSource::recency()
                                }
                                TetrominoSource::Cycle { .. } => TetrominoSource::uniform(),
                            };
                    }
                    3 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().preview_count =
                            self.settings.config().preview_count.saturating_sub(1);
                    }
                    4 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().delayed_auto_shift = self
                            .settings
                            .config()
                            .delayed_auto_shift
                            .saturating_sub(Duration::from_millis(1));
                    }
                    5 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().auto_repeat_rate = self
                            .settings
                            .config()
                            .auto_repeat_rate
                            .saturating_sub(Duration::from_millis(1));
                    }
                    6 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        if self.settings.config().soft_drop_factor > 0.0 {
                            self.settings.config_mut().soft_drop_factor -= 0.5;
                        }
                    }
                    7 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().line_clear_delay = self
                            .settings
                            .config()
                            .line_clear_delay
                            .saturating_sub(Duration::from_millis(10));
                    }
                    8 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
                        self.settings.config_mut().appearance_delay = self
                            .settings
                            .config()
                            .appearance_delay
                            .saturating_sub(Duration::from_millis(10));
                    }
                    9 => {
                        if_slot_is_default_then_copy_and_switch(&mut self.settings);
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
