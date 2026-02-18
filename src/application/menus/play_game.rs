use std::{
    io::{self, Write},
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::Print,
    terminal::{self, Clear, ClearType},
    ExecutableCommand,
};
use falling_tetromino_engine::{
    Button, ButtonChange, Feedback, Game, GameOver, InGameTime, UpdateGameError,
};

use crate::{
    application::{
        Application, CompressedInputHistory, GameMetaData, GameRestorationData, GameSave, Menu,
        MenuUpdate, ScoresEntry, UncompressedInputHistory,
    },
    fmt_helpers::get_play_keybinds_legend,
    game_renderers::Renderer,
    live_input_handler::{self, LiveTermSignal},
};

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_play_game(
        &mut self,
        game: &mut Game,
        game_input_history: &mut UncompressedInputHistory,
        game_meta_data: &mut GameMetaData,
        game_renderer: &mut impl Renderer,
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
        if self.runtime_data.kitty_assumed {
            let f = Self::KEYBOARD_ENHANCEMENT_FLAGS;
            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
            let _v = self.term.execute(event::PushKeyboardEnhancementFlags(f));
        }

        // Prepare channel from which to receive terminal inputs.
        let (input_sender, input_receiver) = mpsc::channel();

        // Spawn input handler thread.
        let _join_handle =
            live_input_handler::spawn(input_sender, self.settings.keybinds().clone());

        let keybinds_legend = get_play_keybinds_legend(self.settings.keybinds());

        // FPS counter.
        let mut renders_per_second_counter = 0u32;
        let mut renders_per_second_counter_start_time = Instant::now();

        // Initial render.
        game_renderer.render(
            game,
            game_meta_data,
            &self.settings,
            &keybinds_legend,
            None,
            &mut self.term,
            true,
        )?;

        // Explicitly tells the renderer if entire screen needs to be re-drawn once.
        let mut rerender_entire_view = false;

        // How much time passes between each refresh.
        let frame_interval = Duration::from_secs_f64(self.settings.graphics().game_fps.recip());

        // Time of the game when we enter the game loop.
        let ingametime_when_game_loop_entered = game.state().time;

        // The 'real-life' time at which we enter the game loop.
        let time_game_loop_entered = Instant::now();

        // The number of the frame. This is used to calculate the time of the next frame.
        let mut time_next_frame = time_game_loop_entered;

        // Main Game Loop

        let menu_update = 'update_and_render: loop {
            // Start new iteration of [render->input->] loop.

            if let Some(game_result) = game.result() {
                // Game ended, cannot actually continue playing;
                // Convert to scoreboard entry and return appropriate game-ended menu.
                let scores_entry = ScoresEntry::new(game, game_meta_data);

                let compressed_game_input_history = CompressedInputHistory::new(game_input_history);
                let forfeit =
                    matches!(game_result, Err(GameOver::Forfeit)).then_some(game.state().time);

                let game_restoration_data =
                    GameRestorationData::new(game, compressed_game_input_history, forfeit);

                self.scores_and_replays
                    .entries
                    .push((scores_entry.clone(), Some(game_restoration_data)));

                let menu = if game_result.is_ok() {
                    Menu::GameComplete
                } else {
                    Menu::GameOver
                }(Box::new(scores_entry));

                break 'update_and_render MenuUpdate::Push(menu);
            }

            // Calculate the time of the next render we can catch.
            // We actually just skip a render if we missed the window anyway.
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

                // Read terminal signal or finish waiting.
                match input_receiver.recv_timeout(refresh_time_budget_remaining) {
                    Ok((signal, timestamp)) => {
                        match signal {
                            // Found a recognized game input: use it.
                            LiveTermSignal::RecognizedButton(button, key_event_kind) => {
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

                                if self.runtime_data.kitty_assumed {
                                    // Enhanced keyboard events: determinedly send a single press or release.

                                    let button_change =
                                        (match key_event_kind {
                                            KeyEventKind::Press => ButtonChange::Press,
                                            // Kitty does not actually care about terminal/OS keyboard 'repeat' events.
                                            KeyEventKind::Repeat => continue 'wait,
                                            KeyEventKind::Release => ButtonChange::Release,
                                        })(button);

                                    game_input_history.push((update_target_time, button_change));

                                    match game.update(update_target_time, Some(button_change)) {
                                        Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),
                                        Err(UpdateGameError::GameEnded) => break 'wait,
                                        Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                                    }
                                } else {
                                    // Special handling for terminals that STILL send "release" events despite us assuming it's not enhanced;
                                    // So we don't accidentally interpret them as presses.
                                    if matches!(key_event_kind, KeyEventKind::Release) {
                                        continue 'wait;
                                    }

                                    // Non-enhanced terminal - since we don't have "release" events, we just assume a button press is an instantaneous sequence of press+release.
                                    let button_change = ButtonChange::Press(button);

                                    game_input_history.push((update_target_time, button_change));

                                    match game.update(update_target_time, Some(button_change)) {
                                        Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),
                                        Err(UpdateGameError::GameEnded) => break 'wait,
                                        Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                                    }

                                    // Note that we do not expect a button release to actually end the game or similar, but we handle things properly anyway.
                                    let button_change = ButtonChange::Release(button);

                                    game_input_history.push((update_target_time, button_change));

                                    let update_result =
                                        game.update(update_target_time, Some(button_change));

                                    match update_result {
                                        Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),
                                        Err(UpdateGameError::GameEnded) => break 'wait,
                                        Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                                    }
                                }
                            }

                            // Some other input that does not cause an 'in-game action': Process it.
                            LiveTermSignal::RawEvent(event) => {
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
                                                break 'update_and_render MenuUpdate::Push(
                                                    Menu::Pause,
                                                );
                                            }

                                            // [Ctrl+C]: Abort program.
                                            (KeyCode::Char('c' | 'C'), KeyModifiers::CONTROL) => {
                                                break 'update_and_render MenuUpdate::Push(
                                                    Menu::Quit,
                                                );
                                            }

                                            // [Ctrl+D]: Forfeit game.
                                            (KeyCode::Char('d' | 'D'), KeyModifiers::CONTROL) => {
                                                game.forfeit();

                                                game_renderer.push_game_feedback_msgs([(
                                                    game.state().time,
                                                    Feedback::Text("Forfeit Game!".to_owned()),
                                                )]);

                                                continue 'update_and_render;
                                            }

                                            // [Ctrl+S]: Store savepoint.
                                            (KeyCode::Char('s' | 'S'), KeyModifiers::CONTROL) => {
                                                self.game_saves = (
                                                    0,
                                                    vec![GameSave {
                                                        game_meta_data: game_meta_data.clone(),
                                                        game_restoration_data:
                                                            GameRestorationData::new(
                                                                game,
                                                                game_input_history.clone(),
                                                                matches!(
                                                                    game.result(),
                                                                    Some(Err(GameOver::Forfeit))
                                                                )
                                                                .then_some(game.state().time),
                                                            ),
                                                        inputs_to_load: game_input_history.len(),
                                                    }],
                                                );

                                                game_renderer.push_game_feedback_msgs([(
                                                    game.state().time,
                                                    Feedback::Text(
                                                        "(Savepoint stored!)".to_owned(),
                                                    ),
                                                )]);
                                            }

                                            // [Ctrl+E]: Store seed.
                                            (KeyCode::Char('e' | 'E'), KeyModifiers::CONTROL) => {
                                                self.settings.new_game.custom_seed =
                                                    Some(game.state_init().seed);

                                                game_renderer.push_game_feedback_msgs([(
                                                    game.state().time,
                                                    Feedback::Text(format!(
                                                        "(Seed stored: {})",
                                                        game.state_init().seed
                                                    )),
                                                )]);
                                            }

                                            // [Ctrl+Shift+B]: (Un-)Blindfold.
                                            (KeyCode::Char('b' | 'B'), _)
                                                if modifiers.contains(
                                                    KeyModifiers::CONTROL
                                                        .union(KeyModifiers::SHIFT),
                                                ) =>
                                            {
                                                self.settings.graphics_mut().blindfolded ^= true;
                                                if self.settings.graphics().blindfolded {
                                                    game_renderer.push_game_feedback_msgs([(
                                                        game.state().time,
                                                        Feedback::Text(
                                                            "Blindfolded! [Ctrl+Shift+B]"
                                                                .to_owned(),
                                                        ),
                                                    )]);
                                                } else {
                                                    game_renderer.push_game_feedback_msgs([(
                                                        game.state().time,
                                                        Feedback::Text(
                                                            "Blindfolds removed! [Ctrl+Shift+B]"
                                                                .to_owned(),
                                                        ),
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
                                        rerender_entire_view = true;
                                        break 'wait;
                                    }
                                }
                            }
                        }
                    }

                    Err(recv_timeout_error) => {
                        match recv_timeout_error {
                            // Frame idle/budget expired on its own: leave wait loop.
                            mpsc::RecvTimeoutError::Timeout => {
                                break 'wait;
                            }

                            // Input handler thread died... Pause game for now.
                            mpsc::RecvTimeoutError::Disconnected => {
                                // FIXME: Maybe we could try restarting the thread manually?
                                // Although this error 'seems rare', and pausing the game like so fixes this with just an extra step.
                                break 'update_and_render MenuUpdate::Push(Menu::Pause);
                            }
                        }
                    }
                }
            }

            let now = Instant::now();

            // We first calculate the intended time at time of reaching here.
            let update_target_time = ingametime_when_game_loop_entered
                + now.saturating_duration_since(time_game_loop_entered);

            match game.update(update_target_time, None) {
                // Update.
                Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),

                // We do not care if game ended or time is in past here:
                // We just care about best-effort updating state to show it to player.
                Err(UpdateGameError::GameEnded | UpdateGameError::TargetTimeInPast) => {}
            }

            // Render current state of the game.
            game_renderer.render(
                game,
                game_meta_data,
                &self.settings,
                &keybinds_legend,
                None,
                &mut self.term,
                rerender_entire_view,
            )?;

            renders_per_second_counter += 1;

            // Reset state of this variable since render just occurred.
            rerender_entire_view = false;

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

        if self.runtime_data.kitty_assumed {
            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
            let _v = self.term.execute(event::PopKeyboardEnhancementFlags);
        }

        if let Some(game_result) = game.result() {
            let h_console = terminal::size()?.1;
            if game_result.is_ok() {
                for i in 0..h_console {
                    self.term
                        .execute(MoveTo(0, i))?
                        .execute(Clear(ClearType::CurrentLine))?;
                    std::thread::sleep(Duration::from_secs_f32(0.01));
                }
            } else {
                for i in (0..h_console).rev() {
                    self.term
                        .execute(MoveTo(0, i))?
                        .execute(Clear(ClearType::CurrentLine))?;
                    std::thread::sleep(Duration::from_secs_f32(0.01));
                }
            };
        } else {
            // Game not done:.
            // Manually release any pressed buttons to avoid weird persistent-buttonpress behavior.
            let unpress_time = game.state().time;
            'button_unpressing: for button in Button::VARIANTS {
                if game.state().buttons_pressed[button].is_some() {
                    let button_change = ButtonChange::Release(button);

                    let update_result = game.update(unpress_time, Some(button_change));

                    game_input_history.push((unpress_time, button_change));
                    match update_result {
                        Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),
                        Err(UpdateGameError::GameEnded) => break 'button_unpressing,
                        Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                    }
                }
            }
        }

        Ok(menu_update)
    }
}
