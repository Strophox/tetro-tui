use std::{
    io::{self, Write},
    sync::mpsc,
    time::{Duration, Instant},
};

use crate::tetromino_engine::{
    Button, Game, GameEndCause, Input, Notification, Phase, UpdateGameError,
};
use crossterm::{
    ExecutableCommand,
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::Print,
    terminal::{self, Clear},
};

use crate::{
    Application, EncodedInputHistory, GameMetaData, GameRestorationData, GameSave, RawInputHistory,
    ScoreSummary, Statistics,
    fmt_helpers::{fmt_button_keybinds, increment_game_mode_derivative},
    game_renderers::{Renderer, TetroTUIRenderer, calc_game_keybinds_legend},
    game_restoration::{InputHistoryEncoder, QuantizeInGameTime},
    tui_menus::{Menu, MenuUpdate},
};

impl<T: Write> Application<T> {
    pub fn run_menu_play_game(
        &mut self,
        game: &mut Game,
        raw_input_history: &mut RawInputHistory,
        game_meta_data: &mut GameMetaData,
        game_renderer: &mut TetroTUIRenderer,
        selection_id_for_game_retry: Option<usize>,
    ) -> io::Result<MenuUpdate> {
        /* Our game loop recipe looks like this:
          * Enter 'update_and_render loop:
            - If game has ended, break loop.
            - Enter 'wait loop (budget based on 'latest refresh'):
              + Use player input to update game.
              + If budget ran out, break loop.
            - Set 'latest refresh' variable to ::now().
            - Do game.update().
              ** Note that in-game time at time of update can be determined with either
                 -- `duration elapsed IRL - duration paused`,
                 -- `in-game time before entering loop + in-game time elapsed since loop entered`.

            - Do game.render().
            - Continue 'update_and_render.
        */

        // Prepare everything to enter the game (react & render) loop.

        // Toggle on enhanced-keyboard-events.
        if self.temp_data.kitty_assumed {
            let f = Self::GAME_KEYBOARD_ENHANCEMENT_FLAGS;
            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
            let _r = self.term.execute(event::PushKeyboardEnhancementFlags(f));
        }

        // Prepare channel from which to receive terminal inputs.
        let (input_sender, input_receiver) = mpsc::channel();

        // Spawn input catcher thread.
        let is_stop_event = |event: Event| {
            let Event::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                ..
            }) = event
            else {
                return false;
            };
            matches!(code, KeyCode::Esc | KeyCode::Char('?'))
                || matches!(
                    (code, modifiers),
                    (
                        KeyCode::Char(
                            'd' | 'D' | 'c' | 'C' | 'r' | 'R' /* Reset is done by pushing entirely new game menu. */
                        ),
                        KeyModifiers::CONTROL
                    )
                )
        };
        let _thread_handle = input_catcher::spawn(input_sender, is_stop_event);

        let mut temp_statistics = Statistics::default();

        // FIXME: Might falsely lead to a teleport if the player pressed move within the time window at the beginning.
        // But we don't care much, as for 'usual' values this should not really happen.
        // Stores `(last_time_move_pressed, was_left_not_right)`.
        let mut temp_last_move = (Instant::now(), false);

        let keybinds_legend = calc_game_keybinds_legend(self.settings.keybinds());

        // FPS counter.
        let mut renders_per_second_counter = 0u32;
        let mut renders_per_second_counter_start_time = Instant::now();

        // Initial render.
        game_renderer.reset_viewport_state_with_offset_and_area(
            (0, 0),
            terminal::size().unwrap_or_default(),
        );
        self.term.execute(Clear(terminal::ClearType::All))?;
        game_renderer.render(
            &mut self.term,
            game,
            game_meta_data,
            &self.settings,
            &self.temp_data,
            &keybinds_legend,
            None,
        )?;

        // How much time passes between each refresh.
        let frame_interval = Duration::from_secs_f64(self.settings.graphics().fps.get().recip());

        // Time of the game when we enter the game loop.
        let mut ingametime_when_game_loop_entered = game.state().time;

        // The 'real-life' time at which we enter the game loop.
        let mut time_game_loop_entered = Instant::now();

        // The number of the frame. This is used to calculate the time of the next frame.
        let mut time_next_frame = time_game_loop_entered;

        // Main Game Loop

        let menu_update = 'update_and_render: loop {
            // Start new iteration of [render->input->] loop.

            if let Phase::GameEnd { cause, is_win } = game.phase() {
                self.statistics.games_ended += 1;

                // Game ended, cannot actually continue playing;
                // Convert to scoreboard entry and return appropriate game-ended menu.
                let scores_entry = ScoreSummary {
                    game_meta_data: game_meta_data.clone(),
                    is_win: *is_win,
                    end_cause: cause.clone(),
                    time: game.state().time,
                    pieces: game.state().pieces_locked,
                    lineclears: game.state().lineclears,
                    fall_delay_reached: game.state().fall_delay,
                    lock_delay_reached: game.state().lock_delay,
                    points: game.state().points,
                };

                let encoded_input_history = EncodedInputHistory::encode(raw_input_history);
                let forfeit =
                    matches!(cause, GameEndCause::Forfeit { .. }).then_some(game.state().time);

                let game_restoration_data =
                    GameRestorationData::new(game, encoded_input_history, forfeit);

                self.scores_and_replays
                    .entries
                    .insert(0, (scores_entry.clone(), Some(game_restoration_data)));

                let game_scoring = Box::new(scores_entry);

                let menu = if *is_win {
                    Menu::GameComplete { game_scoring }
                } else {
                    Menu::GameOver { game_scoring }
                };

                break 'update_and_render MenuUpdate::Push(menu);
            }

            // Calculate the time of the next render (according to frame_interval heartbeat) we can catch.
            // This means we skip renders if we missed their window anyway.
            let now = Instant::now();
            loop {
                time_next_frame += frame_interval;
                if time_next_frame < now {
                    continue;
                }
                break;
            }

            'wait: loop {
                // Compute duration left until we should stop waiting.
                let refresh_time_budget_remaining =
                    time_next_frame.saturating_duration_since(Instant::now());

                let recv_result = input_receiver.recv_timeout(refresh_time_budget_remaining);
                /* FIXME: Unused code: Reconsider tradeoff:
                The problem with the following code is a fine tradeoff between `std::mpsc::recv_timeout(rcvr, dt)` and `crossterm::event::poll(dt)`.
                The exact tradeoff is very unclear, but we trust the Rust stdlib for its slightly better performance/reliability
                in some ad-hoc testing, despite the 'direct' approach not requiring an input catcher thread.

                // Wait for poll response.
                let event_available = event::poll(refresh_time_budget_remaining)?;
                // Finished waiting with no terminal signal available.
                if !event_available {
                    break 'wait;
                }
                let timestamp = Instant::now();
                let event = event::read()?;
                */

                // Read terminal signal or finish waiting.
                let (event, timestamp) = match recv_result {
                    Err(recv_timeout_error) => match recv_timeout_error {
                        // Frame idle/budget expired on its own: leave wait loop.
                        mpsc::RecvTimeoutError::Timeout => {
                            break 'wait;
                        }

                        // Input handler thread died... Pause game for now.
                        mpsc::RecvTimeoutError::Disconnected => {
                            // FIXME: This 'extremely' rare error is currently fixed by pausing the game
                            // which means no extra work for us and just one extra step for the user.
                            // But maybe properly try restarting the thread manually?...
                            break 'update_and_render MenuUpdate::Push(Menu::Pause);
                        }
                    },
                    Ok(x) => x,
                };

                if let Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind,
                    state: _,
                }) = event
                    && let Some(mut button) =
                        self.settings.keybinds().get((code, modifiers)).copied()
                {
                    // We first calculate the intended time at time of reaching here.
                    let update_target_time = ingametime_when_game_loop_entered
                        + timestamp.saturating_duration_since(time_game_loop_entered);

                    // Guarantee update cannot fail because it lies in the game's past:
                    // Worst case react to player input as quickly as possible now.
                    let update_target_time = game.state().time.max(update_target_time);

                    // Lastly we (artificially) compress the information of when an input happened (we essentially round it).
                    let update_target_time = update_target_time.quantize();

                    if self.temp_data.kitty_assumed {
                        // Enhanced keyboard events: determinedly send a single press or release.

                        let mut player_input = (match kind {
                            KeyEventKind::Press => Input::Activate,
                            // Kitty does not actually care about terminal/OS keyboard 'repeat' events.
                            KeyEventKind::Repeat => continue 'wait,
                            KeyEventKind::Release => Input::Deactivate,
                        })(button);

                        // FIXME: We only transform `Activate`s into teleports currently,
                        // but we forget the release events (which will just be move releases,
                        // i.e. teleport will remain active).
                        // In usual games this will not lead to issues but logically unclean (also, modding behavior).
                        // We expect primary users of double-tap finesse to be non-enhanced-terminal users anyway.
                        if let Some(tap_move_delay) = self.settings.gameplay().dtapfinesse {
                            match player_input {
                                Input::Activate(Button::MoveLeft) => {
                                    if temp_last_move.1
                                        && timestamp.duration_since(temp_last_move.0)
                                            <= tap_move_delay
                                    {
                                        player_input = Input::Activate(Button::TeleLeft);
                                    }
                                    temp_last_move = (timestamp, true);
                                }
                                Input::Activate(Button::MoveRight) => {
                                    if !temp_last_move.1
                                        && timestamp.duration_since(temp_last_move.0)
                                            <= tap_move_delay
                                    {
                                        player_input = Input::Activate(Button::TeleRight);
                                    }
                                    temp_last_move = (timestamp, false);
                                }
                                _ => {}
                            }
                        }

                        raw_input_history
                            .inputs
                            .push((update_target_time, player_input));

                        match game.update(update_target_time, Some(player_input)) {
                            Ok(msgs) => {
                                temp_statistics.accumulate_from_feed(&msgs);
                                game_renderer.update_feed(msgs, &self.settings)
                            }
                            Err(UpdateGameError::AlreadyEnded) => break 'wait,
                            Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                        }
                    } else {
                        // Else: Non-kitty input handling.

                        // Special handling here for terminals that STILL send "release" events despite us assuming it's not enhanced;
                        // So we don't accidentally interpret them as presses.
                        if !matches!(kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                            continue 'wait;
                        }

                        if let Some(tap_move_delay) = self.settings.gameplay().dtapfinesse {
                            match button {
                                Button::MoveLeft => {
                                    if temp_last_move.1
                                        && timestamp.duration_since(temp_last_move.0)
                                            <= tap_move_delay
                                    {
                                        button = Button::TeleLeft;
                                    }
                                    temp_last_move = (timestamp, true);
                                }
                                Button::MoveRight => {
                                    if !temp_last_move.1
                                        && timestamp.duration_since(temp_last_move.0)
                                            <= tap_move_delay
                                    {
                                        button = Button::TeleRight;
                                    }
                                    temp_last_move = (timestamp, false);
                                }
                                _ => {}
                            }
                        }

                        // Non-enhanced terminal - since we don't have "release" events, we just assume a button press is an instantaneous sequence of press+release.
                        let input = Input::Activate(button);

                        raw_input_history.inputs.push((update_target_time, input));

                        match game.update(update_target_time, Some(input)) {
                            Ok(msgs) => {
                                temp_statistics.accumulate_from_feed(&msgs);
                                game_renderer.update_feed(msgs, &self.settings);
                            }
                            Err(UpdateGameError::AlreadyEnded) => break 'wait,
                            Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                        }

                        // Note that we do not expect a button release to actually end the game or similar, but we handle things properly anyway.
                        let input = Input::Deactivate(button);

                        raw_input_history.inputs.push((update_target_time, input));

                        let update_result = game.update(update_target_time, Some(input));

                        match update_result {
                            Ok(msgs) => {
                                temp_statistics.accumulate_from_feed(&msgs);
                                game_renderer.update_feed(msgs, &self.settings)
                            }
                            Err(UpdateGameError::AlreadyEnded) => break 'wait,
                            Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                        }
                    }
                }

                // Process input as non-tetro game input action.
                match event {
                    event::Event::Key(KeyEvent {
                        code,
                        modifiers,
                        kind,
                        state: _,
                    }) => {
                        if !matches!(kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                            // It just so happens that, once we're done considering in-game-relevant presses,
                            // for the remaining controls we don't care about releases.
                            continue 'wait;
                        }

                        match (code, modifiers) {
                            // [Esc]: Pause.
                            (KeyCode::Esc, _) => {
                                break 'update_and_render MenuUpdate::Push(Menu::Pause);
                            }

                            // [Ctrl+C]: Quit program.
                            (KeyCode::Char('c' | 'C'), KeyModifiers::CONTROL) => {
                                break 'update_and_render MenuUpdate::Push(Menu::Quit);
                            }

                            // Keybinds help menu.
                            (KeyCode::Char('?'), _) => {
                                let client_menu_name = "Live Game";
                                let legend = vec![
                                    (
                                        "Normal keybinds".to_owned(),
                                        [
                                            ("Esc", "Open Pause menu"),
                                            ("?", "Open Keybinds overview"),
                                        ]
                                        .into_iter()
                                        .map(|(lhs, rhs)| (lhs.to_owned(), rhs.to_owned()))
                                        .collect(),
                                    ),
                                    (
                                        "Special keybinds".to_owned(),
                                        [
                                            ("Ctrl+D", "Forfeit game"),
                                            (
                                                "Ctrl+R",
                                                "Restart game mode (overwrites current game!)",
                                            ),
                                            (
                                                "Ctrl+Z",
                                                "Undo last input (overwrites current game!)",
                                            ),
                                            ("Ctrl+L", "Load game save (overwrites current game!)"),
                                            ("Ctrl+S", "Store game save"),
                                            ("Ctrl+E", "Store seed for custom game"),
                                            (
                                                "Ctrl+G/Ctrl+Alt+G",
                                                "Cycle forward/backward through Graphics slots",
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
                                    (
                                        "Player keybinds".to_owned(),
                                        Button::VARIANTS
                                            .into_iter()
                                            .map(|b| {
                                                (
                                                    format!("{b:?}"),
                                                    fmt_button_keybinds(
                                                        b,
                                                        self.settings.keybinds(),
                                                        " ",
                                                    ),
                                                )
                                            })
                                            .collect(),
                                    ),
                                ];

                                break 'update_and_render MenuUpdate::Push(
                                    Menu::KeybindsOverview {
                                        client_menu_name,
                                        legend,
                                    },
                                );
                            }

                            // [Ctrl+D]: Forfeit game.
                            (KeyCode::Char('d' | 'D'), KeyModifiers::CONTROL) => {
                                match game.forfeit() {
                                    Ok(msgs) => {
                                        temp_statistics.accumulate_from_feed(&msgs);
                                        game_renderer.update_feed(msgs, &self.settings);
                                    }

                                    // We do not care if game ended or time is in past here.
                                    Err(
                                        UpdateGameError::AlreadyEnded
                                        | UpdateGameError::TargetTimeInPast,
                                    ) => {}
                                }

                                break 'wait;
                            }

                            // [Ctrl+S]: Store game save.
                            (KeyCode::Char('s' | 'S'), KeyModifiers::CONTROL) => {
                                self.game_saves.selected = 0;
                                self.game_saves.slots = vec![GameSave {
                                    game_meta_data: game_meta_data.clone(),
                                    game_restoration_data: GameRestorationData::new(
                                        game,
                                        raw_input_history.clone(),
                                        matches!(
                                            game.phase(),
                                            Phase::GameEnd {
                                                cause: GameEndCause::Forfeit { .. },
                                                ..
                                            }
                                        )
                                        .then_some(game.state().time),
                                    ),
                                    inputs_to_load: raw_input_history.inputs.len(),
                                }];

                                game_renderer.update_feed(
                                    [(
                                        Notification::Custom("(Game save stored)".to_owned()),
                                        game.state().time,
                                    )],
                                    &self.settings,
                                );
                            }

                            // [Ctrl+L]: Load game save.
                            (KeyCode::Char('l' | 'L'), KeyModifiers::CONTROL) => {
                                if let Some(GameSave {
                                    game_meta_data: saved_meta_data,
                                    game_restoration_data,
                                    inputs_to_load,
                                }) = &self.game_saves.get()
                                {
                                    *game = game_restoration_data.restore(*inputs_to_load);

                                    *game_meta_data = saved_meta_data.clone();
                                    increment_game_mode_derivative(&mut game_meta_data.title);

                                    raw_input_history.inputs = game_restoration_data
                                        .input_history
                                        .inputs
                                        .iter()
                                        .take(*inputs_to_load)
                                        .copied()
                                        .collect();

                                    game_renderer.reset_veffects_state();
                                    game_renderer.update_feed(
                                        [(
                                            Notification::Custom("(Game save loaded)".to_owned()),
                                            game.state().time,
                                        )],
                                        &self.settings,
                                    );

                                    // What we do here is rather unholy, so we have to adapt the game loop state itself.
                                    self.statistics.play_time += Instant::now()
                                        .saturating_duration_since(time_game_loop_entered);

                                    ingametime_when_game_loop_entered = game.state().time;
                                    time_game_loop_entered = Instant::now();
                                }
                            }

                            // [Ctrl+Z]: Undo last input.
                            (KeyCode::Char('z' | 'Z'), KeyModifiers::CONTROL) => {
                                // Try to pop away latest input.
                                if let Some(mut latest_timed_input) = raw_input_history.inputs.pop()
                                {
                                    // Here we pop away *additional inputs* that happen at the *exact same time*.
                                    // This is more ergonomic for the case where an unenhanced terminal does not distinguish
                                    // between 'press' and 'release' and instead emulated them with press and release happening
                                    // at the exact same time. If user really need the distinction, the should just use the precise
                                    // input history selection for game saves in the 'Start New Game' menu.
                                    while let Some(second_latest_timed_input) =
                                        raw_input_history.inputs.last()
                                    {
                                        if second_latest_timed_input.0 != latest_timed_input.0 {
                                            break;
                                        }
                                        latest_timed_input = *second_latest_timed_input;
                                        raw_input_history.inputs.pop();
                                    }

                                    // Manually restore game to previous state.
                                    let (builder, mod_ids_cfgs) = game.reproduce_builder();
                                    let game_restoration_data = GameRestorationData {
                                        builder,
                                        mod_ids_cfgs,
                                        input_history: raw_input_history.clone(),
                                        forfeit: None,
                                    };
                                    *game = game_restoration_data
                                        .restore(game_restoration_data.input_history.inputs.len());

                                    // Mark undo as such.
                                    increment_game_mode_derivative(&mut game_meta_data.title);

                                    game_renderer.reset_veffects_state();
                                    game_renderer.update_feed(
                                        [(
                                            Notification::Custom(format!(
                                                "(Undo '{}')",
                                                match latest_timed_input.1 {
                                                    Input::Activate(button) =>
                                                        format!("{button:?} press"),
                                                    Input::Deactivate(button) =>
                                                        format!("{button:?} release"),
                                                }
                                            )),
                                            game.state().time,
                                        )],
                                        &self.settings,
                                    );

                                    // What we do here is rather unholy, so we have to adapt the game loop state itself.
                                    self.statistics.play_time += Instant::now()
                                        .saturating_duration_since(time_game_loop_entered);

                                    ingametime_when_game_loop_entered = game.state().time;
                                    time_game_loop_entered = Instant::now();
                                }
                            }

                            // [Ctrl+E]: Store seed.
                            (KeyCode::Char('e' | 'E'), KeyModifiers::CONTROL) => {
                                self.settings.game_mode_preferences.custom_config.seed =
                                    Some(game.state_init().seed);

                                game_renderer.update_feed(
                                    [(
                                        Notification::Custom(format!(
                                            "(Seed stored: {})",
                                            game.state_init().seed
                                        )),
                                        game.state().time,
                                    )],
                                    &self.settings,
                                );
                            }

                            // [Ctrl+R]: Retry game.
                            (KeyCode::Char('r' | 'R'), KeyModifiers::CONTROL) => {
                                if let Some(selection) = selection_id_for_game_retry {
                                    let game_menu = self.create_game_menu(selection);
                                    break 'update_and_render MenuUpdate::Push(game_menu);
                                }
                            }

                            // [Ctrl+Alt+B]: (Un-)Blindfold.
                            (KeyCode::Char('b' | 'B'), _)
                                if {
                                    modifiers
                                        .contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT))
                                } =>
                            {
                                self.temp_data.blindfold_game ^= true;
                                if self.temp_data.blindfold_game {
                                    game_renderer.update_feed(
                                        [(
                                            Notification::Custom(
                                                "Blindfolded! [Ctrl+Alt+B]".to_owned(),
                                            ),
                                            game.state().time,
                                        )],
                                        &self.settings,
                                    );
                                } else {
                                    game_renderer.update_feed(
                                        [(
                                            Notification::Custom(
                                                "Blindfolds removed [Ctrl+Alt+B]".to_owned(),
                                            ),
                                            game.state().time,
                                        )],
                                        &self.settings,
                                    );
                                }
                            }

                            // [Ctrl(+Alt)+G]: Cycle graphics slots.
                            (KeyCode::Char('g' | 'G'), _)
                                if { modifiers.contains(KeyModifiers::CONTROL) } =>
                            {
                                self.settings.graphics_selected +=
                                    if modifiers.contains(KeyModifiers::ALT) {
                                        self.settings.graphics_slotmachine.slots.len() - 1
                                    } else {
                                        1
                                    };
                                self.settings.graphics_selected %=
                                    self.settings.graphics_slotmachine.slots.len();
                                let msg = format!(
                                    "(Graphics: {})",
                                    self.settings
                                        .graphics_slotmachine
                                        .grab(self.settings.graphics_selected)
                                        .0
                                );
                                game_renderer.update_feed(
                                    [(Notification::Custom(msg), game.state().time)],
                                    &self.settings,
                                );
                            }

                            // [Ctrl+Alt+L]: Reload app from savefile.
                            (KeyCode::Char('l' | 'L'), _)
                                if {
                                    modifiers
                                        .contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT))
                                } =>
                            {
                                let msg = if let Err(e) = self.savefile_load() {
                                    format!("(Error reloading from savefile: {e}")
                                } else {
                                    "(Reloaded from savefile.)".to_owned()
                                };
                                game_renderer.update_feed(
                                    [(Notification::Custom(msg), game.state().time)],
                                    &self.settings,
                                );
                            }

                            // [Ctrl+Alt+S]: Store to savefile.
                            (KeyCode::Char('s' | 'S'), _)
                                if {
                                    modifiers
                                        .contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT))
                                } =>
                            {
                                let msg = if let Err(e) = self.savefile_store() {
                                    format!("(Error on savefile store: {e}")
                                } else {
                                    "(Savefile store done.)".to_owned()
                                };
                                game_renderer.update_feed(
                                    [(Notification::Custom(msg), game.state().time)],
                                    &self.settings,
                                );
                            }

                            // Other misc. key event: We don't care.
                            _ => {}
                        }
                    }

                    // Auto-pause on focus lost.
                    event::Event::FocusLost => {
                        if self.temp_data.pause_on_focus_lost {
                            break 'update_and_render MenuUpdate::Push(Menu::Pause);
                        }
                    }

                    event::Event::FocusGained => {}
                    event::Event::Mouse(_) => {}
                    event::Event::Paste(_) => {}
                    event::Event::Resize(cols, rows) => {
                        // Need to redraw screen for proper centering etc.
                        game_renderer
                            .reset_viewport_state_with_offset_and_area((0, 0), (cols, rows));
                        self.term.execute(Clear(terminal::ClearType::All))?;
                        break 'wait;
                    }
                }
            }

            let now = Instant::now();

            // We first calculate the intended time at time of reaching here.
            let update_target_time = ingametime_when_game_loop_entered
                + now.saturating_duration_since(time_game_loop_entered);

            match game.update(update_target_time, None) {
                // Update.
                Ok(msgs) => {
                    temp_statistics.accumulate_from_feed(&msgs);
                    game_renderer.update_feed(msgs, &self.settings)
                }

                // We do not care if game ended or time is in past here:
                // We just care about best-effort updating state to show it to player.
                Err(UpdateGameError::AlreadyEnded | UpdateGameError::TargetTimeInPast) => {}
            }

            // Render current state of the game.
            game_renderer.render(
                &mut self.term,
                game,
                game_meta_data,
                &self.settings,
                &self.temp_data,
                &keybinds_legend,
                None,
            )?;

            renders_per_second_counter += 1;

            // Render FPS counter.
            if self.settings.graphics().show_fps {
                let secs_diff = now
                    .saturating_duration_since(renders_per_second_counter_start_time)
                    .as_secs_f64();
                const REFRESH_FPS_COUNTER_EVERY_N_SECS: f64 = 1.0;

                // One second has passed since we started counting.
                if secs_diff >= REFRESH_FPS_COUNTER_EVERY_N_SECS {
                    self.term.execute(MoveTo(0, 0))?;
                    self.term.execute(Print(format!(
                        "{:_>7}",
                        format!(
                            "{:.1}fps",
                            f64::from(renders_per_second_counter) / secs_diff
                        )
                    )))?;

                    // Reset data.
                    renders_per_second_counter = 0;
                    renders_per_second_counter_start_time = now;
                }
            }
        };

        // Game loop epilogue: De-initialization.

        if self.temp_data.kitty_assumed {
            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
            let _r = self.term.execute(event::PopKeyboardEnhancementFlags);
        }

        if !game.has_ended() {
            // Game not done:.
            // Manually release any pressed buttons to avoid weird persistent-buttonpress behavior.
            let unpress_time = game.state().time.quantize();
            'button_unpressing: for button in Button::VARIANTS {
                if game.state().active_buttons[button].is_some() {
                    let input = Input::Deactivate(button);

                    let update_result = game.update(unpress_time, Some(input));

                    raw_input_history.inputs.push((unpress_time, input));
                    match update_result {
                        Ok(msgs) => {
                            temp_statistics.accumulate_from_feed(&msgs);
                            game_renderer.update_feed(msgs, &self.settings);
                        }
                        Err(UpdateGameError::AlreadyEnded) => break 'button_unpressing,
                        Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                    }
                }
            }
        }

        self.statistics.play_time +=
            Instant::now().saturating_duration_since(time_game_loop_entered);

        if !Statistics::GAME_MODE_TITLE_PREFIX_BLACKLIST
            .iter()
            .any(|prefix| game_meta_data.title.starts_with(prefix))
        {
            self.statistics.accumulate(&temp_statistics);
        }

        Ok(menu_update)
    }
}

mod input_catcher {
    use std::{
        sync::mpsc::{SendError, Sender},
        thread::{self, JoinHandle},
        time::Instant,
    };

    use crossterm::event::{self, Event};

    pub fn spawn(
        input_sender: Sender<(Event, Instant)>,
        is_stop_event: impl Fn(Event) -> bool + Send + 'static,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            'detect_events: loop {
                // Read event.
                match event::read() {
                    Ok(event) => {
                        let timestamp = Instant::now();

                        // Send signal.
                        match input_sender.send((event.clone(), timestamp)) {
                            Ok(()) => {}
                            Err(SendError(_event_which_failed_to_transmit)) => {
                                break 'detect_events;
                            }
                        }

                        if is_stop_event(event) {
                            break 'detect_events;
                        }
                    }

                    // FIXME: Handle io::Error? If not, why not?
                    Err(_e) => {}
                }
            }
        })
    }
}
