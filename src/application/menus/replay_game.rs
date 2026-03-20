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
use falling_tetromino_engine::{
    Feedback, Game, GameEndCause, InGameTime, Input, Phase, UpdateGameError,
};

use crate::{
    application::{
        Application, GameMetaData, GameRestorationData, GameSave, Menu, MenuUpdate,
        UncompressedInputHistory,
    },
    fmt_helpers::{fmt_duration, replay_keybinds_legend},
    game_renderers::Renderer,
    keybinds::normalize,
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
        replay_length: InGameTime,
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
        if self.temp_data.kitty_assumed {
            let f = Self::KEYBOARD_ENHANCEMENT_FLAGS;
            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
            let _v = self.term.execute(event::PushKeyboardEnhancementFlags(f));
        }

        // Prepare channel from which to receive terminal inputs.
        let (input_sender, input_receiver) = mpsc::channel();

        // Spawn input handler thread.
        let empty_game_control_keybinds = HashMap::new();
        let is_stop_keybind = |code: KeyCode, modifiers: KeyModifiers| {
            matches!(code, KeyCode::Esc)
                || matches!(code, KeyCode::Char('q' | 'Q'))
                || matches!(code, KeyCode::Backspace)
                || matches!(
                    (code, modifiers),
                    (KeyCode::Char('c' | 'C'), KeyModifiers::CONTROL)
                )
        };
        let _join_handle =
            live_input_handler::spawn(input_sender, empty_game_control_keybinds, is_stop_keybind);

        // Replay data/variables setup:

        // Replay: keybinds legend.
        let keybinds_legend = replay_keybinds_legend();

        // Store whether to pause.
        let mut paused = false;

        // This toggle enables users to actually do inputs on the game.
        let mut enable_game_intervention_inputs = false;

        /* FIXME: This is a workaround for FLOATING POINT INPRECISION.
           Originally we had `let replay_speed = 1.0f64;` but then we had issues such as:
        ```
        // Carefully don't go below desired minimum delta...
        if replay_speed > speed_delta {  /* <- rep_spd = 0.05000000000002 > 0.05; */
            replay_speed -= speed_delta; /* <- rep_spd = 0.00000000000002 OOF.    */
        }
        ``` */
        const REPLAY_SPEED_STEP_EQUIVALENT_TO_SPEED_MULTIPLIER_1: u32 = 20;
        const REPLAY_SPEED_STEPSIZE: f64 =
            1.0 / (REPLAY_SPEED_STEP_EQUIVALENT_TO_SPEED_MULTIPLIER_1 as f64);
        let mut replay_speed_stepper = REPLAY_SPEED_STEP_EQUIVALENT_TO_SPEED_MULTIPLIER_1;
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

        let (x_main, y_main) = Application::<T>::fetch_main_xy();
        game_renderer.set_render_offset(usize::from(x_main), usize::from(y_main));
        game_renderer.reset_view_diff_state();
        game_renderer.render(
            &mut self.term,
            &game,
            game_meta_data,
            &self.settings,
            &self.temp_data,
            &keybinds_legend,
            Some((replay_length, calc_speed(replay_speed_stepper))),
        )?;

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

            // Whether we should pause on next frame. Stores a boolean which may request one additional re-render of state.
            let mut next_paused_with_extra_render_request = paused.then_some(false);

            let mut jump_to_anchor: Option<usize> = None;

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
                                            (
                                                KeyCode::Esc
                                                | KeyCode::Char('q' | 'Q')
                                                | KeyCode::Backspace,
                                                _,
                                            ) => {
                                                break 'update_and_render MenuUpdate::Pop;
                                            }

                                            // [Ctrl+C]: Exit program.
                                            (KeyCode::Char('c' | 'C'), KeyModifiers::CONTROL) => {
                                                break 'update_and_render MenuUpdate::Push(
                                                    Menu::Quit,
                                                );
                                            }

                                            // [Ctrl+I]: Enable Interactive Instant-Input Intervention.
                                            (KeyCode::Char('i' | 'I'), KeyModifiers::CONTROL) => {
                                                enable_game_intervention_inputs ^= true;

                                                let str = if enable_game_intervention_inputs {
                                                    "(Enabled inputs)"
                                                } else {
                                                    "(Disabled inputs)"
                                                };

                                                game_renderer.push_game_feedback_msgs([(
                                                    game.state().time,
                                                    Feedback::Message(str.to_owned()),
                                                )]);

                                                next_paused_with_extra_render_request = Some(true);
                                            }

                                            (code, modifiers)
                                                if enable_game_intervention_inputs =>
                                            {
                                                match self
                                                    .settings
                                                    .keybinds()
                                                    .get(&normalize((code, modifiers)))
                                                {
                                                    // No binding: Just ignore.
                                                    None => {}

                                                    // Binding found: Usebutton un-/press.
                                                    Some(&button) => {
                                                        match game.update(
                                                            game.state().time,
                                                            Some(Input::Activate(button)),
                                                        ) {
                                                            Ok(msgs) => game_renderer
                                                                .push_game_feedback_msgs(msgs),
                                                            // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                                                            Err(
                                                                UpdateGameError::TargetTimeInPast,
                                                            ) => {}
                                                            // Game ended.
                                                            Err(UpdateGameError::AlreadyEnded) => {}
                                                        }
                                                        match game.update(
                                                            game.state().time,
                                                            Some(Input::Deactivate(button)),
                                                        ) {
                                                            Ok(msgs) => game_renderer
                                                                .push_game_feedback_msgs(msgs),
                                                            // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                                                            Err(
                                                                UpdateGameError::TargetTimeInPast,
                                                            ) => {}
                                                            // Game ended.
                                                            Err(UpdateGameError::AlreadyEnded) => {}
                                                        }
                                                    }
                                                }

                                                // Pause and render.
                                                next_paused_with_extra_render_request = Some(true);
                                                break 'wait;
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
                                                                    game.phase(),
                                                                    Phase::GameEnd {
                                                                        cause:
                                                                            GameEndCause::Forfeit { .. },
                                                                        ..
                                                                    }
                                                                )
                                                                .then_some(game.state().time),
                                                            ),
                                                        inputs_to_load: inputs_loaded,
                                                    }],
                                                );

                                                game_renderer.push_game_feedback_msgs([(
                                                    game.state().time,
                                                    Feedback::Message(
                                                        "(Stored savepoint)".to_owned(),
                                                    ),
                                                )]);

                                                if paused {
                                                    next_paused_with_extra_render_request =
                                                        Some(true);
                                                    break 'wait;
                                                }
                                            }

                                            // [Ctrl+E]: Store seed.
                                            (KeyCode::Char('e' | 'E'), KeyModifiers::CONTROL) => {
                                                self.settings.new_game.custom_seed =
                                                    Some(game.state_init().seed);

                                                game_renderer.push_game_feedback_msgs([(
                                                    game.state().time,
                                                    Feedback::Message(format!(
                                                        "(Seed stored: {}.)",
                                                        game.state_init().seed
                                                    )),
                                                )]);

                                                if paused {
                                                    next_paused_with_extra_render_request =
                                                        Some(true);
                                                    break 'wait;
                                                }
                                            }

                                            // [Space]: (Un-)Pause replay.
                                            (KeyCode::Char(' '), _) => {
                                                next_paused_with_extra_render_request =
                                                    if paused { None } else { Some(true) };
                                                break 'wait;
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
                                                    if modifier.contains(KeyModifiers::ALT) {
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

                                                if paused {
                                                    next_paused_with_extra_render_request =
                                                        Some(true);
                                                    break 'wait;
                                                }
                                            }

                                            // [-]: Reset replay speed to 1.
                                            (KeyCode::Char('-'), _) => {
                                                replay_speed_stepper = REPLAY_SPEED_STEP_EQUIVALENT_TO_SPEED_MULTIPLIER_1;

                                                if paused {
                                                    next_paused_with_extra_render_request =
                                                        Some(true);
                                                    break 'wait;
                                                }
                                            }

                                            // [Alt+.]: Skip one *update?* forward.
                                            (KeyCode::Char('.'), KeyModifiers::ALT) => {
                                                if let Some(mut update_target_time) =
                                                    game.peek_next_update_time()
                                                {
                                                    let mut opt_input = None;

                                                    let mut do_forfeit = false;

                                                    if let Some(forfeit_time) =
                                                        game_restoration_data.forfeit
                                                    {
                                                        // FIXME: I'm actually not sure about the semantics of whether forfeit or game-update is handled in such a case. Forfeiting is weird I guess.
                                                        if forfeit_time <= update_target_time {
                                                            update_target_time = forfeit_time;
                                                            do_forfeit = true;
                                                        }
                                                    }

                                                    // Note how we use `inputs_loaded` as because this automatically corresponds to the *index* of the next desired input.
                                                    if let Some((next_input_time, input)) =
                                                        game_restoration_data
                                                            .input_history
                                                            .get(inputs_loaded)
                                                    {
                                                        // By using 'less than' we can actually load the environmental game effects and user inputs separately!
                                                        if *next_input_time < update_target_time {
                                                            update_target_time = *next_input_time;
                                                            do_forfeit = false;
                                                            opt_input = Some(*input);
                                                            inputs_loaded += 1;
                                                        }
                                                    }

                                                    match game.update(update_target_time, opt_input)
                                                    {
                                                        Ok(msgs) => game_renderer
                                                            .push_game_feedback_msgs(msgs),
                                                        // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                                                        Err(UpdateGameError::TargetTimeInPast) => {}
                                                        // Game ended, no more inputs.
                                                        Err(UpdateGameError::AlreadyEnded) => {}
                                                    }

                                                    if do_forfeit {
                                                        match game.forfeit() {
                                                            Ok(msgs) => game_renderer
                                                                .push_game_feedback_msgs(msgs),

                                                            // We do not care if game ended or time is in past here.
                                                            Err(
                                                                UpdateGameError::AlreadyEnded
                                                                | UpdateGameError::TargetTimeInPast,
                                                            ) => {}
                                                        };
                                                    }

                                                    next_paused_with_extra_render_request =
                                                        Some(true);
                                                    break 'wait;
                                                }
                                            }

                                            // [.]: Skip one input forward.
                                            (KeyCode::Char('.'), _) => {
                                                if let Some((next_input_time, button_change)) =
                                                    game_restoration_data
                                                        .input_history
                                                        .get(inputs_loaded)
                                                {
                                                    // FIXME: We do not handle degenerate cases where input is available even tho game should forfeit.

                                                    match game.update(
                                                        *next_input_time,
                                                        Some(*button_change),
                                                    ) {
                                                        Ok(msgs) => game_renderer
                                                            .push_game_feedback_msgs(msgs),
                                                        // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                                                        Err(UpdateGameError::TargetTimeInPast) => {}
                                                        // Game ended, no more inputs.
                                                        Err(UpdateGameError::AlreadyEnded) => {}
                                                    }
                                                    inputs_loaded += 1;
                                                    next_paused_with_extra_render_request =
                                                        Some(true);
                                                    break 'wait;
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
                                            (KeyCode::Enter | KeyCode::Char('e' | 'E'), _)
                                                if !game.has_ended() =>
                                            {
                                                // We yank the *exact* gamestate. Leave some dummy in its place that shouldn't be used/relevant...
                                                let the_game = std::mem::replace(
                                                    &mut game,
                                                    Game::builder().build(),
                                                );

                                                let mut the_meta_data = game_meta_data.clone();
                                                the_meta_data.title.push('\'');

                                                break 'update_and_render MenuUpdate::Push(
                                                    Menu::PlayGame {
                                                        game: Box::new(the_game),
                                                        game_input_history: game_restoration_data
                                                            .input_history
                                                            .iter()
                                                            .take(inputs_loaded)
                                                            .copied()
                                                            .collect(),
                                                        game_meta_data: the_meta_data,
                                                        // FIXME: Clone renderer when entering live game from here?
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
                                        let (x_main, y_main) = Application::<T>::fetch_main_xy();
                                        game_renderer.set_render_offset(
                                            usize::from(x_main),
                                            usize::from(y_main),
                                        );
                                        game_renderer.reset_view_diff_state();

                                        if paused {
                                            next_paused_with_extra_render_request = Some(true);
                                        }

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
                    // Workaround for if we o not have a game anchor available: Restore the game from scratch actually.
                    // Note that this is currently only required for modded games.
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

                // Reset renderer's state associated with game (since we could be at any other game state now).
                game_renderer.reset_game_associated_state();

                // Re-render full state.
                game_renderer.render(
                    &mut self.term,
                    &game,
                    game_meta_data,
                    &self.settings,
                    &self.temp_data,
                    &keybinds_legend,
                    Some((replay_length, calc_speed(replay_speed_stepper))),
                )?;

                renders_per_second_counter += 1;

                // Restart update-render loop as if we just entered it.
                continue 'update_and_render;
            } else if !paused && !game.has_ended() {
                // Game has not ended and is not paused: progress the game.

                // We first calculate the intended time at time of reaching here.
                let mut update_target_time = game.state().time
                    + now
                        .saturating_duration_since(time_last_refresh)
                        .mul_f64(calc_speed(replay_speed_stepper));

                let mut do_forfeit = false;

                if let Some(forfeit_time) = game_restoration_data.forfeit {
                    if forfeit_time <= update_target_time {
                        update_target_time = forfeit_time;
                        do_forfeit = true;
                    }
                }

                'feed_inputs: loop {
                    // Note how we use `inputs_loaded` as because this automatically corresponds to the *index* of the next desired input.
                    let Some((next_input_time, button_change)) =
                        game_restoration_data.input_history.get(inputs_loaded)
                    else {
                        // No more inputs.
                        break 'feed_inputs;
                    };

                    if *next_input_time > update_target_time {
                        // Next input would be beyond target, stop loading in inputs.
                        break 'feed_inputs;
                    }

                    match game.update(*next_input_time, Some(*button_change)) {
                        Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),
                        // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                        Err(UpdateGameError::TargetTimeInPast) => {}
                        // Game ended? Do not attempt to feed more inputs.
                        Err(UpdateGameError::AlreadyEnded) => break 'feed_inputs,
                    }

                    inputs_loaded += 1;
                }

                match game.update(update_target_time, None) {
                    // Update.
                    Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),

                    // We do not care if game ended or time is in past here:
                    // We just care about best-effort updating state to show it to player.
                    Err(UpdateGameError::AlreadyEnded | UpdateGameError::TargetTimeInPast) => {}
                }

                if do_forfeit {
                    match game.forfeit() {
                        Ok(msgs) => game_renderer.push_game_feedback_msgs(msgs),

                        // We do not care if game ended or time is in past here.
                        Err(UpdateGameError::AlreadyEnded | UpdateGameError::TargetTimeInPast) => {}
                    };
                }
            }

            // Render frame only if not paused or paused but render requested
            if !paused || next_paused_with_extra_render_request == Some(true) {
                // Render current state of the game.
                game_renderer.render(
                    &mut self.term,
                    &game,
                    game_meta_data,
                    &self.settings,
                    &self.temp_data,
                    &keybinds_legend,
                    Some((replay_length, calc_speed(replay_speed_stepper))),
                )?;

                renders_per_second_counter += 1;
            }

            // Render 'paused' message.
            if !paused && next_paused_with_extra_render_request.is_some()
                || next_paused_with_extra_render_request == Some(true)
            {
                self.term.execute(MoveTo(0, 0))?;
                self.term
                    .execute(PrintStyledContent(Stylize::italic("Replay Paused...")))?;

            // Remove 'paused' message.
            } else if paused && next_paused_with_extra_render_request.is_none() {
                self.term.execute(MoveTo(0, 0))?;
                self.term
                    .execute(Clear(crossterm::terminal::ClearType::CurrentLine))?;

            // Render FPS counter.
            } else if next_paused_with_extra_render_request.is_none()
                && self.settings.graphics().show_fps
            {
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

            // Update values.

            // Remember: We convene on logically setting the 'refresh point' to before the update and render happens.
            time_last_refresh = now;
            paused = next_paused_with_extra_render_request.is_some();
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
                    Err(UpdateGameError::AlreadyEnded) => break 'calculate_anchors,
                }

                inputs_loaded += 1;
            }

            // Game was forfeit before anchor.
            if let Some(forfeit_time) = game_restoration_data.forfeit {
                if forfeit_time <= next_anchor_time {
                    break 'calculate_anchors;
                }
            }

            // Anchor is next.
            match game.update(next_anchor_time, None) {
                Ok(_msgs) => {}
                // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                Err(UpdateGameError::TargetTimeInPast) => {}
                // Game ended, no more anchors.
                Err(UpdateGameError::AlreadyEnded) => break 'calculate_anchors,
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
