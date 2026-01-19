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
use tetrs_engine::{Game, GameBuilder, Rules, Stat};

use crate::{
    application::{Application, GameMetaData, Menu, MenuUpdate, RecordedUserInput},
    game_modifiers,
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn menu_new_game(&mut self) -> io::Result<MenuUpdate> {
        let mut selected = 0usize;
        let mut customization_selected = 0usize;
        let (d_time, d_score, d_pieces, d_lines, d_gravity) =
            (Duration::from_secs(5), 100, 1, 1, 1);
        loop {
            #[allow(clippy::type_complexity)]
            let mut game_presets: Vec<(
                String,
                (Stat, bool),
                String,
                Box<dyn Fn(&GameBuilder) -> Game>,
            )> = vec![
                (
                    "40-Lines".to_owned(),
                    (Stat::TimeElapsed(Duration::ZERO), true),
                    "How fast can you clear forty lines?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::forty_lines()).build()
                    }),
                ),
                (
                    "Marathon".to_owned(),
                    (Stat::PointsScored(0), false),
                    "Can you make it to level 15?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::marathon()).build()
                    }),
                ),
                (
                    "Time Trial".to_owned(),
                    (Stat::PointsScored(0), false),
                    "What highscore can you get in 3 minutes?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::time_trial()).build()
                    }),
                ),
                (
                    "Master".to_owned(),
                    (Stat::PointsScored(0), false),
                    "Can you clear 15 levels at instant gravity?".to_owned(),
                    Box::new(|builder: &GameBuilder| {
                        builder.clone().rules(Rules::master()).build()
                    }),
                ),
                (
                    "Puzzle".to_owned(),
                    (Stat::TimeElapsed(Duration::ZERO), true),
                    "Get perfect clears in all 24 puzzle levels.".to_owned(),
                    Box::new(game_modifiers::puzzle::build),
                ),
                (
                    format!(
                        "{}Cheese",
                        if let Some(limit) = self.new_game_settings.cheese_linelimit {
                            format!("{limit}-")
                        } else {
                            "".to_owned()
                        }
                    ),
                    (Stat::PiecesLocked(0), true),
                    format!(
                        "Eat through lines like Swiss cheese. Limit: {:?}",
                        self.new_game_settings.cheese_linelimit
                    ),
                    Box::new({
                        let cheese_limit = self.new_game_settings.cheese_linelimit;
                        let cheese_gap_size = self.new_game_settings.cheese_gapsize;
                        let cheese_gravity = self.new_game_settings.cheese_gravity;
                        move |builder: &GameBuilder| {
                            game_modifiers::cheese::build(
                                builder,
                                cheese_limit,
                                cheese_gap_size,
                                cheese_gravity,
                            )
                        }
                    }),
                ),
                (
                    format!(
                        "{}Combo",
                        if let Some(limit) = self.new_game_settings.combo_linelimit {
                            format!("{limit}-")
                        } else {
                            "".to_owned()
                        }
                    ),
                    (Stat::TimeElapsed(Duration::ZERO), true),
                    format!(
                        "Get consecutive line clears. Limit: {:?}{}",
                        self.new_game_settings.combo_linelimit,
                        if self.new_game_settings.combo_startlayout
                            != game_modifiers::combo_board::LAYOUTS[0]
                        {
                            format!(", Layout={:b}", self.new_game_settings.combo_startlayout)
                        } else {
                            "".to_owned()
                        }
                    ),
                    Box::new({
                        let linelimit = self.new_game_settings.combo_linelimit;
                        let start_layout = self.new_game_settings.combo_startlayout;
                        move |builder: &GameBuilder| {
                            let end_conditions = match linelimit {
                                Some(c) => vec![(Stat::LinesCleared(c.get()), true)],
                                None => vec![],
                            };
                            let rules = Rules {
                                initial_gravity: 1,
                                progressive_gravity: false,
                                end_conditions,
                            };
                            let combo_board = game_modifiers::combo_board::modifier(start_layout);
                            builder.clone().rules(rules).build_modified([combo_board])
                        }
                    }),
                ),
            ];
            if self.new_game_settings.experimental_mode_unlocked {
                game_presets.push((
                    "Descent (experimental)".to_owned(),
                    (Stat::PointsScored(0), false),
                    "Spin the piece and collect 'gems' by touching them.".to_owned(),
                    Box::new(game_modifiers::descent::build),
                ))
            }
            // First part: rendering the menu.
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            let savepoint_available = if self.savepoint.is_some() { 1 } else { 0 };
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
            for (i, (title, _cmp_stat, description, _build)) in game_presets.iter().enumerate() {
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
            if let Some(sp) = &self.savepoint {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(game_presets.len() + 2).unwrap(),
                    ))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        if selected == selection_len - 2 {
                            format!(">> Load \"{}\" from {} [Del] <<", sp.0.title, sp.0.datetime)
                        } else {
                            format!("Load savepoint... ({})", sp.0.title)
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
                                if self.new_game_settings.custom_seed.is_some() {
                                    " seed"
                                } else {
                                    ""
                                },
                                if self.new_game_settings.custom_board.is_some() {
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
                        self.new_game_settings.custom_rules.initial_gravity
                    ),
                    format!(
                        "| Auto-increase gravity: {}",
                        self.new_game_settings.custom_rules.progressive_gravity
                    ),
                    format!(
                        "| Limit: {:?} [→]",
                        self.new_game_settings.custom_rules.end_conditions.first()
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
                                    || !self
                                        .new_game_settings
                                        .custom_rules
                                        .end_conditions
                                        .is_empty()
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
                                self.new_game_settings.custom_rules.initial_gravity += d_gravity;
                            }
                            2 => {
                                self.new_game_settings.custom_rules.progressive_gravity ^= true;
                            }
                            3 => {
                                match self
                                    .new_game_settings
                                    .custom_rules
                                    .end_conditions
                                    .first_mut()
                                {
                                    Some((Stat::TimeElapsed(ref mut t), _)) => {
                                        *t += d_time;
                                    }
                                    Some((Stat::PiecesLocked(ref mut p), _)) => {
                                        *p += d_pieces;
                                    }
                                    Some((Stat::LinesCleared(ref mut l), _)) => {
                                        *l += d_lines;
                                    }
                                    Some((Stat::GravityReached(ref mut g), _)) => {
                                        *g += d_gravity;
                                    }
                                    Some((Stat::PointsScored(ref mut s), _)) => {
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
                                let r = &mut self.new_game_settings.custom_rules.initial_gravity;
                                *r = r.saturating_sub(d_gravity);
                            }
                            2 => {
                                self.new_game_settings.custom_rules.progressive_gravity ^= true;
                            }
                            3 => {
                                match self
                                    .new_game_settings
                                    .custom_rules
                                    .end_conditions
                                    .first_mut()
                                {
                                    Some((Stat::TimeElapsed(ref mut t), _)) => {
                                        *t = t.saturating_sub(d_time);
                                    }
                                    Some((Stat::PiecesLocked(ref mut p), _)) => {
                                        *p = p.saturating_sub(d_pieces);
                                    }
                                    Some((Stat::LinesCleared(ref mut l), _)) => {
                                        *l = l.saturating_sub(d_lines);
                                    }
                                    Some((Stat::GravityReached(ref mut g), _)) => {
                                        *g = g.saturating_sub(d_gravity);
                                    }
                                    Some((Stat::PointsScored(ref mut s), _)) => {
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
                    ..
                }) => {
                    if selected == selection_len - 1 && customization_selected > 0 {
                        customization_selected += customization_selection_size - 1
                    } else if selected == 5 {
                        if let Some(limit) = self.new_game_settings.cheese_linelimit {
                            self.new_game_settings.cheese_linelimit =
                                NonZeroUsize::try_from(limit.get() - 1).ok();
                        }
                    } else if selected == 6 {
                        if let Some(limit) = self.new_game_settings.combo_linelimit {
                            self.new_game_settings.combo_linelimit =
                                NonZeroUsize::try_from(limit.get() - 1).ok();
                        }
                    }
                }

                // Move selector right (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    // If custom gamemode selected, allow incrementing stat selection.
                    if selected == selection_len - 1 {
                        // If reached last stat, cycle through stats for limit.
                        if customization_selected == customization_selection_size - 1 {
                            self.new_game_settings.custom_rules.end_conditions =
                                match self.new_game_settings.custom_rules.end_conditions.first() {
                                    Some((Stat::TimeElapsed(_), _)) => {
                                        vec![(Stat::PointsScored(9000), true)]
                                    }
                                    Some((Stat::PointsScored(_), _)) => {
                                        vec![(Stat::PiecesLocked(100), true)]
                                    }
                                    Some((Stat::PiecesLocked(_), _)) => {
                                        vec![(Stat::LinesCleared(40), true)]
                                    }
                                    Some((Stat::LinesCleared(_), _)) => {
                                        vec![(Stat::GravityReached(20), true)]
                                    }
                                    Some((Stat::GravityReached(_), _)) => vec![],
                                    None => {
                                        vec![(Stat::TimeElapsed(Duration::from_secs(180)), true)]
                                    }
                                };
                        } else {
                            customization_selected += 1
                        }
                    } else if selected == 5 {
                        self.new_game_settings.cheese_linelimit =
                            if let Some(limit) = self.new_game_settings.cheese_linelimit {
                                limit.checked_add(1)
                            } else {
                                Some(NonZeroUsize::MIN)
                            };
                    } else if selected == 6 {
                        self.new_game_settings.combo_linelimit =
                            if let Some(limit) = self.new_game_settings.combo_linelimit {
                                limit.checked_add(1)
                            } else {
                                Some(NonZeroUsize::MIN)
                            };
                    }
                }

                // Move selector right (select stat).
                Event::Key(KeyEvent {
                    code: KeyCode::Delete | KeyCode::Char('d'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if selected == selection_len - 1 {
                        self.new_game_settings.custom_seed = None;
                        self.new_game_settings.custom_board = None;
                        self.new_game_settings.custom_rules = Rules::default();
                    } else if selected == selection_len - 2 {
                        self.savepoint = None;
                    } else if selected == 6 {
                        let new_layout_idx = if let Some(i) = game_modifiers::combo_board::LAYOUTS
                            .iter()
                            .position(|lay| *lay == self.new_game_settings.combo_startlayout)
                        {
                            let layout_cnt = game_modifiers::combo_board::LAYOUTS.len();
                            (i + 1) % layout_cnt
                        } else {
                            0
                        };
                        self.new_game_settings.combo_startlayout =
                            game_modifiers::combo_board::LAYOUTS[new_layout_idx];
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
                // Build one of the selected game modes.
                let (game, meta_data, recorded_user_input) = if selected < game_presets.len() {
                    let (title, comparison_stat, _desc, build) = &game_presets[selected];
                    let builder = Game::builder().config(self.settings.config().clone());
                    let preset_game = build(&builder);
                    let new_meta_data = GameMetaData {
                        datetime: chrono::Utc::now().format("%Y-%m-%d_%H:%M").to_string(),
                        title: title.to_owned(),
                        comparison_stat: comparison_stat.to_owned(),
                    };
                    let new_recorded_user_input = RecordedUserInput::new();
                    (preset_game, new_meta_data, new_recorded_user_input)
                // Load saved game.
                } else if selected == selection_len - 2 {
                    let (game_meta_data, game_restoration_data) = &self.savepoint.as_ref().unwrap();
                    let restored_game = game_restoration_data.restore();
                    let mut restored_meta_data = game_meta_data.clone();
                    restored_meta_data.title.push('\'');
                    let restored_recorded_user_input =
                        game_restoration_data.recorded_user_input.clone();
                    (
                        restored_game,
                        restored_meta_data,
                        restored_recorded_user_input,
                    )
                // Build custom game.
                } else {
                    let mut builder = Game::builder()
                        .config(self.settings.config().clone())
                        .rules(self.new_game_settings.custom_rules.clone());
                    // Optionally load custom seed.
                    if self.new_game_settings.custom_seed.is_some() {
                        builder.seed = self.new_game_settings.custom_seed;
                    }
                    // Optionally load custom board.
                    let custom_game = if let Some(board) = &self.new_game_settings.custom_board {
                        builder.build_modified([
                            game_modifiers::misc::custom_start_board::modifier(board),
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
                    let new_recorded_user_input = RecordedUserInput::new();
                    (custom_game, new_meta_data, new_recorded_user_input)
                };
                let now = Instant::now();
                let time_started = now - game.state().time;
                break Ok(MenuUpdate::Push(Menu::Game {
                    game: Box::new(game),
                    meta_data,
                    time_started,
                    last_paused: now,
                    total_pause_duration: Duration::ZERO,
                    recorded_user_input,
                    game_renderer: Default::default(),
                }));
            }
        }
    }
}
