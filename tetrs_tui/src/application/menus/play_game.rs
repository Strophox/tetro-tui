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
use tetrs_engine::{Button, ButtonChange, Feedback, Game, UpdateGameError};

use crate::{
    application::{
        Application, CompressedGameInputHistory, GameInputHistory, GameMetaData,
        GameRestorationData, Menu, MenuUpdate, ScoreboardEntry,
    },
    fmt_helpers::get_play_keybinds_legend,
    game_renderers::Renderer,
    live_input_handler::{self, LiveTermSignal},
};

impl<T: Write> Application<T> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::application) fn menu_play_game(
        &mut self,
        game: &mut Game,
        game_meta_data: &mut GameMetaData,
        timestamp_play_started: Instant,
        time_last_paused: &mut Instant,
        total_pause_duration: &mut Duration,
        game_input_history: &mut GameInputHistory,
        game_renderer: &mut impl Renderer,
    ) -> io::Result<MenuUpdate> {
        // Prepare everything to enter the game (react & render) loop.

        let keybinds_legend = get_play_keybinds_legend(self.settings.keybinds());

        // Toggle on enhanced-keyboard-events.
        if self.runtime_data.kitty_assumed {
            let f = Self::KEYBOARD_ENHANCEMENT_FLAGS;
            self.term.execute(event::PushKeyboardEnhancementFlags(f))?;
        }

        // Prepare channel from which to receive terminal inputs.
        let (input_sender, input_receiver) = mpsc::channel();

        // Spawn input handler thread.
        let _join_handle =
            live_input_handler::spawn(input_sender, self.settings.keybinds().clone());

        // Game Loop

        let session_resumed = Instant::now();
        *total_pause_duration += session_resumed.saturating_duration_since(*time_last_paused);

        let mut render_id = 0u32;

        let mut renders_per_second_counter = 0;
        let mut renders_per_second_counter_start_time = Instant::now();

        // Explicitly tells the renderer if entire screen needs to be re-drawn once.
        let mut refresh_entire_view = true;

        let menu_update = 'render_and_input: loop {
            // Start new iteration of [render->input->] loop.

            // Render current state of the game.
            game_renderer.render(
                game,
                game_meta_data,
                &self.settings,
                &keybinds_legend,
                None,
                &mut self.term,
                refresh_entire_view,
            )?;
            renders_per_second_counter += 1;

            // Reset state of this variable since render just occurred.
            refresh_entire_view = false;

            // Render FPS counter.
            if self.settings.graphics().show_fps {
                let now = Instant::now();
                // One second has passed since we started counting.
                if now.saturating_duration_since(renders_per_second_counter_start_time)
                    >= Duration::from_secs(1)
                {
                    self.term.execute(MoveTo(0, 0))?.execute(Print(format!(
                        "{:_>6}",
                        format!("{renders_per_second_counter}fps")
                    )))?;
                    renders_per_second_counter = 0;
                    renders_per_second_counter_start_time = now;
                }
            }

            // Calculate the time of the next render we can catch.
            // We actually completely base this off the start of the session,
            // and just skip a render if we miss the window.
            let now = Instant::now();
            let next_render_at = loop {
                let planned_render_at = session_resumed
                    + Duration::from_secs_f64(
                        f64::from(render_id) / self.settings.graphics().game_fps,
                    );

                if planned_render_at < now {
                    render_id += 1;
                } else {
                    break planned_render_at;
                }
            };

            if let Some(game_result) = game.result() {
                // Game ended, cannot actually continue playing;
                // Convert to scoreboard entry and return appropriate game-ended menu.
                let scoreboard_entry = ScoreboardEntry::new(game, game_meta_data);

                let compressed_game_input_history =
                    CompressedGameInputHistory::new(game_input_history);
                let game_restoration_data =
                    GameRestorationData::new(game, compressed_game_input_history);

                self.scoreboard
                    .entries
                    .push((scoreboard_entry.clone(), Some(game_restoration_data)));

                let menu = if game_result.is_ok() {
                    Menu::GameComplete
                } else {
                    Menu::GameOver
                }(Box::new(scoreboard_entry));

                break 'render_and_input MenuUpdate::Push(menu);
            }

            'frame_idle: loop {
                // Compute time left until we should stop waiting.
                let frame_idle_remaining = next_render_at - Instant::now();

                let recv_result = input_receiver.recv_timeout(frame_idle_remaining);

                match recv_result {
                    Ok((signal, timestamp)) => {
                        match signal {
                            // Found a recognized game input: use it.
                            LiveTermSignal::RecognizedButton(button, key_event_kind) => {
                                let game_time_userinput = timestamp
                                    .saturating_duration_since(timestamp_play_started)
                                    - *total_pause_duration;

                                // Guarantee update cannot fail because it lies in the game's past:
                                // Instead react to player input as quickly as possible now.
                                let update_target_time =
                                    std::cmp::max(game_time_userinput, game.state().time);

                                // Here we actually compress the information in update_target_time:
                                // We round it to millisecond, and up (ceiling, to not be in game's past).
                                let update_target_time_millis =
                                    (update_target_time.as_millis() + 1) as u64;
                                let update_target_time =
                                    Duration::from_millis(update_target_time_millis);

                                if self.runtime_data.kitty_assumed {
                                    // Enhanced keyboard events: determinedly send press or release.
                                    let button_change =
                                        (match key_event_kind {
                                            KeyEventKind::Press => ButtonChange::Press,
                                            // Kitty does not actually care about terminal/OS keyboard 'repeat' events.
                                            KeyEventKind::Repeat => continue 'frame_idle,
                                            KeyEventKind::Release => ButtonChange::Release,
                                        })(button);

                                    let update_result =
                                        game.update(update_target_time, Some(button_change));

                                    game_input_history.push((update_target_time, button_change));
                                    match update_result {
                                        Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),
                                        Err(UpdateGameError::GameEnded) => break 'frame_idle,
                                        Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                                    }
                                } else {
                                    // Normal terminal - since we don't have "release" events, we just assume a button press is instantaneous.
                                    let button_change = ButtonChange::Press(button);

                                    // Special handling for terminal that STILL send "release" events, so we don't interpret them as presses.
                                    if matches!(key_event_kind, KeyEventKind::Release) {
                                        continue 'frame_idle;
                                    }

                                    let update_result =
                                        game.update(update_target_time, Some(button_change));

                                    game_input_history.push((update_target_time, button_change));
                                    match update_result {
                                        Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),
                                        Err(UpdateGameError::GameEnded) => break 'frame_idle,
                                        Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                                    }

                                    // Note that we do not expect a button release to actually end the game or similar, but we handle things properly anyway.
                                    let button_change = ButtonChange::Release(button);

                                    let update_result =
                                        game.update(update_target_time, Some(button_change));

                                    game_input_history.push((update_target_time, button_change));
                                    match update_result {
                                        Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),
                                        Err(UpdateGameError::GameEnded) => break 'frame_idle,
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
                                            continue 'frame_idle;
                                        }

                                        match (code, modifiers) {
                                            // [Esc]: Pause.
                                            (KeyCode::Esc, _) => {
                                                break 'render_and_input MenuUpdate::Push(
                                                    Menu::Pause,
                                                );
                                            }

                                            // [Ctrl+C]: Abort program.
                                            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                                                break 'render_and_input MenuUpdate::Push(
                                                    Menu::Quit,
                                                );
                                            }

                                            // [Ctrl+D]: Forfeit game.
                                            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                                                game.forfeit();

                                                game_renderer.push_game_feedback_msgs([(
                                                    game.state().time,
                                                    Feedback::Text("Forfeit Game!".to_owned()),
                                                )]);

                                                continue 'render_and_input;
                                            }

                                            // [Ctrl+S]: Store savepoint.
                                            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                                                self.game_savepoint = Some((
                                                    game_meta_data.clone(),
                                                    GameRestorationData::new(
                                                        game,
                                                        game_input_history.clone(),
                                                    ),
                                                    game_input_history.len(),
                                                ));

                                                game_renderer.push_game_feedback_msgs([(
                                                    game.state().time,
                                                    Feedback::Text(
                                                        "(Savepoint stored!)".to_owned(),
                                                    ),
                                                )]);
                                            }

                                            // [Ctrl+E]: Store seed.
                                            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
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
                                            (KeyCode::Char('b'), _)
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
                                            _ => continue 'frame_idle,
                                        }
                                    }

                                    event::Event::Mouse(_) => {}
                                    event::Event::Paste(_) => {}
                                    event::Event::FocusGained => {}
                                    event::Event::FocusLost => {}
                                    event::Event::Resize(_, _) => {
                                        // Need to redraw screen for proper centering etc.
                                        refresh_entire_view = true;
                                        continue 'render_and_input;
                                    }
                                }
                            }
                        }
                    }

                    Err(recv_timeout_error) => {
                        match recv_timeout_error {
                            // Frame idle expired on its own: update game.
                            mpsc::RecvTimeoutError::Timeout => {
                                let update_target_time = Instant::now()
                                    .saturating_duration_since(timestamp_play_started)
                                    - *total_pause_duration;

                                let update_result = game.update(update_target_time, None);

                                match update_result {
                                    // Update
                                    Ok(msgs) => {
                                        game_renderer.push_game_feedback_msgs(msgs);
                                    }

                                    Err(_e) => {
                                        // FIXME: Handle UpdateGameError? If not, why not?
                                    }
                                }

                                break 'frame_idle;
                            }

                            // Input handler thread died... Pause game for now.
                            mpsc::RecvTimeoutError::Disconnected => {
                                break 'render_and_input MenuUpdate::Push(Menu::Pause);
                            }
                        }
                    }
                }
            }
        };

        // Play-Game epilogue: De-initialization.
        *time_last_paused = Instant::now();

        if self.runtime_data.kitty_assumed {
            // FIXME: Handle io::Error? If not, why not?
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
