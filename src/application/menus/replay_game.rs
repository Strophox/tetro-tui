use std::{
    collections::HashMap,
    io::{self, Write},
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{Print, PrintStyledContent, Stylize},
    terminal::Clear,
    ExecutableCommand,
};
use falling_tetromino_engine::{Feedback, Game, GameOver, UpdateGameError};

use crate::{
    application::{
        Application, GameMetaData, GameRestorationData, GameSave, Menu, MenuUpdate,
        UncompressedInputHistory,
    },
    fmt_helpers::{fmt_duration, replay_keybinds_legend},
    game_renderers::Renderer,
    live_input_handler::{self, LiveTermSignal},
};

struct GameSaveAnchor {
    game: Game,
    inputs_loaded: usize,
}

impl<T: Write> Application<T> {
    pub(in crate::application) fn run_menu_replay_game(
        &mut self,
        game_restoration_data: &GameRestorationData<UncompressedInputHistory>,
        game_meta_data: &GameMetaData,
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
            self.term.execute(event::PushKeyboardEnhancementFlags(f))?;
        }

        // Prepare channel from which to receive terminal inputs.
        let (input_sender, input_receiver) = mpsc::channel();

        // Spawn input handler thread.
        let no_game_keybinds = HashMap::new();
        let _join_handle = live_input_handler::spawn(input_sender, no_game_keybinds);

        // Replay data/variables setup:

        let mut jump_to_anchor: Option<usize> = None;

        // Replay: keybinds legend.
        let keybinds_legend = replay_keybinds_legend();

        let replay_length = game_restoration_data
            .input_history
            .last()
            .map(|x| x.0)
            .unwrap_or_default();

        let mut is_paused = false;

