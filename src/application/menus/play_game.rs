use std::{
    io::{self, Write},
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::Print,
    ExecutableCommand,
};
use falling_tetromino_engine::{
    Button, Game, GameEndCause, InGameTime, Input, Notification, Phase, UpdateGameError,
};

use crate::{
    application::{
        menus::{Menu, MenuUpdate},
        Application, CompressedInputHistory, GameMetaData, GameRestorationData, GameSave,
        ScoreEntry, Statistics, UncompressedInputHistory,
    },
    fmt_helpers::get_play_keybinds_legend,
    game_renderers::{Renderer, TetroTUIRenderer},
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_play_game(
        &mut self,
        game: &mut Game,
        game_input_history: &mut UncompressedInputHistory,
        game_meta_data: &mut GameMetaData,
        game_renderer: &mut TetroTUIRenderer,
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
            let _v = self.term.execute(event::PushKeyboardEnhancementFlags(f));
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
            matches!(code, KeyCode::Esc)
                || matches!(
                    (code, modifiers),
                    (KeyCode::Char('d' | 'D'), KeyModifiers::CONTROL)
                )
                || matches!(
                    (code, modifiers),
                    (KeyCode::Char('c' | 'C'), KeyModifiers::CONTROL)
                )
        };
        let _thread_handle = input_catcher::spawn(input_sender, is_stop_event);

        let mut temp_statistics = Statistics::default();

        // Stores `(last_time_move_pressed, was_left_not_right)`.
        // FIXME: Might falsely lead to a teleport if the player pressed move within the time window at the beginning.
        // But we don't care much, as for 'usual' values this should not really happen (worst case they lose a few ms in overall run).
        let mut temp_last_move = (Instant::now(), false);

        let keybinds_legend = get_play_keybinds_legend(self.settings.keybinds());

        // FPS counter.
        let mut renders_per_second_counter = 0u32;
        let mut renders_per_second_counter_start_time = Instant::now();

        // Initial render.
        let (x_main, y_main) = Application::<T>::fetch_main_xy();
        game_renderer.set_render_offset(usize::from(x_main), usize::from(y_main));
        game_renderer.reset_view_diff_state();
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
        let frame_interval = Duration::from_secs_f64(self.settings.graphics().game_fps.recip());

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
                self.statistics.total_games_ended += 1;

                // Game ended, cannot actually continue playing;
                // Convert to scoreboard entry and return appropriate game-ended menu.
                let scores_entry = ScoreEntry {
                    game_meta_data: game_meta_data.clone(),
                    is_win: *is_win,
                    end_cause: cause.clone(),
                    time_elapsed: game.state().time,
                    pieces_locked: game.state().pieces_locked,
                    lineclears: game.state().lineclears,
                    fall_delay_reached: game.state().fall_delay,
                    lock_delay_reached: (game
                        .state()
                        .fall_delay_lowerbound_hit_at_n_lineclears
                        .is_some()
                        && !game.config.lock_delay_params.is_constant())
                    .then_some(game.state().lock_delay),
                    points_scored: game.state().points,
                };

                let compressed_game_input_history = CompressedInputHistory::new(game_input_history);
                let forfeit =
                    matches!(cause, GameEndCause::Forfeit { .. }).then_some(game.state().time);

                let game_restoration_data =
                    GameRestorationData::new(game, compressed_game_input_history, forfeit);

                self.scores_and_replays
                    .entries
                    .push((scores_entry.clone(), Some(game_restoration_data)));

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
                /* FIXME: Remove unused code or reconsider.
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
                {
                    if let Some(mut button) =
                        self.settings.keybinds().get((code, modifiers)).copied()
                    {
                        // We first calculate the intended time at time of reaching here.
                        let update_target_time = ingametime_when_game_loop_entered
                            + timestamp.saturating_duration_since(time_game_loop_entered);

                        // Guarantee update cannot fail because it lies in the game's past:
                        // Worst case react to player input as quickly as possible now.
                        let update_target_time = game.state().time.max(update_target_time);

                        // Lastly we (artificially) compress the information of when an input happened:
                        // We round it to milliseconds (manually do ceiling rounding, to not be in game's past).
                        let nanos = update_target_time.as_nanos();
                        const NANOS_PER_MILLI: u128 = 1_000_000;
                        let update_target_time = InGameTime::from_millis(
                            (nanos / NANOS_PER_MILLI
                                + if nanos.is_multiple_of(NANOS_PER_MILLI) {
                                    0
                                } else {
                                    1
                                }) as u64,
                        );

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

                            game_input_history.push((update_target_time, player_input));

                            match game.update(update_target_time, Some(player_input)) {
                                Ok(msgs) => {
                                    temp_statistics.accumulate_from_feed(&msgs);
                                    game_renderer.push_game_notification_feed(msgs)
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
                            let button_change = Input::Activate(button);

                            game_input_history.push((update_target_time, button_change));

                            match game.update(update_target_time, Some(button_change)) {
                                Ok(msgs) => {
                                    temp_statistics.accumulate_from_feed(&msgs);
                                    game_renderer.push_game_notification_feed(msgs);
                                }
                                Err(UpdateGameError::AlreadyEnded) => break 'wait,
                                Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                            }

                            // Note that we do not expect a button release to actually end the game or similar, but we handle things properly anyway.
                            let button_change = Input::Deactivate(button);

                            game_input_history.push((update_target_time, button_change));

                            let update_result =
                                game.update(update_target_time, Some(button_change));

                            match update_result {
                                Ok(msgs) => {
                                    temp_statistics.accumulate_from_feed(&msgs);
                                    game_renderer.push_game_notification_feed(msgs)
                                }
                                Err(UpdateGameError::AlreadyEnded) => break 'wait,
                                Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                            }
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
                        if !matches!(kind, KeyEventKind::Press) {
                            // It just so happens that, once we're done considering in-game-relevant presses,
                            // for the remaining controls we only care about key*down*s.
                            continue 'wait;
                        }

                        match (code, modifiers) {
                            // [Esc]: Pause.
                            (KeyCode::Esc, _) => {
                                break 'update_and_render MenuUpdate::Push(Menu::Pause);
                            }

                            // [Ctrl+C]: Exit program.
                            (KeyCode::Char('c' | 'C'), KeyModifiers::CONTROL) => {
                                break 'update_and_render MenuUpdate::Push(Menu::Quit);
                            }

                            // [Ctrl+D]: Forfeit game.
                            (KeyCode::Char('d' | 'D'), KeyModifiers::CONTROL) => {
                                match game.forfeit() {
                                    Ok(msgs) => {
                                        temp_statistics.accumulate_from_feed(&msgs);
                                        game_renderer.push_game_notification_feed(msgs);
                                    }

                                    // We do not care if game ended or time is in past here.
                                    Err(
                                        UpdateGameError::AlreadyEnded
                                        | UpdateGameError::TargetTimeInPast,
                                    ) => {}
                                }

                                break 'wait;
                            }

                            // [Ctrl+S]: Store savepoint.
                            (KeyCode::Char('s' | 'S'), KeyModifiers::CONTROL) => {
                                self.game_saves = (
                                    0,
                                    vec![GameSave {
                                        game_meta_data: game_meta_data.clone(),
                                        game_restoration_data: GameRestorationData::new(
                                            game,
                                            game_input_history.clone(),
                                            matches!(
                                                game.phase(),
                                                Phase::GameEnd {
                                                    cause: GameEndCause::Forfeit { .. },
                                                    ..
                                                }
                                            )
                                            .then_some(game.state().time),
                                        ),
                                        inputs_to_load: game_input_history.len(),
                                    }],
                                );

                                game_renderer.push_game_notification_feed([(
                                    Notification::Custom("(Stored savepoint)".to_owned()),
                                    game.state().time,
                                )]);
                            }

                            // [Ctrl+L]: Load savepoint.
                            (KeyCode::Char('l' | 'L'), KeyModifiers::CONTROL) => {
                                if let Some(GameSave {
                                    game_meta_data: saved_meta_data,
                                    game_restoration_data,
                                    inputs_to_load,
                                }) = &self.game_saves.1.get(self.game_saves.0)
                                {
                                    *game = game_restoration_data.restore(*inputs_to_load);

                                    *game_meta_data = saved_meta_data.clone();
                                    // Mark restored game as such.
                                    game_meta_data.title.push('\'');

                                    *game_input_history = game_restoration_data
                                        .input_history
                                        .iter()
                                        .take(*inputs_to_load)
                                        .copied()
                                        .collect();

                                    game_renderer.reset_game_associated_state();
                                    game_renderer.push_game_notification_feed([(
                                        Notification::Custom("(Loaded savepoint)".to_owned()),
                                        game.state().time,
                                    )]);

                                    // What we do here is rather unholy, so we have to adapt the game loop state itself.
                                    self.statistics.total_play_time += Instant::now()
                                        .saturating_duration_since(time_game_loop_entered);

                                    ingametime_when_game_loop_entered = game.state().time;
                                    time_game_loop_entered = Instant::now();
                                }
                            }

                            // [Ctrl+E]: Store seed.
                            (KeyCode::Char('e' | 'E'), KeyModifiers::CONTROL) => {
                                self.settings.newgame.custom_seed = Some(game.state_init().seed);

                                game_renderer.push_game_notification_feed([(
                                    Notification::Custom(format!(
                                        "(Seed stored: {})",
                                        game.state_init().seed
                                    )),
                                    game.state().time,
                                )]);
                            }

                            // [Ctrl+Alt+B]: (Un-)Blindfold.
                            (KeyCode::Char('b' | 'B'), _)
                                if {
                                    modifiers
                                        .contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT))
                                } =>
                            {
                                self.temp_data.blindfold_enabled ^= true;
                                if self.temp_data.blindfold_enabled {
                                    game_renderer.push_game_notification_feed([(
                                        Notification::Custom(
                                            "Blindfolded! [Ctrl+Alt+B]".to_owned(),
                                        ),
                                        game.state().time,
                                    )]);
                                } else {
                                    game_renderer.push_game_notification_feed([(
                                        Notification::Custom(
                                            "Blindfolds removed [Ctrl+Alt+B]".to_owned(),
                                        ),
                                        game.state().time,
                                    )]);
                                }
                            }

                            // Other misc. key event: We don't care.
                            _ => continue 'wait,
                        }
                    }

                    event::Event::Mouse(_) => {}
                    event::Event::Paste(_) => {}
                    event::Event::FocusGained => {}
                    event::Event::FocusLost => {}
                    event::Event::Resize(_, _) => {
                        // Need to redraw screen for proper centering etc.
                        let (x_main, y_main) = Application::<T>::fetch_main_xy();
                        game_renderer.set_render_offset(usize::from(x_main), usize::from(y_main));
                        game_renderer.reset_view_diff_state();
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
                    game_renderer.push_game_notification_feed(msgs)
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

        /* Note that at this point the player will have exited the loop between two calls to `.update()`.
        For correctness, we could add the lines below, but if we don't do it the player 'just' sees
        the same frame *and* underlying game state as he last saw here, which might be even better.
        ```
            let update_target_time = Instant::now().duration_since(time_game_loop_entered);

            match game.update(update_target_time, None) {
                // Update
                Ok(msgs) => {
                    game_renderer.push_game_feedback_msgs(msgs);
                }

                // FIXME: Handle UpdateGameError? If not, why not?
                Err(_e) => {}
            }
        ``` */

        if self.temp_data.kitty_assumed {
            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
            let _v = self.term.execute(event::PopKeyboardEnhancementFlags);
        }

        if !game.has_ended() {
            // Game not done:.
            // Manually release any pressed buttons to avoid weird persistent-buttonpress behavior.
            let unpress_time = game.state().time;
            'button_unpressing: for button in Button::VARIANTS {
                if game.state().active_buttons[button].is_some() {
                    let button_change = Input::Deactivate(button);

                    let update_result = game.update(unpress_time, Some(button_change));

                    game_input_history.push((unpress_time, button_change));
                    match update_result {
                        Ok(msgs) => {
                            temp_statistics.accumulate_from_feed(&msgs);
                            game_renderer.push_game_notification_feed(msgs);
                        }
                        Err(UpdateGameError::AlreadyEnded) => break 'button_unpressing,
                        Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                    }
                }
            }
        }

        self.statistics.total_play_time +=
            Instant::now().saturating_duration_since(time_game_loop_entered);

        if !Statistics::BLACKLIST_TITLE_PREFIXES
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
