use std::{
    io::{self, Write},
    num::NonZeroU32,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::Print,
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use tetrs_engine::{DelayParameters, ExtDuration, ExtNonNegF64, Game, Stat};

use crate::{
    application::{
        Application, ButtonInputHistory, GameMetaData, GameRestorationData, GameplaySettings, Menu,
        MenuUpdate,
    },
    fmt_helpers::{fmt_button_change, fmt_duration, fmt_hertz},
    game_mode_presets::{
        self, game_modifiers::combo_board::LAYOUTS as COMBO_STARTLAYOUTS, GameModePreset,
    },
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_new_game(&mut self) -> io::Result<MenuUpdate> {
        let mut selected = 0usize;
        let mut customization_selected = 0usize;

        let d_time = Duration::from_secs(10);
        let d_score = 10;
        let d_pieces = 1;
        let d_lines = 1;

        let d_fall_delay = ExtDuration::from(Duration::from_millis(100));
        let mult_fall_delay = ExtNonNegF64::from(10);

        loop {
            #[allow(clippy::type_complexity)]
            let mut game_presets: Vec<(GameModePreset, String)> = vec![
                (
                    game_mode_presets::forty_lines(),
                    "How fast can you clear forty lines?".to_owned(),
                ),
                (
                    game_mode_presets::marathon(),
                    "Clear 150 lines at increasing gravity.".to_owned(),
                ),
                (
                    game_mode_presets::time_trial(),
                    "How high a score can you get in 3 min.?".to_owned(),
                ),
                (
                    game_mode_presets::master(),
                    "Clear 150 lines at instant gravity.".to_owned(),
                ),
                (
                    game_mode_presets::puzzle(),
                    "Clear 24 hand-crafted puzzles.".to_owned(),
                ),
                (
                    game_mode_presets::n_cheese(
                        self.settings.new_game.cheese_linelimit,
                        self.settings.new_game.cheese_tiles_per_line,
                        self.settings.new_game.cheese_fall_delay,
                    ),
                    format!(
                        "Eat through lines like Swiss cheese. Limit: {:?}",
                        self.settings.new_game.cheese_linelimit
                    ),
                ),
                (
                    game_mode_presets::n_combo(
                        self.settings.new_game.combo_linelimit,
                        self.settings.new_game.combo_startlayout,
                    ),
                    format!(
                        "Get consecutive line clears. Limit: {:?}{}",
                        self.settings.new_game.combo_linelimit,
                        if self.settings.new_game.combo_startlayout != COMBO_STARTLAYOUTS[0] {
                            format!(", Layout={:b}", self.settings.new_game.combo_startlayout)
                        } else {
                            "".to_owned()
                        }
                    ),
                ),
            ];
            if self.settings.new_game.experimental_mode_unlocked {
                game_presets.push((
                    game_mode_presets::ascent(),
                    "(Experimental. Needs 180° rot.) Per aspera ad astra".to_owned(),
                ))
            }
            // First part: rendering the menu.
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            let savepoint_available = if self.game_savepoint.is_some() { 1 } else { 0 };
            // Normal presets + 2 spaces if savepoint option available + custom preset.
            let selection_len = game_presets.len() + savepoint_available + 1;
            // There are four columns for the custom stat selection.
            let customization_selection_size = 4;
            selected %= selection_len;
            customization_selected %= customization_selection_size;
            // Render menu title.
            self.term
                .queue(Clear(ClearType::All))?
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(Print(format!("{:^w_main$}", "+ Start New Game +")))?
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            // Render normal and special gamemodes.
            for (i, ((title, _cmp_stat, _build), description)) in game_presets.iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main
                            + y_selection
                            + 4
                            + u16::try_from(i + if 4 <= i { 1 } else { 0 }).unwrap(),
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
            // Render load savepoint option.
            if let Some((game_meta_data, GameRestorationData { input_history, .. }, load_offset)) =
                &self.game_savepoint
            {
                let load_title = &game_meta_data.title;
                let load_offset_max = input_history.0.len();
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(game_presets.len() + 2).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if selected == selection_len - 2 {
                            if *load_offset == 0 {
                                format!(">> Load {load_title:?} run from start [Del] <<")
                            } else {
                                let (load_time, load_input) = ButtonInputHistory::decode(
                                    input_history.0[(load_offset - 1) % input_history.0.len()]);
                                let load_time = fmt_duration(load_time);
                                let load_input = fmt_button_change(load_input);
                                format!(">> Load {load_title} from: {load_offset}/{load_offset_max} ({load_input} @{load_time}) [Del] <<")
                            }
                        } else {
                            format!("Savepoint - {load_title} run...")
                        },
                    )))?;
            }
            // Render custom mode option.
            self.term
                .queue(MoveTo(
                    x_main,
                    y_main
                        + y_selection
                        + 4
                        + u16::try_from(selection_len + savepoint_available + 1).unwrap(),
                ))?
                .queue(Print(format!(
                    "{:^w_main$}",
                    if selected == selection_len - 1 {
                        format!(
                            "{:<42}",
                            format!(
                                "{} Custom: [Del]=reset{}{}",
                                if customization_selected == 0 {
                                    ">>"
                                } else {
                                    " |"
                                },
                                if self.settings.new_game.custom_seed.is_some() {
                                    " seed"
                                } else {
                                    ""
                                },
                                if self.settings.new_game.custom_board.is_some() {
                                    " board"
                                } else {
                                    ""
                                },
                            ),
                        )
                    } else {
                        "Custom".to_owned()
                    }
                )))?;
            // Render custom mode stuff.
            if selected == selection_len - 1 {
                let stats_strs = [
                    format!(
                        "| Initial fall delay: {:?}s | gravity: {}",
                        self.settings
                            .new_game
                            .custom_fall_delay_params
                            .base_delay()
                            .as_secs_ennf64()
                            .get(),
                        fmt_hertz(
                            self.settings
                                .new_game
                                .custom_fall_delay_params
                                .base_delay()
                                .as_hertz()
                        ),
                    ),
                    format!(
                        "| Increasing gravity: {}",
                        !self
                            .settings
                            .new_game
                            .custom_fall_delay_params
                            .is_constant()
                    ),
                    format!(
                        "| Limit: {:?} [→]",
                        self.settings.new_game.custom_win_condition
                    ),
                ];
                for (j, stat_str) in stats_strs.into_iter().enumerate() {
                    self.term
                        .queue(MoveTo(
                            x_main + 25 + 4 * u16::try_from(j).unwrap(),
                            y_main
                                + y_selection
                                + 4
                                + u16::try_from(2 + j + selection_len + savepoint_available)
                                    .unwrap(),
                        ))?
                        .queue(Print(if j + 1 == customization_selected {
                            format!(
                                ">{stat_str}{}",
                                if customization_selected != 3
                                    || self.settings.new_game.custom_win_condition.is_some()
                                {
                                    " [↓|↑]"
                                } else {
                                    ""
                                }
                            )
                        } else {
                            stat_str
                        }))?;
                }
            }
            self.term.flush()?;
            // Wait for new input.
            let mut immediately_start_new_game = false;
            match event::read()? {
                // Quit app.
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

                // Exit menu.
                Event::Key(KeyEvent {
                    code:
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace | KeyCode::Char('b'),
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                // Try select mode.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e'),
                    kind: Press,
                    ..
                }) => {
                    immediately_start_new_game = true;
                }

                // Move selector up or increase stat.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => {
                    if customization_selected > 0 {
                        match customization_selected {
                            1 => {
                                // Increase custom fall delay.
                                let base_delay =
                                    self.settings.new_game.custom_fall_delay_params.base_delay();
                                let new_base_delay = if modifiers.contains(KeyModifiers::SHIFT) {
                                    base_delay.mul_ennf64(mult_fall_delay)
                                } else {
                                    base_delay + d_fall_delay
                                };
                                let lowerbound =
                                    self.settings.new_game.custom_fall_delay_params.lowerbound();
                                self.settings.new_game.custom_fall_delay_params = self
                                    .settings
                                    .new_game
                                    .custom_fall_delay_params
                                    .with_bounds(new_base_delay, lowerbound)
                                    .unwrap();
                            }
                            2 => {
                                // Toggle increasing fall delay.
                                let (new_factor, new_subtrahend) = if self
                                    .settings
                                    .new_game
                                    .custom_fall_delay_params
                                    .is_constant()
                                {
                                    let c = DelayParameters::constant(Default::default());
                                    (c.factor(), c.subtrahend())
                                } else {
                                    let d = DelayParameters::default_fall();
                                    (d.factor(), d.subtrahend())
                                };
                                self.settings.new_game.custom_fall_delay_params = self
                                    .settings
                                    .new_game
                                    .custom_fall_delay_params
                                    .with_coefficients(new_factor, new_subtrahend)
                                    .unwrap();
                            }
                            3 => {
                                match self.settings.new_game.custom_win_condition {
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
                    code: KeyCode::Down | KeyCode::Char('j'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => {
                    // Selected custom stat; decrease it.
                    if customization_selected > 0 {
                        match customization_selected {
                            1 => {
                                // Increase custom fall delay.
                                let base_delay =
                                    self.settings.new_game.custom_fall_delay_params.base_delay();
                                let new_base_delay = if modifiers.contains(KeyModifiers::SHIFT) {
                                    base_delay.div_ennf64(mult_fall_delay)
                                } else {
                                    base_delay.saturating_sub(d_fall_delay)
                                };
                                let lowerbound =
                                    self.settings.new_game.custom_fall_delay_params.lowerbound();
                                self.settings.new_game.custom_fall_delay_params = self
                                    .settings
                                    .new_game
                                    .custom_fall_delay_params
                                    .with_bounds(new_base_delay, lowerbound)
                                    .unwrap();
                            }
                            2 => {
                                // Toggle increasing fall delay.
                                let (new_factor, new_subtrahend) = if self
                                    .settings
                                    .new_game
                                    .custom_fall_delay_params
                                    .is_constant()
                                {
                                    let c = DelayParameters::constant(Default::default());
                                    (c.factor(), c.subtrahend())
                                } else {
                                    let d = DelayParameters::default_fall();
                                    (d.factor(), d.subtrahend())
                                };
                                self.settings.new_game.custom_fall_delay_params = self
                                    .settings
                                    .new_game
                                    .custom_fall_delay_params
                                    .with_coefficients(new_factor, new_subtrahend)
                                    .unwrap();
                            }
                            3 => {
                                match self.settings.new_game.custom_win_condition {
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
                    code: KeyCode::Left | KeyCode::Char('h'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => {
                    if selected == selection_len - 1 && customization_selected > 0 {
                        customization_selected += customization_selection_size - 1
                    } else if selected == 5 {
                        if let Some(limit) = self.settings.new_game.cheese_linelimit {
                            self.settings.new_game.cheese_linelimit =
                                NonZeroU32::try_from(limit.get() - 1).ok();
                        }
                    } else if selected == 6 {
                        if let Some(limit) = self.settings.new_game.combo_linelimit {
                            self.settings.new_game.combo_linelimit =
                                NonZeroU32::try_from(limit.get() - 1).ok();
                        }
                    } else if let Some((_game_meta_data, game_restoration_data, load_offset)) =
                        &mut self.game_savepoint
                    {
                        if selected == selection_len - 2 {
                            *load_offset += game_restoration_data.input_history.0.len()
                                * if modifiers.contains(KeyModifiers::SHIFT) {
                                    20
                                } else {
                                    1
                                };
                            *load_offset %= game_restoration_data.input_history.0.len() + 1;
                        }
                    }
                }

                // Move selector right (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    modifiers,
                    ..
                }) => {
                    // If custom gamemode selected, allow incrementing stat selection.
                    if selected == selection_len - 1 {
                        // If reached last stat, cycle through stats for limit.
                        if customization_selected == customization_selection_size - 1 {
                            self.settings.new_game.custom_win_condition =
                                match self.settings.new_game.custom_win_condition {
                                    Some(Stat::TimeElapsed(_)) => Some(Stat::PointsScored(200)),
                                    Some(Stat::PointsScored(_)) => Some(Stat::PiecesLocked(100)),
                                    Some(Stat::PiecesLocked(_)) => Some(Stat::LinesCleared(40)),
                                    Some(Stat::LinesCleared(_)) => None,
                                    None => Some(Stat::TimeElapsed(Duration::from_secs(180))),
                                };
                        } else {
                            customization_selected += 1
                        }
                    } else if selected == 5 {
                        self.settings.new_game.cheese_linelimit =
                            if let Some(limit) = self.settings.new_game.cheese_linelimit {
                                limit.checked_add(1)
                            } else {
                                Some(NonZeroU32::MIN)
                            };
                    } else if selected == 6 {
                        self.settings.new_game.combo_linelimit =
                            if let Some(limit) = self.settings.new_game.combo_linelimit {
                                limit.checked_add(1)
                            } else {
                                Some(NonZeroU32::MIN)
                            };
                    } else if let Some((_game_meta_data, game_restoration_data, load_offset)) =
                        &mut self.game_savepoint
                    {
                        if selected == selection_len - 2 {
                            *load_offset += if modifiers.contains(KeyModifiers::SHIFT) {
                                20
                            } else {
                                1
                            };
                            *load_offset %= game_restoration_data.input_history.0.len() + 1;
                        }
                    }
                }

                // Move selector right (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == selection_len - 1 {
                        self.settings.new_game.custom_seed = None;
                        self.settings.new_game.custom_board = None;
                        self.settings.new_game.custom_fall_delay_params =
                            DelayParameters::default_fall();
                        self.settings.new_game.custom_win_condition = None;
                    } else if selected == 6 {
                        let new_layout_idx = if let Some(i) = COMBO_STARTLAYOUTS
                            .iter()
                            .position(|lay| *lay == self.settings.new_game.combo_startlayout)
                        {
                            let layout_cnt = COMBO_STARTLAYOUTS.len();
                            (i + 1) % layout_cnt
                        } else {
                            0
                        };
                        self.settings.new_game.combo_startlayout =
                            COMBO_STARTLAYOUTS[new_layout_idx];
                    } else if selected == selection_len - 2 {
                        self.game_savepoint = None;
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char(c @ '0'..='9'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    let n = c.to_string().parse::<usize>().unwrap();
                    if n <= selection_len {
                        selected = if n == 0 { 10 - 1 } else { n - 1 };
                        immediately_start_new_game = true;
                    }
                }

                // Other event: don't care.
                _ => {}
            }

            if immediately_start_new_game {
                let GameplaySettings {
                    rotation_system,
                    tetromino_generator,
                    piece_preview_count,
                    delayed_auto_shift,
                    auto_repeat_rate,
                    soft_drop_factor,
                    line_clear_duration,
                    spawn_delay,
                    allow_prespawn_actions,
                } = self.settings.gameplay().clone();
                let mut builder = Game::builder();
                builder
                    .rotation_system(rotation_system)
                    .tetromino_generator(tetromino_generator)
                    .piece_preview_count(piece_preview_count)
                    .delayed_auto_shift(delayed_auto_shift)
                    .auto_repeat_rate(auto_repeat_rate)
                    .soft_drop_divisor(soft_drop_factor)
                    .line_clear_duration(line_clear_duration)
                    .spawn_delay(spawn_delay)
                    .allow_prespawn_actions(allow_prespawn_actions);
                // Build one of the selected game modes.
                let (meta_data, game, button_input_history) = if selected < game_presets.len() {
                    let ((title, comparison_stat, build), _desc) = &game_presets[selected];
                    let new_game = build(&builder);
                    let new_meta_data = GameMetaData {
                        datetime: chrono::Utc::now().format("%Y-%m-%d_%H:%M").to_string(),
                        title: title.to_owned(),
                        comparison_stat: comparison_stat.to_owned(),
                    };
                    let new_input_history = ButtonInputHistory::default();
                    (new_meta_data, new_game, new_input_history)
                // Load saved game.
                } else if selected == selection_len - 2 {
                    let (game_meta_data, game_restoration_data, load_offset) =
                        &self.game_savepoint.as_ref().unwrap();
                    let restored_game = game_restoration_data.restore(*load_offset);
                    let mut restored_meta_data = game_meta_data.clone();
                    // Mark restored game as such.
                    restored_meta_data.title.push('\'');
                    let restored_input_history = game_restoration_data.input_history.clone();
                    (restored_meta_data, restored_game, restored_input_history)
                // Build custom game.
                } else {
                    let n = &self.settings.new_game;
                    builder
                        .fall_delay_params(n.custom_fall_delay_params)
                        .end_conditions(match n.custom_win_condition {
                            Some(stat) => vec![(stat, true)],
                            None => vec![],
                        });
                    // Optionally load custom seed.
                    if let Some(seed) = n.custom_seed {
                        builder.seed(seed);
                    }
                    // Optionally load custom board.
                    let new_custom_game = if let Some(board) = &n.custom_board {
                        builder.build_modded([
                            game_mode_presets::game_modifiers::custom_start_board::modifier(board),
                        ])
                    // Otherwise just build a normal custom game.
                    } else {
                        builder.build()
                    };
                    let new_meta_data = GameMetaData {
                        datetime: chrono::Utc::now().format("%Y-%m-%d_%H:%M").to_string(),
                        title: "Custom".to_owned(),
                        comparison_stat: (Stat::PointsScored(0), false),
                    };
                    let new_input_history = ButtonInputHistory::default();
                    (new_meta_data, new_custom_game, new_input_history)
                };
                // FIXME: Remove or implement as feature/toggle.
                // let mut game = game;
                // game.modifiers.push(game_mode_presets::game_modifiers::print_fall_delay::modifier());
                // game.modifiers.push(game_mode_presets::game_modifiers::misc_modifiers::print_recency_tet_gen_stats::modifier());
                // game.modifiers.push(tetrs_engine::Modifier { descriptor: "always_clear_board".to_owned(), mod_function: Box::new(|_c, _i, s, _m, _f| { s.board = Default::default(); })});
                let now = Instant::now();
                let time_started = now - game.state().time;
                break Ok(MenuUpdate::Push(Menu::Game {
                    game: Box::new(game),
                    meta_data,
                    time_started,
                    last_paused: now,
                    total_pause_duration: Duration::ZERO,
                    button_input_history,
                    game_renderer: Default::default(),
                }));
            }
        }
    }
}