        /* FIXME: This is a workaround for FLOATING POINT INPRECISION.
           Originally we had `let replay_speed = 1.0f64;` but then we had issues such as:
        ```
        // Carefully don't go below desired minimum delta...
        if replay_speed > speed_delta {  /* <- rep_spd = 0.05000000000002 > 0.05; */
            replay_speed -= speed_delta; /* <- rep_spd = 0.00000000000002 OOF.    */
        }
        ``` */
        const REPLAY_SPEED_STEPSIZE: f64 = 0.05;
        let mut replay_speed_stepper = 20u32;
        const SPEED_SMALL_STEPPER_DELTA: u32 = 1;
        const SPEED_NORMAL_STEPPER_DELTA: u32 = 5;

        let calc_speed =
            |replay_speed_stepper: u32| f64::from(replay_speed_stepper) * REPLAY_SPEED_STEPSIZE;

        // Initialized/load game and generate game_save_anchors if possible.
        const ANCHOR_INTERVAL: Duration = Duration::from_millis(1000);
        let (mut game, game_save_anchors) =
            self.calculate_game_save_anchors(game_restoration_data, ANCHOR_INTERVAL)?;

        let mut inputs_loaded = 0usize;

        // FPS counter.
        let mut renders_per_second_counter = 0u32;
        let mut renders_per_second_counter_start_time = Instant::now();

        // Initial render.
        game_renderer.render(
            &game,
            game_meta_data,
            &self.settings,
            &keybinds_legend,
            Some((replay_length, calc_speed(replay_speed_stepper))),
            &mut self.term,
            true,
        )?;

        // Explicitly tells the renderer if entire screen needs to be re-drawn once.
        let mut rerender_entire_view = false;

        // The 'real-life' time at which we enter the game loop.
        let time_game_loop_entered = Instant::now();

        // How much time passes between each refresh.
        let refresh_time_budget =
            Duration::from_secs_f64(self.settings.graphics().game_fps.recip());

        let mut time_last_refresh = time_game_loop_entered;

        // Main Game Loop

        let menu_update = 'update_and_render: loop {
            // Start new iteration of [render->input->] loop.

            /* NOTE: We could auto-pause when game is over, but we decide not to use the following snippet for now:
            ```
            if let Some(_game_result) = game.result() {
                is_paused = true;
            }
            ``` */

            // Calculate the time of the next render we can catch.
            // We actually completely base this off the start of the session,
            // and just skip a render if we miss the window.
            let time_next_refresh = time_last_refresh + refresh_time_budget;

            'wait: loop {
                // Compute time left until we should stop waiting.
                let refresh_time_budget_remaining = time_next_refresh - Instant::now();

                // Read terminal signal or finish waiting.
                match input_receiver.recv_timeout(refresh_time_budget_remaining) {
                    Ok((signal, _timestamp)) => {
                        match signal {
                            // Found a recognized game input??: DO NOT use it, we're in a game replay.
                            LiveTermSignal::RecognizedButton(_button, _key_event_kind) => {}

                            // Some other input that does not cause an 'in-game action': Process it.
                            LiveTermSignal::RawEvent(event) => {
                                match event {
                                    event::Event::Key(KeyEvent {
                                        code,
                                        modifiers,
                                        kind,
                                        state: _,
                                    }) => {
                                        if matches!(kind, KeyEventKind::Release) {
                                            // It just so happens that, once we're done considering in-game-relevant presses,
                                            // for the remaining controls we only care about key*down*s.
                                            continue 'wait;
                                        }

                                        match (code, modifiers) {
                                            // [Esc]: Stop.
                                            (KeyCode::Esc | KeyCode::Char('q' | 'Q'), _) => {
                                                break 'update_and_render MenuUpdate::Pop;
                                            }

                                            // [Ctrl+C]: Abort program.
                                            (KeyCode::Char('c' | 'C'), KeyModifiers::CONTROL) => {
                                                break 'update_and_render MenuUpdate::Push(
                                                    Menu::Quit,
                                                );
                                            }

                                            // [Ctrl+S]: Store savepoint.
                                            (KeyCode::Char('s' | 'S'), KeyModifiers::CONTROL) => {
                                                self.game_saves = (
                                                    0,
                                                    vec![GameSave {
                                                        game_meta_data: game_meta_data.clone(),
                                                        game_restoration_data:
                                                            GameRestorationData::new(
                                                                &game,
                                                                game_restoration_data
                                                                    .input_history
                                                                    .clone(),
                                                                matches!(
                                                                    game.result(),
                                                                    Some(Err(GameOver::Forfeit))
                                                                )
                                                                .then_some(game.state().time),
                                                            ),
                                                        inputs_to_load: game_restoration_data
                                                            .input_history
                                                            .len(),
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
                                                        "(Seed stored: {}.)",
                                                        game.state_init().seed
                                                    )),
                                                )]);
                                            }

                                            // [Space]: (Un-)Pause replay.
                                            (KeyCode::Char(' '), _) => {
                                                is_paused ^= true;
                                            }

                                            // [↓][↑]: Adjust replay speed.
                                            (
                                                KeyCode::Down
                                                | KeyCode::Char('j' | 'J')
                                                | KeyCode::Up
                                                | KeyCode::Char('k' | 'K'),
                                                modifier,
                                            ) => {
                                                let speed_delta =
                                                    if modifier.contains(KeyModifiers::SHIFT) {
                                                        SPEED_SMALL_STEPPER_DELTA
                                                    } else {
                                                        SPEED_NORMAL_STEPPER_DELTA
                                                    };

                                                if matches!(
                                                    code,
                                                    KeyCode::Up | KeyCode::Char('k' | 'K')
                                                ) {
                                                    replay_speed_stepper += speed_delta;
                                                } else if replay_speed_stepper > speed_delta {
                                                    replay_speed_stepper -= speed_delta;
                                                };
                                            }

                                            // FIXME: Actually catch this keybind for consistency, but don't actually do anything.
                                            // [Ctrl+←]: -
                                            (
                                                KeyCode::Left | KeyCode::Char('h' | 'H'),
                                                KeyModifiers::SHIFT,
                                            ) => {}

                                            // [Ctrl+→]: Skip one input.
                                            (
                                                KeyCode::Right | KeyCode::Char('l' | 'L'),
                                                KeyModifiers::SHIFT,
                                            ) => {
                                                if let Some((next_input_time, button_change)) =
                                                    game_restoration_data
                                                        .input_history
                                                        .get(inputs_loaded)
                                                {
                                                    // VERY Hacky way to advance by one player input.

                                                    time_last_refresh = Instant::now();
                                                    match game.update(
                                                        *next_input_time,
                                                        Some(*button_change),
                                                    ) {
                                                        Ok(msgs) => game_renderer
                                                            .push_game_feedback_msgs(msgs),
                                                        // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                                                        Err(UpdateGameError::TargetTimeInPast) => {}
                                                        // Game ended, no more inputs.
                                                        Err(UpdateGameError::GameEnded) => {}
                                                    }
                                                    inputs_loaded += 1;
                                                    is_paused = true;
                                                    // Re-render full state.
                                                    game_renderer.render(
                                                        &game,
                                                        game_meta_data,
                                                        &self.settings,
                                                        &keybinds_legend,
                                                        Some((
                                                            replay_length,
                                                            calc_speed(replay_speed_stepper),
                                                        )),
                                                        &mut self.term,
                                                        rerender_entire_view,
                                                    )?;
                                                    // Reset state of this variable since render just occurred.
                                                    rerender_entire_view = false;
                                                    // Restart update-render loop as if we just entered it.
                                                    continue 'update_and_render;
                                                }
                                            }

                                            // [←][→]: Skip to anchor save.
                                            (
                                                KeyCode::Left
                                                | KeyCode::Char('h' | 'H')
                                                | KeyCode::Right
                                                | KeyCode::Char('l' | 'L'),
                                                _,
                                            ) => {
                                                let mut anchor_index =
                                                    (game.state().time.as_secs_f64()
                                                        / ANCHOR_INTERVAL.as_secs_f64())
                                                    .floor()
                                                        as usize;

                                                if matches!(
                                                    code,
                                                    KeyCode::Left | KeyCode::Char('h' | 'H')
                                                ) {
                                                    anchor_index = anchor_index.saturating_sub(1);
                                                } else {
                                                    anchor_index += 1;
                                                }

                                                jump_to_anchor = Some(anchor_index);

                                                break 'wait;
                                            }

                                            // [0]-[9]: Skip to X0% anchor save.
                                            (KeyCode::Char(c @ '0'..='9'), _) => {
                                                let n = c.to_string().parse::<u32>().unwrap();

                                                // n/10 * (No.anchors := replen/anchor_interval)
                                                let anchor_index = (f64::from(n) / 10.0
                                                    * (replay_length.as_secs_f64()
                                                        / ANCHOR_INTERVAL.as_secs_f64()))
                                                .floor()
                                                    as usize;

                                                jump_to_anchor = Some(anchor_index);

                                                break 'wait;
                                            }

                                            // [Enter]: Start playable game from here!
                                            (KeyCode::Enter | KeyCode::Char('e' | 'E'), _) => {
                                                // We yank the *exact* gamestate. Leave some dummy in its place that shouldn't be used/relevant...
                                                let the_game = std::mem::replace(
                                                    &mut game,
                                                    Game::builder().build(),
                                                );
                                                break 'update_and_render MenuUpdate::Push(
                                                    Menu::PlayGame {
                                                        game: Box::new(the_game),
                                                        game_input_history: game_restoration_data
                                                            .input_history
                                                            .iter()
                                                            .take(inputs_loaded)
                                                            .cloned()
                                                            .collect(),
                                                        game_meta_data: game_meta_data.clone(),
                                                        game_renderer: Default::default(),
                                                    },
                                                );
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
                                        continue 'update_and_render;
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

                            // Input handler thread died... Pop replay for now.
                            mpsc::RecvTimeoutError::Disconnected => {
                                // FIXME: Maybe we could try restarting the thread manually?
                                // Although this error 'seems rare', and pausing the game like so fixes this with just an extra step.
                                break 'update_and_render MenuUpdate::Pop;
                            }
                        }
                    }
                }
            }

            let now = Instant::now();

            if let Some(anchor_index) = jump_to_anchor.take() {
                // We don't allow skipping beyond last anchor.
                if anchor_index
                    > ((replay_length.as_secs_f64() / ANCHOR_INTERVAL.as_secs_f64()).floor()
                        as usize)
                {
                    continue 'update_and_render;
                }

                // Remember: We convene on logically setting the 'refresh point' to before the update and render happens.
                time_last_refresh = now;

                // Actually jump to position.
                if let Some(game_save_anchors) = &game_save_anchors {
                    if let Some(GameSaveAnchor {
                        game: anchor_game,
                        inputs_loaded: anchor_inputs_loaded,
                    }) = game_save_anchors.get(anchor_index)
                    {
                        game = anchor_game.clone_unmodded();
                        inputs_loaded = *anchor_inputs_loaded;
                    }
                } else {
                    let tgt_time = ANCHOR_INTERVAL.mul_f64(anchor_index as f64);
                    let idx = match game_restoration_data
                        .input_history
                        .binary_search_by_key(&tgt_time, |d_bc| d_bc.0)
                    {
                        Ok(idx) | Err(idx) => idx,
                    };
                    game = game_restoration_data.restore(idx);
                    match game.update(tgt_time, None) {
                        Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),
                        // FIXME: Handle UpdateGameError? If not, why not?
                        Err(_e) => {}
                    }
                    inputs_loaded = idx;
                }

                // Reset renderer.
                *game_renderer = Default::default();

                // Re-render full state.
                game_renderer.render(
                    &game,
                    game_meta_data,
                    &self.settings,
                    &keybinds_legend,
                    Some((replay_length, calc_speed(replay_speed_stepper))),
                    &mut self.term,
                    true,
                )?;

                // Reset state of this variable since render just occurred.
                rerender_entire_view = false;

                // Restart update-render loop as if we just entered it.
                continue 'update_and_render;
            }

            if is_paused || game.result().is_some() {
                // We're paused.

                self.term.execute(MoveTo(0, 0))?;
                self.term
                    .execute(PrintStyledContent(Stylize::italic("Replay Paused...")))?;
            } else {
                // Game has not ended and is not paused: progress the game.

                self.term.execute(MoveTo(0, 0))?;
                self.term
                    .execute(Clear(crossterm::terminal::ClearType::CurrentLine))?;

                // We first calculate the intended time at time of reaching here.
                let update_target_time = game.state().time
                    + now
                        .saturating_duration_since(time_last_refresh)
                        .mul_f64(calc_speed(replay_speed_stepper));

                'feed_inputs: loop {
                    let Some((next_input_time, button_change)) =
                        game_restoration_data.input_history.get(inputs_loaded)
                    else {
                        // No more inputs.
                        break 'feed_inputs;
                    };

                    if update_target_time < *next_input_time {
                        // Target reached.
                        break 'feed_inputs;
                    }

                    match game.update(*next_input_time, Some(*button_change)) {
                        Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),
                        // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                        Err(UpdateGameError::TargetTimeInPast) => {}
                        // Game ended, no more inputs.
                        Err(UpdateGameError::GameEnded) => break 'feed_inputs,
                    }

                    inputs_loaded += 1;
                }

                let (update_target_time, do_forfeit) =
                    if let Some(forfeit_time) = game_restoration_data.forfeit {
                        if forfeit_time <= update_target_time {
                            (forfeit_time, true)
                        } else {
                            (update_target_time, false)
                        }
                    } else {
                        (update_target_time, false)
                    };

                match game.update(update_target_time, None) {
                    // Update.
                    Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),

                    // We do not care if game ended or time is in past here:
                    // We just care about best-effort updating state to show it to player.
                    Err(UpdateGameError::GameEnded | UpdateGameError::TargetTimeInPast) => {}
                }

                if do_forfeit {
                    let msg = game.forfeit();
                    game_renderer.push_game_feedback_msgs([msg])
                }
            }

            // Remember: We convene on logically setting the 'refresh point' to before the update and render happens.
            time_last_refresh = now;

            // Render current state of the game.
            game_renderer.render(
                &game,
                game_meta_data,
                &self.settings,
                &keybinds_legend,
                Some((replay_length, calc_speed(replay_speed_stepper))),
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
            self.term.execute(event::PopKeyboardEnhancementFlags)?;
        }

        Ok(menu_update)
    }

    // NOTE: We do not treat degenerate games that end immediately (total time = 0).
    fn calculate_game_save_anchors(
        &mut self,
        game_restoration_data: &GameRestorationData<UncompressedInputHistory>,
        anchor_interval: Duration,
    ) -> io::Result<(Game, Option<Vec<GameSaveAnchor>>)> {
        let initial_game = game_restoration_data.restore(0);

        // We don't have replay anchors for modded games, because we can't even attempt to clone the mods' internal states at time of writing.
        if !game_restoration_data.mod_descriptors.is_empty() {
            return Ok((initial_game, None));
        }

        let replay_length = game_restoration_data
            .input_history
            .last()
            .map(|x| x.0)
            .unwrap_or_default();

        let mut game = initial_game.clone_unmodded();
        let mut inputs_loaded = 0usize;

        let mut game_save_anchors = vec![GameSaveAnchor {
            game: game.clone_unmodded(),
            inputs_loaded,
        }];

        let mut next_anchor_time = game.state().time + anchor_interval;

        'calculate_anchors: loop {
            self.term.execute(MoveTo(0, 0))?;
            self.term
                .execute(PrintStyledContent(Stylize::italic(format!(
                    "Loading replay... (precalculated {}/{})",
                    fmt_duration(game.state().time),
                    fmt_duration(replay_length)
                ))))?;

            'feed_inputs: loop {
                let Some((next_input_time, button_change)) =
                    game_restoration_data.input_history.get(inputs_loaded)
                else {
                    // No more inputs.
                    break 'feed_inputs;
                };

                if next_anchor_time < *next_input_time {
                    // Anchor reached.
                    break 'feed_inputs;
                }

                match game.update(*next_input_time, Some(*button_change)) {
                    Ok(_msgs) => {}
                    // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                    Err(UpdateGameError::TargetTimeInPast) => {}
                    // Game ended, no more anchors.
                    Err(UpdateGameError::GameEnded) => break 'calculate_anchors,
                }

                inputs_loaded += 1;
            }

            // Anchor is next.
            match game.update(next_anchor_time, None) {
                Ok(_msgs) => {}
                // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                Err(UpdateGameError::TargetTimeInPast) => {}
                // Game ended, no more anchors.
                Err(UpdateGameError::GameEnded) => break 'calculate_anchors,
            }

            game_save_anchors.push(GameSaveAnchor {
                game: game.clone_unmodded(),
                inputs_loaded,
            });

            next_anchor_time = game.state().time + anchor_interval
        }

        Ok((initial_game, Some(game_save_anchors)))
    }
}

// FIXME: Use or remove.
// pub fn stream_updates(game: &mut Game, input_stream: impl IntoIterator<Item = (InGameTime, Option<ButtonChange>)>) -> Result<Vec<FeedbackMsg>, UpdateGameError> {
//     let mut msgs = Vec::new();

//     for (target_time, button_changes) in input_stream {
//         msgs.extend(game.update(target_time, button_changes)?);
//     }

//     Ok(msgs)
// }
