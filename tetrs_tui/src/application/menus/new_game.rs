use std::{
    io::{self, Write},
    num::NonZeroUsize,
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
use tetrs_engine::{Game, Stat};

use crate::{
    application::{Application, ButtonInputs, GameMetaData, GameRestorationData, Menu, MenuUpdate},
    fmt_utils::{fmt_button_change, fmt_duration},
    game_mode_presets::{self, mods::combo_board::LAYOUTS as COMBO_STARTLAYOUTS, GameModePreset},
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_new_game(&mut self) -> io::Result<MenuUpdate> {
        let mut selected = 0usize;
        let mut customization_selected = 0usize;
        let (d_time, d_score, d_pieces, d_lines, d_gravity) =
            (Duration::from_secs(5), 100, 1, 1, 1);
        loop {
            #[allow(clippy::type_complexity)]
            let mut game_presets: Vec<(GameModePreset, String)> = vec![
                (
                    game_mode_presets::forty_lines(),
                    "How fast can you clear forty lines?".to_owned(),
                ),
                (
                    game_mode_presets::marathon(),
                    "Can you make it to level 16?".to_owned(),
                ),
                (
                    game_mode_presets::time_trial(),
                    "What highscore can you get in 3 minutes?".to_owned(),
                ),
                (
                    game_mode_presets::master(),
                    "Can you clear 15 levels at instant gravity?".to_owned(),
                ),
                (
                    game_mode_presets::puzzle(),
                    "Get perfect clears in all 24 puzzle levels.".to_owned(),
                ),
                (
                    game_mode_presets::n_cheese(
                        self.settings.new_game.cheese_linelimit,
                        self.settings.new_game.cheese_gapsize,
                        self.settings.new_game.cheese_gravity,
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
                    "(Experimental; needs 180° rot.) per aspera ad astra".to_owned(),
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
            if let Some((game_meta_data, GameRestorationData { button_inputs, .. }, load_offset)) =
                &self.game_savepoint
            {
                let load_title = &game_meta_data.title;
                let load_offset_max = button_inputs.0.len();
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
                                let (load_time, load_input) = ButtonInputs::decode(
                                    button_inputs.0[(load_offset - 1) % button_inputs.0.len()]);
                                let load_time = fmt_duration(&load_time);
                                let load_input = fmt_button_change(&load_input);
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
                        "| Initial gravity: {}",
                        self.settings.new_game.custom_initial_gravity
                    ),
                    format!(
                        "| Increasing gravity: {}",
                        self.settings.new_game.custom_progressive_gravity
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
                    code: KeyCode::Esc | KeyCode::Char('q'),
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
                    ..
                }) => {
                    if customization_selected > 0 {
                        match customization_selected {
                            1 => {
                                self.settings.new_game.custom_initial_gravity += d_gravity;
                            }
                            2 => {
                                self.settings.new_game.custom_progressive_gravity ^= true;
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
                                    Some(Stat::GravityReached(ref mut g)) => {
                                        *g += d_gravity;
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
                    ..
                }) => {
                    // Selected custom stat; decrease it.
                    if customization_selected > 0 {
                        match customization_selected {
                            1 => {
                                let r = &mut self.settings.new_game.custom_initial_gravity;
                                *r = r.saturating_sub(d_gravity);
                            }
                            2 => {
                                self.settings.new_game.custom_progressive_gravity ^= true;
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
                                    Some(Stat::GravityReached(ref mut g)) => {
                                        *g = g.saturating_sub(d_gravity);
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
                                NonZeroUsize::try_from(limit.get() - 1).ok();
                        }
                    } else if selected == 6 {
                        if let Some(limit) = self.settings.new_game.combo_linelimit {
                            self.settings.new_game.combo_linelimit =
                                NonZeroUsize::try_from(limit.get() - 1).ok();
                        }
                    } else if let Some((_game_meta_data, game_restoration_data, load_offset)) =
                        &mut self.game_savepoint
                    {
                        if selected == selection_len - 2 {
                            *load_offset += game_restoration_data.button_inputs.0.len()
                                * if modifiers.contains(KeyModifiers::SHIFT) {
                                    20
                                } else {
                                    1
                                };
                            *load_offset %= game_restoration_data.button_inputs.0.len() + 1;
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
                                    Some(Stat::TimeElapsed(_)) => Some(Stat::PointsScored(9000)),
                                    Some(Stat::PointsScored(_)) => Some(Stat::PiecesLocked(100)),
                                    Some(Stat::PiecesLocked(_)) => Some(Stat::LinesCleared(40)),
                                    Some(Stat::LinesCleared(_)) => Some(Stat::GravityReached(20)),
                                    Some(Stat::GravityReached(_)) => None,
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
                                Some(NonZeroUsize::MIN)
                            };
                    } else if selected == 6 {
                        self.settings.new_game.combo_linelimit =
                            if let Some(limit) = self.settings.new_game.combo_linelimit {
                                limit.checked_add(1)
                            } else {
                                Some(NonZeroUsize::MIN)
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
                            *load_offset %= game_restoration_data.button_inputs.0.len() + 1;
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
                        self.settings.new_game.custom_initial_gravity = 1;
                        self.settings.new_game.custom_progressive_gravity = true;
                        self.settings.new_game.custom_win_condition = None;
                    } else if selected == selection_len - 2 {
                        self.game_savepoint = None;
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
                let g = self.settings.gameplay();
                let mut builder = Game::builder();
                builder
                    .rotation_system(g.rotation_system)
                    .start_generator(g.tetromino_generator.clone())
                    .piece_preview_size(g.piece_preview_size)
                    .allow_prespawn_actions(g.allow_prespawn_actions)
                    .delayed_auto_shift(g.delayed_auto_shift)
                    .auto_repeat_rate(g.auto_repeat_rate)
                    .soft_drop_factor(g.soft_drop_factor)
                    .line_clear_delay(g.line_clear_delay)
                    .appearance_delay(g.appearance_delay);
                // Build one of the selected game modes.
                let (meta_data, game, button_inputs) = if selected < game_presets.len() {
                    let ((title, comparison_stat, build), _desc) = &game_presets[selected];
                    let preset_game = build(&builder);
                    let new_meta_data = GameMetaData {
                        datetime: chrono::Utc::now().format("%Y-%m-%d_%H:%M").to_string(),
                        title: title.to_owned(),
                        comparison_stat: comparison_stat.to_owned(),
                    };
                    let no_previous_button_inputs = ButtonInputs::default();
                    (new_meta_data, preset_game, no_previous_button_inputs)
                // Load saved game.
                } else if selected == selection_len - 2 {
                    let (game_meta_data, game_restoration_data, load_offset) =
                        &self.game_savepoint.as_ref().unwrap();
                    let restored_game = game_restoration_data.restore(*load_offset);
                    let mut restored_meta_data = game_meta_data.clone();
                    // Mark restored game as such.
                    restored_meta_data.title.push('\'');
                    let button_inputs = game_restoration_data.button_inputs.clone();
                    (restored_meta_data, restored_game, button_inputs)
                // Build custom game.
                } else {
                    let n = &self.settings.new_game;
                    builder
                        .initial_gravity(n.custom_initial_gravity)
                        .progressive_gravity(n.custom_progressive_gravity)
                        .end_conditions(match n.custom_win_condition {
                            Some(stat) => vec![(stat, true)],
                            None => vec![],
                        });
                    // Optionally load custom seed.
                    if n.custom_seed.is_some() {
                        builder.seed = n.custom_seed;
                    }
                    // Optionally load custom board.
                    let custom_game = if let Some(board) = &n.custom_board {
                        builder.build_modded([
                            game_mode_presets::mods::miscellany::custom_start_board::modifier(
                                board,
                            ),
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
                    let no_previous_button_inputs = ButtonInputs::default();
                    (new_meta_data, custom_game, no_previous_button_inputs)
                };
                // FIXME: Remove or implement as feature/toggle.
                // game.modifiers_mut()
                //     .push(game_modifiers::misc_modifiers::print_recency_tet_gen_stats::modifier());
                // game.modifiers_mut().push(tetrs_engine::Modifier { descriptor: "always_clear_board".to_owned(), mod_function: Box::new(|_c, _i, s, _m, _f| {
                //     s.board = Default::default();
                // })});
                let now = Instant::now();
                let time_started = now - game.state().time;
                break Ok(MenuUpdate::Push(Menu::Game {
                    game: Box::new(game),
                    meta_data,
                    time_started,
                    last_paused: now,
                    total_pause_duration: Duration::ZERO,
                    recorded_button_inputs: button_inputs,
                    game_renderer: Default::default(),
                }));
            }
        }
    }
}
