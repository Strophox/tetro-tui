use std::{
    io::{self, Write},
    num::NonZeroU32,
    time::Duration,
};

use crossterm::{
    QueueableCommand,
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
};
use falling_tetromino_engine::{
    DelayParameters, ExtDuration, ExtNonNegF64, Game, GameLimits, InGameTime, Stat,
};

use crate::{
    Application, GameMetaData, GameSave,
    fmt_helpers::{
        BoolAsOnOff, fmt_duration, fmt_hertz, fmt_player_input, generate_timestamp,
        increment_game_mode_derivative,
    },
    game_modding::{self, Combo},
    game_mode_presets::GameModePreset,
    game_renderers::{Renderer, ShowStats, TetroTUIRenderer},
    game_restoration::{GameRestorationData, RawInputHistory},
    tui_menus::{
        Menu, MenuUpdate, heading_line,
        replay_game::{REPLAY_ANCHOR_INTERVAL, calculate_game_and_replay_anchors},
    },
    tui_settings::{GameModePreferences, GameplaySettings},
};

impl<T: Write> Application<T> {
    pub fn run_menu_new_game(&mut self) -> io::Result<MenuUpdate> {
        let mut selected = 0usize;
        let mut customization_selected = 0usize;

        let minval_cheese = NonZeroU32::new(10).unwrap();
        let minval_combo = NonZeroU32::new(10).unwrap();

        let d_time = Duration::from_secs(5);
        let d_score = 10;
        let d_pieces = 1;
        let d_lines = 1;

        let d_fall_delay: ExtDuration = Duration::from_millis(10).into();
        let mult_fall_delay: ExtNonNegF64 = ExtNonNegF64::new(10.0).unwrap();
        let minval_fall_delay: ExtDuration = Duration::from_secs_f64(1e-9).into();
        let maxval_fall_delay: ExtDuration = Duration::from_secs_f64(100.0).into();

        loop {
            // First part: rendering the menu.
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();
            let y_selection = Self::H_MAIN / 5;

            let game_modes = self.available_base_game_modes();

            let game_save_available = if !self.game_saves.slots.is_empty() {
                1
            } else {
                0
            };
            let idx_cheese = 3;
            let idx_combo = 4;
            let idx_custom = game_modes.len();
            let opt_idx_game_save =
                (!self.game_saves.slots.is_empty()).then_some(game_modes.len() + 1);

            // Normal presets + 2 spaces if game_save option available + custom preset.
            let selection_len = game_modes.len() + game_save_available + 1;
            // There are four columns for the custom stat selection.
            let customization_selection_size = 4;
            selected %= selection_len;
            customization_selected %= customization_selection_size;
            // Render menu title.
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "+ Start New Game +").bold(),
                ))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", heading_line(&self.settings))))?;
            // Render normal and special game modes.
            for (
                i,
                GameModePreset {
                    title,
                    description,
                    show_stats: _,
                    stat_and_is_order_desc: _,
                    build: _,
                },
            ) in game_modes.iter().enumerate()
            {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(i).unwrap()
                            + if i
                                >= 2 + if self.settings.game_mode_preferences.master_mode_unlocked {
                                    1
                                } else {
                                    0
                                }
                            {
                                1
                            } else {
                                0
                            },
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if i == selected {
                            format!(">> {title}: {description} <<")
                        } else {
                            title.to_string()
                        }
                    )))?;
            }
            // Render custom mode option.
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 3 + 1 + u16::try_from(game_modes.len() + 2).unwrap(),
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == idx_custom {
                        format!(
                            "{:<42}",
                            format!(
                                "{} Custom: [Del]=reset{}{}",
                                if customization_selected == 0 {
                                    ">>"
                                } else {
                                    " |"
                                },
                                if self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .seed
                                    .is_some()
                                {
                                    " *seed"
                                } else {
                                    ""
                                },
                                if self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .start_board
                                    .is_some()
                                {
                                    " *board"
                                } else {
                                    ""
                                },
                            ),
                        )
                    } else {
                        "Custom".to_owned()
                    }
                )))?;
            /*TODO/ Render custom mode stuff.
            if selected == idx_custom {
                let stats_strs = [
                    format!(
                        "| Initial fall delay = {:?}s (Gravity: {})",
                        self.settings
                            .game_mode_preferences
                            .custom_config
                            .fall_params
                            .base_delay()
                            .as_secs_ennf64()
                            .get(),
                        fmt_hertz(
                            self.settings
                                .game_mode_preferences
                                .custom_config
                                .fall_params
                                .base_delay()
                                .as_hertz()
                        ),
                    ),
                    format!(
                        "| Progressive gravity = {}",
                        (!self
                            .settings
                            .game_mode_preferences
                            .custom_config
                            .fall_params
                            .is_constant())
                        .on_off()
                    ),
                    format!(
                        "| Limit = {:?} [→]",
                        self.settings
                            .game_mode_preferences
                            .custom_config
                            .win_condition
                    ),
                ];
                for (j, stat_str) in stats_strs.into_iter().enumerate() {
                    self.term
                        .queue(MoveTo(
                            x_main + 16 + 4 * u16::try_from(j).unwrap(),
                            y_main
                                + y_selection
                                + 3
                                + u16::try_from(1 + j + 1 + game_modes.len() + 2).unwrap(),
                        ))?
                        .queue(Print(if j + 1 == customization_selected {
                            format!(
                                ">{stat_str}{}",
                                if customization_selected != 3
                                    || self
                                        .settings
                                        .game_mode_preferences
                                        .custom_config
                                        .win_condition
                                        .is_some()
                                {
                                    " [↓/↑]"
                                } else {
                                    ""
                                }
                            )
                        } else {
                            stat_str
                        }))?;
                }
            }*/

            // Render load game save option.
            if let Some(GameSave {
                game_meta_data,
                game_restoration_data: GameRestorationData { input_history, .. },
                inputs_to_load,
            }) = &self.game_saves.get()
            {
                let load_title = &game_meta_data.title;
                let load_offset_max = input_history.inputs.len();
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + 1 + u16::try_from(game_modes.len() + 1).unwrap() + if selected == idx_custom { 5 } else { 2 },
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if Some(selected) == opt_idx_game_save {
                            if *inputs_to_load == 0 {
                                format!(">> Load {load_title} from beginning [Del] <<")
                            } else {
                                let (load_time, load_input) = input_history.inputs[(inputs_to_load - 1) % input_history.inputs.len()];
                                let load_time = fmt_duration(load_time);
                                let load_input = fmt_player_input(load_input, self.settings.tui_symbols().buttons);
                                format!(">> Load {load_title} from input {inputs_to_load}/{load_offset_max} ({load_input} @ {load_time}) [Del] <<")
                            }
                        } else {
                            format!("Game save ({load_title})")
                        },
                    )))?;
            }

            self.term.flush()?;
            // Wait for new input.
            let mut start_new_game = false;
            match event::read()? {
                // Quit app.
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
                    let client_menu_name = "New Game menu";
                    let legend = vec![
                        ("Normal keybinds".to_owned(), [
                            ("Enter e", "Select"),
                            ("Escape Backspace q", "Exit menu"),
                            ("Delete d", "Delete game save, reset configuration of Combo/Cheese/Custom modes"), // TODO: not relevant.
                            ("↓/↑ j/k", "Navigate down/up, adjust values of Custom mode"),
                            ("←/→ h/l", "Load/unload inputs for game save, Adjust values of Combo/Cheese modes, navigate Custom mode values"),
                            ("?", "Open Keybinds overview"),
                        ].into_iter().map(|(lhs,rhs)| (lhs.to_owned(), rhs.to_owned())).collect()),
                        ("Special keybinds".to_owned(), [
                            ("Home/End", "Set fall delay to infinite/zero for Custom mode, Jump to first/last input for game save"),
                            ("Alt+←/→", "Adjust start layout of Combo mode"),
                            ("Alt+↓/↑ Alt+j/k", "Adjust initial fall delay of Custom mode multiplicatively"),
                            ("Alt+Enter", "View game save as replay"),
                            ("Ctrl+U", "Unlock all game modes"),
                            ("Ctrl+Alt+L", "Reload app from savefile (overwrites current data!)"),
                            ("Ctrl+Alt+S", "Perform savefile store (respects save preferences)"),
                            ("Ctrl+C", "Exit program (respects save preferences)"),
                        ].into_iter().map(|(lhs,rhs)| (lhs.to_owned(), rhs.to_owned())).collect()),
                    ];

                    break Ok(MenuUpdate::Push(Menu::KeybindsOverview {
                        client_menu_name,
                        legend,
                    }));
                }

                // Exit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q' | 'Q') | KeyCode::Backspace,
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                // Try select mode.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e' | 'E'),
                    kind: Press,
                    modifiers,
                    ..
                }) => {
                    if modifiers.contains(KeyModifiers::ALT)
                        && let Some(GameSave {
                            game_meta_data,
                            game_restoration_data,
                            inputs_to_load: _,
                        }) = &self.game_saves.slots.get(self.game_saves.selected)
                    {
                        let replay_length = if let Some((time, _)) =
                            game_restoration_data.input_history.inputs.last()
                        {
                            *time
                        } else {
                            Duration::ZERO
                        };
                        break Ok(MenuUpdate::Push(Menu::ReplayGame {
                            game_restoration_data: Box::new(game_restoration_data.clone()),
                            game_meta_data: game_meta_data.clone(),
                            replay_length,
                            game_renderer: Box::new(TetroTUIRenderer::with_num(
                                self.temp_data.renderer_used,
                            )),
                            cached_game_and_replay_anchors: Box::new(
                                calculate_game_and_replay_anchors(
                                    &mut self.term,
                                    game_restoration_data,
                                    REPLAY_ANCHOR_INTERVAL,
                                    replay_length,
                                )?,
                            ),
                        }));
                    }
                    start_new_game = true;
                }

                // Move selector up or increase stat.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k' | 'K'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => {
                    if customization_selected > 0 {
                        match customization_selected {
                            1 => {
                                /*TODO// Increase custom fall delay.
                                let base_delay = self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params
                                    .base_delay();

                                let new_base_delay = if base_delay.is_zero() {
                                    // Bootstrap from zero to baseline.
                                    if modifiers.contains(KeyModifiers::ALT) {
                                        lowerbound_fall_delay
                                    } else {
                                        d_fall_delay
                                    }
                                } else if base_delay.is_infinite() {
                                    // Already at max.
                                    base_delay
                                } else {
                                    // Naïvely increase first.
                                    let new_base_delay = if modifiers.contains(KeyModifiers::ALT) {
                                        base_delay.mul_ennf64(mult_fall_delay)
                                    } else {
                                        base_delay + d_fall_delay
                                    };
                                    // Manually cap.
                                    if new_base_delay > maxval_fall_delay {
                                        ExtDuration::Infinite
                                    } else {
                                        new_base_delay
                                    }
                                };

                                // Adjust lock curve to either be decreasing or infinite as well.
                                self.settings
                                    .game_mode_preferences
                                    .custom_config
                                    .lock_params = if new_base_delay.is_infinite() {
                                    DelayParameters::constant(ExtDuration::Infinite)
                                } else {
                                    DelayParameters::standard_lock()
                                };

                                // Get previous fall lowerbound.
                                let lowerbound = self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params
                                    .lowerbound();

                                self.settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params = self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params
                                    .with_bounds(new_base_delay, lowerbound)
                                    .unwrap_or_else(DelayParameters::standard_fall);
                                // Normally lowerbound is 0, can only enter this if config was modified.*/
                            }
                            2 => {
                                /*TODO// Toggle decreasing fall/lock delay.
                                let (ftemp, ltemp) = if self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params
                                    .is_constant()
                                {
                                    (
                                        DelayParameters::standard_fall(),
                                        DelayParameters::standard_lock(),
                                    )
                                } else {
                                    // Note delay args don't matter, we're interested in constant factor and subtrahend coefficients not the delay.
                                    (
                                        DelayParameters::constant(Default::default()),
                                        DelayParameters::constant(Default::default()),
                                    )
                                };
                                self.settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params = self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params
                                    .with_coefficients(ftemp.factor(), ftemp.subtrahend())
                                    .unwrap();
                                self.settings
                                    .game_mode_preferences
                                    .custom_config
                                    .lock_params = self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .lock_params
                                    .with_coefficients(ltemp.factor(), ltemp.subtrahend())
                                    .unwrap();*/
                            }
                            3 => {
                                match self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .win_condition
                                {
                                    Some(Stat::TimeElapsed(ref mut t)) => {
                                        *t += d_time;
                                    }
                                    Some(Stat::PiecesLocked(ref mut p)) => {
                                        *p += d_pieces;
                                    }
                                    Some(Stat::LinesCleared(ref mut l)) => {
                                        *l += d_lines;
                                    }
                                    Some(Stat::PointsScored(ref mut s)) => {
                                        *s += d_score;
                                    }
                                    None => {}
                                };
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        selected += selection_len - 1;
                    }
                }

                // Move selector down or decrease stat.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j' | 'J'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => {
                    // Selected custom stat; decrease it.
                    if customization_selected > 0 {
                        match customization_selected {
                            1 => {
                                /*TODO// Increase custom fall delay.
                                let base_delay = self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params
                                    .base_delay();

                                let new_base_delay = if base_delay.is_zero() {
                                    // Already at zero, leave it.
                                    base_delay
                                } else if base_delay.is_infinite() {
                                    // Bootstrap(?) it down from infinity to upper bound.
                                    maxval_fall_delay
                                } else {
                                    // Naïvely decrease first.
                                    let new_base_delay = if modifiers.contains(KeyModifiers::ALT) {
                                        base_delay.div_ennf64(mult_fall_delay)
                                    } else {
                                        base_delay.saturating_sub(d_fall_delay)
                                    };
                                    // Manually cap.
                                    if new_base_delay < lowerbound_fall_delay {
                                        ExtDuration::ZERO
                                    } else {
                                        new_base_delay
                                    }
                                };

                                // Adjust lock curve to either be decreasing or infinite as well.
                                self.settings
                                    .game_mode_preferences
                                    .custom_config
                                    .lock_params = if new_base_delay.is_infinite() {
                                    DelayParameters::constant(ExtDuration::Infinite)
                                } else {
                                    DelayParameters::standard_lock()
                                };

                                let lowerbound = self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params
                                    .lowerbound();

                                self.settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params = self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params
                                    .with_bounds(new_base_delay, lowerbound)
                                    .unwrap_or_else(DelayParameters::standard_fall);
                                // Normally lowerbound is 0, can only enter this if config was modified.*/
                            }
                            2 => {
                                /*TODO// Toggle decreasing fall/lock delay.
                                let (ftemp, ltemp) = if self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params
                                    .is_constant()
                                {
                                    (
                                        DelayParameters::standard_fall(),
                                        DelayParameters::standard_lock(),
                                    )
                                } else {
                                    // Note delay args don't matter, we're interested in constant factor and subtrahend coefficients not the delay.
                                    (
                                        DelayParameters::constant(Default::default()),
                                        DelayParameters::constant(Default::default()),
                                    )
                                };
                                self.settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params = self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .fall_params
                                    .with_coefficients(ftemp.factor(), ftemp.subtrahend())
                                    .unwrap();
                                self.settings
                                    .game_mode_preferences
                                    .custom_config
                                    .lock_params = self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .lock_params
                                    .with_coefficients(ltemp.factor(), ltemp.subtrahend())
                                    .unwrap();*/
                            }
                            3 => {
                                match self
                                    .settings
                                    .game_mode_preferences
                                    .custom_config
                                    .win_condition
                                {
                                    Some(Stat::TimeElapsed(ref mut t)) => {
                                        *t = t.saturating_sub(d_time);
                                    }
                                    Some(Stat::PiecesLocked(ref mut p)) => {
                                        *p = p.saturating_sub(d_pieces);
                                    }
                                    Some(Stat::LinesCleared(ref mut l)) => {
                                        *l = l.saturating_sub(d_lines);
                                    }
                                    Some(Stat::PointsScored(ref mut s)) => {
                                        *s = s.saturating_sub(d_score);
                                    }
                                    None => {}
                                };
                            }
                            _ => unreachable!(),
                        }
                    // Move gamemode selector
                    } else {
                        selected += 1;
                    }
                }

                // Move selector left (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => {
                    if selected == idx_custom && customization_selected > 0 {
                        customization_selected += customization_selection_size - 1
                    } else if selected == idx_cheese {
                        if let Some(limit) = self.settings.game_mode_preferences.cheese_config.limit
                        {
                            self.settings.game_mode_preferences.cheese_config.limit =
                                if limit > minval_cheese {
                                    NonZeroU32::try_from(limit.get() - 1).ok()
                                } else {
                                    None
                                };
                        }
                    } else if selected == idx_combo {
                        if modifiers.contains(KeyModifiers::ALT) {
                            let new_layout_idx = if let Some(i) =
                                Combo::LAYOUTS.iter().position(|lay| {
                                    *lay == self
                                        .settings
                                        .game_mode_preferences
                                        .combo_config
                                        .start_layout
                                }) {
                                let layout_cnt = Combo::LAYOUTS.len();
                                (i + layout_cnt - 1) % layout_cnt
                            } else {
                                0
                            };
                            self.settings
                                .game_mode_preferences
                                .combo_config
                                .start_layout = Combo::LAYOUTS[new_layout_idx];
                        } else if let Some(limit) =
                            self.settings.game_mode_preferences.combo_config.limit
                        {
                            self.settings.game_mode_preferences.combo_config.limit =
                                if limit > minval_combo {
                                    NonZeroU32::try_from(limit.get() - 1).ok()
                                } else {
                                    None
                                };
                        }
                    } else if Some(selected) == opt_idx_game_save
                        && let Some(GameSave {
                            game_restoration_data: GameRestorationData { input_history, .. },
                            inputs_to_load,
                            ..
                        }) = self.game_saves.get_mut()
                    {
                        *inputs_to_load += input_history.inputs.len()
                            * if modifiers.contains(KeyModifiers::ALT) {
                                20
                            } else {
                                1
                            };
                        *inputs_to_load %= input_history.inputs.len() + 1;
                    }
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

                // Move selector right (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => {
                    // If custom gamemode selected, allow incrementing stat selection.
                    if selected == idx_custom {
                        // If reached last stat, cycle through stats for limit.
                        if customization_selected == customization_selection_size - 1 {
                            self.settings
                                .game_mode_preferences
                                .custom_config
                                .win_condition = match self
                                .settings
                                .game_mode_preferences
                                .custom_config
                                .win_condition
                            {
                                Some(Stat::TimeElapsed(_)) => Some(Stat::PointsScored(200)),
                                Some(Stat::PointsScored(_)) => Some(Stat::PiecesLocked(100)),
                                Some(Stat::PiecesLocked(_)) => Some(Stat::LinesCleared(40)),
                                Some(Stat::LinesCleared(_)) => None,
                                None => Some(Stat::TimeElapsed(Duration::from_secs(300))),
                            };
                        } else {
                            customization_selected += 1
                        }
                    } else if selected == idx_cheese {
                        self.settings.game_mode_preferences.cheese_config.limit =
                            if let Some(limit) =
                                self.settings.game_mode_preferences.cheese_config.limit
                            {
                                limit.checked_add(1)
                            } else {
                                Some(minval_cheese)
                            };
                    } else if selected == idx_combo {
                        if modifiers.contains(KeyModifiers::ALT) {
                            let new_layout_idx = if let Some(i) =
                                Combo::LAYOUTS.iter().position(|lay| {
                                    *lay == self
                                        .settings
                                        .game_mode_preferences
                                        .combo_config
                                        .start_layout
                                }) {
                                let layout_cnt = Combo::LAYOUTS.len();
                                (i + 1) % layout_cnt
                            } else {
                                0
                            };
                            self.settings
                                .game_mode_preferences
                                .combo_config
                                .start_layout = Combo::LAYOUTS[new_layout_idx];
                        } else {
                            self.settings.game_mode_preferences.combo_config.limit =
                                if let Some(limit) =
                                    self.settings.game_mode_preferences.combo_config.limit
                                {
                                    limit.checked_add(1)
                                } else {
                                    Some(minval_combo)
                                };
                        }
                    } else if Some(selected) == opt_idx_game_save
                        && let Some(GameSave {
                            game_restoration_data: GameRestorationData { input_history, .. },
                            inputs_to_load,
                            ..
                        }) = self.game_saves.get_mut()
                    {
                        *inputs_to_load += if modifiers.contains(KeyModifiers::ALT) {
                            20
                        } else {
                            1
                        };
                        *inputs_to_load %= input_history.inputs.len() + 1;
                    }
                }

                // Load first input for game save.
                Event::Key(KeyEvent {
                    code: KeyCode::Home,
                    kind: Press | Repeat,
                    ..
                }) => {
                    /*TODO// If custom gamemode selected, allow setting speed curve to 'zero gravity'.
                    if selected == idx_custom
                    /*&& customization_selected == customization_selection_size - 1*/
                    {
                        self.settings
                            .game_mode_preferences
                            .custom_config
                            .fall_params = DelayParameters::constant(ExtDuration::Infinite);
                        self.settings
                            .game_mode_preferences
                            .custom_config
                            .lock_params = DelayParameters::constant(ExtDuration::Infinite);
                    } else if Some(selected) == opt_idx_game_save {
                        if let Some(GameSave { inputs_to_load, .. }) = self.game_saves.get_mut() {
                            *inputs_to_load = 0;
                        }
                    }*/
                }

                // Load last input for game save.
                Event::Key(KeyEvent {
                    code: KeyCode::End,
                    kind: Press | Repeat,
                    ..
                }) => {
                    /*TODO// If custom gamemode selected, allow setting speed curve to 'zero gravity'.
                    if selected == idx_custom
                    /*&& customization_selected == customization_selection_size - 1*/
                    {
                        self.settings
                            .game_mode_preferences
                            .custom_config
                            .fall_params = DelayParameters::constant(ExtDuration::ZERO);
                        self.settings
                            .game_mode_preferences
                            .custom_config
                            .lock_params = DelayParameters::standard_lock();
                    } else if Some(selected) == opt_idx_game_save {
                        if let Some(GameSave {
                            game_restoration_data: GameRestorationData { input_history, .. },
                            inputs_to_load,
                            ..
                        }) = self.game_saves.get_mut()
                        {
                            *inputs_to_load = input_history.inputs.len();
                        }
                    }*/
                }

                // Move selector right (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d' | 'D'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => {
                    /*TODOif selected == idx_custom {
                        self.settings.game_mode_preferences.custom_config.seed = None;
                        self.settings
                            .game_mode_preferences
                            .custom_config
                            .start_board = None;
                        self.settings
                            .game_mode_preferences
                            .custom_config
                            .fall_params = DelayParameters::standard_fall();
                        self.settings
                            .game_mode_preferences
                            .custom_config
                            .lock_params = DelayParameters::standard_lock();
                        self.settings
                            .game_mode_preferences
                            .custom_config
                            .win_condition = None;
                    } else if selected == idx_cheese {
                        self.settings.game_mode_preferences.cheese_config.limit =
                            GameModePreferences::default().cheese_config.limit;
                    } else if selected == idx_combo {
                        if modifiers.contains(KeyModifiers::ALT) {
                            self.settings
                                .game_mode_preferences
                                .combo_config
                                .start_layout = Combo::LAYOUTS[0];
                        } else {
                            self.settings.game_mode_preferences.combo_config.limit =
                                GameModePreferences::default().combo_config.limit;
                        }
                    } else if Some(selected) == opt_idx_game_save {
                        self.game_saves.slots.remove(self.game_saves.selected);
                        self.game_saves.selected = 0;
                    }*/
                }

                // Secret - This unlocks things.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('u' | 'U'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    ..
                }) => {
                    self.settings.game_mode_preferences.master_mode_unlocked = true;
                    self.settings
                        .game_mode_preferences
                        .experimental_mode_unlocked = true;
                }

                // Other event: don't care.
                _ => {}
            }

            if start_new_game {
                let game_menu = self.create_game_menu(selected);
                self.statistics.new_games_started += 1;

                break Ok(MenuUpdate::Push(game_menu));
            }
        }
    }

    pub fn available_base_game_modes(&self) -> Vec<GameModePreset> {
        let mut game_modes = vec![
            GameModePreset::swift(),
            GameModePreset::regular(),
            GameModePreset::puzzle(),
            GameModePreset::survival(self.settings.game_mode_preferences.survival_config),
            GameModePreset::cheese(
                self.settings.game_mode_preferences.cheese_config,
                self.settings
                    .game_mode_preferences
                    .cheese_fall_and_lock_delays,
            ),
            GameModePreset::combo(self.settings.game_mode_preferences.combo_config),
        ];

        if self.settings.game_mode_preferences.master_mode_unlocked {
            game_modes.insert(2, GameModePreset::master());
        }

        if self
            .settings
            .game_mode_preferences
            .experimental_mode_unlocked
        {
            game_modes.push(GameModePreset::ascent())
        }

        game_modes
    }

    pub fn create_game_menu(&self, selection: usize) -> Menu {
        let game_modes = self.available_base_game_modes();

        let GameplaySettings {
            rotsys,
            tetgen,
            preview: prev,
            das,
            arr,
            sdf,
            lcd,
            are,
            initsys,
            dtapfinesse: _,
        } = *self.settings.gameplay();

        let mut builder = Game::builder();

        builder
            .rotation_system(rotsys)
            .tetromino_generator(tetgen)
            .generate_piece_preview(prev)
            .delayed_auto_shift(das)
            .auto_repeat_rate(arr)
            .soft_drop_speedup(sdf)
            .line_clear_duration(lcd)
            .spawn_delay(are)
            .allow_spawn_manipulation(initsys);

        let (game_meta_data, mut game, raw_input_history) = if selection < game_modes.len() {
            // Build one of the selected game modes.
            let GameModePreset {
                title,
                description: _,
                show_stats,
                stat_and_is_order_desc,
                build,
            } = &game_modes[selection];

            let preset_game = build(&builder);

            let preset_game_meta_data = GameMetaData {
                timestamp: generate_timestamp(),
                title: title.to_owned(),
                show_stats: *show_stats,
                stat_and_desc_order: *stat_and_is_order_desc,
            };

            let blank_input_history = RawInputHistory::default();

            (preset_game_meta_data, preset_game, blank_input_history)
        } else if selection == game_modes.len() + 1 && self.game_saves.get().is_some() {
            // Load saved game.
            // SAFETY: `self.game_saves.get().is_some()`.
            let GameSave {
                game_meta_data,
                game_restoration_data,
                inputs_to_load,
            } = &self.game_saves.get().unwrap();

            let restored_game = game_restoration_data.restore(*inputs_to_load);

            let mut restored_game_meta_data = game_meta_data.clone();
            // Mark restored game as such.
            increment_game_mode_derivative(&mut restored_game_meta_data.title);

            let restored_input_history = game_restoration_data
                .input_history
                .inputs
                .iter()
                .take(*inputs_to_load)
                .copied()
                .collect::<Vec<_>>()
                .into();

            (
                restored_game_meta_data,
                restored_game,
                restored_input_history,
            )
        } else {
            // Build custom game.
            let n = &self.settings.game_mode_preferences;

            builder
                .fall_delay_curve(n.custom_config.fall_curve.clone())
                .lock_delay_curve(n.custom_config.lock_curve.clone())
                .game_limits(match n.custom_config.win_condition {
                    Some(stat) => GameLimits::single(stat, true),
                    None => GameLimits::new(),
                });

            // Optionally load custom seed.
            if let Some(seed) = n.custom_config.seed {
                builder.seed(seed);
            }

            // Optionally load custom board.
            let new_custom_game = if let Some(encoded_board) = &n.custom_config.start_board {
                game_modding::StartBoard::build(&builder, encoded_board.clone())
            // Otherwise just build a normal custom game.
            } else {
                builder.build()
            };

            let title = match n.custom_config.win_condition {
                Some(stat) => match stat {
                    Stat::TimeElapsed(duration) => format!("Time-{}s", duration.as_secs()),
                    Stat::PiecesLocked(p) => format!("Pieces-{p}"),
                    Stat::LinesCleared(l) => format!("Lines-{l}"),
                    Stat::PointsScored(s) => format!("Score-{s}"),
                },
                None => "Limitless".to_owned(),
            };

            // TODO: Changeable?
            const CUSTOM_SHOW_STATS: ShowStats = ShowStats::all();

            let custom_game_meta_data = GameMetaData {
                timestamp: generate_timestamp(),
                title,
                show_stats: CUSTOM_SHOW_STATS,
                stat_and_desc_order: (Stat::PointsScored(0), false),
            };
            let blank_input_history = RawInputHistory::default();
            (
                /*success or just fallback?: selection == game_modes.len(),*/
                custom_game_meta_data,
                new_custom_game,
                blank_input_history,
            )
        };
        // FIXME: Unused code: modifier addition.
        // game.modifiers.push(game_mode_presets::game_modifiers::print_fall_delay::modifier());
        // game.modifiers.push(game_mode_presets::game_modifiers::misc_modifiers::print_recency_tet_gen_stats::modifier());
        // game.modifiers.push(falling_tetromino_engine::Modifier { descriptor: "always_clear_board".to_owned(), mod_function: Box::new(|_c, _i, s, _m, _f| { s.board = Default::default(); })});

        let mut game_renderer = TetroTUIRenderer::with_num(self.temp_data.renderer_used);

        // We do an initial update, which allows a piece to spawn and queue to get generated.
        // We do this so the renderer does not render a first frame when game is in its raw start state.
        if game.state().time.is_zero() {
            match game.update(InGameTime::ZERO, None) {
                Ok(msgs) => game_renderer.update_feed(msgs, &self.settings),
                // ?? but i didn't even do anything yet
                Err(_update_game_error) => {}
            }
        }

        Menu::PlayGame {
            game: game.into(),
            raw_input_history,
            game_meta_data,
            game_renderer: game_renderer.into(),
            selection_id_for_game_retry: Some(selection),
        }
    }
}
