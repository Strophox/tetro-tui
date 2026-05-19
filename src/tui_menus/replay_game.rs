use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

use crate::{
    core_game_engine::{
        Game, GameEndCause, InGameTime, Input, Notification, Phase, UpdateGameError,
    },
    terminal_buffers::TermCell,
};
use crossterm::{
    ExecutableCommand,
    cursor::MoveTo,
    event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{Print, PrintStyledContent, Stylize},
    terminal::{self, Clear},
};

use crate::{
    Application, GameMetaData, GameSave,
    fmt_helpers::{BoolAsOnOff, fmt_duration, increment_game_mode_derivative},
    game_renderers::{GameRenderer, MiscGameRenderers, replay_keybinds_legend},
    game_restoration::{GameRestorationData, RawInputHistory},
    tui_menus::{Menu, MenuUpdate},
};

#[derive(Debug)]
pub struct GameSaveAnchor {
    game: Game,
    inputs_loaded: usize,
}

impl<W: Write> Application<W> {
    pub fn run_menu_replay_game(
        &mut self,
        game_restoration_data: &GameRestorationData<RawInputHistory>,
        game_meta_data: &GameMetaData,
        replay_length: InGameTime,
        game_renderer: &mut MiscGameRenderers,
        cached_game_and_replay_anchors: &mut (Game, Option<Vec<GameSaveAnchor>>),
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
            // NOTE: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
            let _r = self.term.execute(event::PushKeyboardEnhancementFlags(f));
        }

        // Replay data/variables setup:

        // Replay: keybinds legend.
        let keybinds_legend = replay_keybinds_legend();

        // Store whether to pause.
        let mut paused = false;

        let mut loop_replay = false;

        // This toggle enables users to actually do inputs on the game.
        let mut enable_game_intervention_inputs = false;

        /*
        FIXME: This is a workaround for floating point imprecision.
           Originally we had `let replay_speed = 1.0f64;` but then we had issues such as:
        ```
        // Carefully don't go below desired minimum delta...
        if replay_speed > speed_delta {  /* <- rep_spd = 0.05000000000002 > 0.05; */
            replay_speed -= speed_delta; /* <- rep_spd = 0.00000000000002 OOF.    */
        }
        ```
        */
        const REPLAY_SPEED_STEP_EQUIVALENT_TO_SPEED_MULTIPLIER_1: u32 = 20;
        const REPLAY_SPEED_STEPSIZE: f64 =
            1.0 / (REPLAY_SPEED_STEP_EQUIVALENT_TO_SPEED_MULTIPLIER_1 as f64);
        let mut replay_speed_stepper = REPLAY_SPEED_STEP_EQUIVALENT_TO_SPEED_MULTIPLIER_1;
        const SPEED_SMALL_STEPPER_DELTA: u32 = 1;
        const SPEED_NORMAL_STEPPER_DELTA: u32 = 5;

        let calc_speed =
            |replay_speed_stepper: u32| f64::from(replay_speed_stepper) * REPLAY_SPEED_STEPSIZE;

        // Initialized/load game and generate game_save_anchors if possible.
        // FIXME: Precalculate this once and cache in Menu:ReplayGame struct.
        let (game, replay_anchors) = cached_game_and_replay_anchors;

        let mut inputs_loaded = 0usize;

        // FPS counter.
        let mut renders_per_second_counter = 0u32;
        let mut renders_per_second_counter_start_time = Instant::now();

        // Initial render.

        game_renderer.reset_viewport_state(
            (0, 0),
            terminal::size().unwrap_or_default(),
            TermCell {
                bg: self.settings.tui_coloring().bg_tui,
                ..TermCell::BLANK
            },
        );
        game_renderer.render(
            &mut self.term,
            game,
            game_meta_data,
            &self.settings,
            &self.temp_data,
            &keybinds_legend,
            Some((replay_length, calc_speed(replay_speed_stepper))),
        )?;

        // The 'real-life' time at which we enter the game loop.
        let time_game_loop_entered = Instant::now();

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

            // How much time passes between each refresh.
            let refresh_time_budget =
                Duration::from_secs_f64(self.settings.graphics().fps.get().recip());
            // Calculate the time of the next render we can catch.
            // We actually completely base this off the start of the session,
            // and just skip a render if we miss the window.
            let time_next_refresh = time_last_refresh + refresh_time_budget;

            'wait: loop {
                // Compute time left until we should stop waiting.
                let Some(refresh_time_budget_remaining) =
                    time_next_refresh.checked_duration_since(Instant::now())
                else {
                    break 'wait;
                };

                // Wait for poll response.
                let event_available = event::poll(refresh_time_budget_remaining)?;

                // Finished waiting with no terminal signal available.
                if !event_available {
                    break 'wait;
                }

                // FIXME: Unused code: for performance measurements, this busy-spin can be used.
                // let event_available = event::poll(Duration::ZERO)?;
                // if !event_available {
                //     if time_next_refresh.checked_duration_since(Instant::now()).is_none() {
                //         break 'wait
                //     } else {
                //         continue 'wait
                //     }
                // }

                let event = event::read()?;
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
                            // [Ctrl+C]: Quit program.
                            (KeyCode::Char('c' | 'C'), KeyModifiers::CONTROL) => {
                                break 'update_and_render MenuUpdate::Push(Menu::Quit);
                            }

                            // Keybinds help menu.
                            (KeyCode::Char('?'), _) => {
                                let client_menu_name = "Game Replay";
                                let legend = vec![
                                    ("Normal keybinds".to_owned(), [
                                        ("Enter e", "Start Game from current replay state ('take over')"),
                                        ("Escape Backspace q", "Exit replay"),
                                        ("Space", "Pause replay"),
                                        ("↓/↑ j/k", "Speed up / slow down replay by ±0.25x"),
                                        ("-", "Reset replay speed to =1.0x"),
                                        ("←/→, h/l", "Skip backward / forward 1s in time"),
                                        (".", "Skip forward one player input & pause"),
                                        ("1/2/3...", "Jump to 10%/20%/30%/... of replay"),
                                        ("Home/End", "Jump to beginning/end of replay"),
                                        ("?", "Open Keybinds overview"),
                                        ].into_iter().map(|(lhs,rhs)| (lhs.to_owned(), rhs.to_owned())).collect()),
                                    ("Special keybinds".to_owned(), [
                                        ("Alt+↓/↑ Alt+j/k", "Speed up / slow down replay by ±0.05x"),
                                        ("Alt+.", "Skip forward one game state change & pause (might not work properly for modded games)"),
                                        ("Ctrl+L", "Toggle replay loop"),
                                        ("Ctrl+S", "Store game save"),
                                        ("Ctrl+E", "Store seed for custom game"),
                                        ("Alt+I", "(Experimental) Toggle instantaneous interactive input intervention mode"),
                                        ("Ctrl+G/Ctrl+Alt+G", "Cycle forward/backward through Graphics slots"),
                                        ("Ctrl+C", "Quit program (respects save preferences)"),
                                        ("Ctrl+Alt+L", "Reload app from savefile (overwrites current data!)"),
                                        ("Ctrl+Alt+S", "Perform savefile store (respects save preferences)"),
                                    ].into_iter().map(|(lhs,rhs)| (lhs.to_owned(), rhs.to_owned())).collect()),
                                ];

                                break 'update_and_render MenuUpdate::Push(
                                    Menu::KeybindsOverview {
                                        client_menu_name,
                                        legend,
                                    },
                                );
                            }

                            // [Esc]: Stop.
                            (KeyCode::Esc | KeyCode::Char('q' | 'Q') | KeyCode::Backspace, _) => {
                                break 'update_and_render MenuUpdate::Pop;
                            }

                            // [Ctrl+I]: Enable Interactive Instant-Input Intervention.
                            (KeyCode::Char('i' | 'I'), KeyModifiers::ALT) => {
                                enable_game_intervention_inputs ^= true;

                                let str = if enable_game_intervention_inputs {
                                    "(Inter-inputs enabled)"
                                } else {
                                    "(Inter-inputs disabled)"
                                };

                                game_renderer.update_feed(
                                    [(Notification::Custom(str.to_owned()), game.state().time)],
                                    &self.settings,
                                );

                                next_paused_with_extra_render_request = Some(true);
                            }

                            (code, modifiers) if enable_game_intervention_inputs => {
                                match self.settings.keybinds().get((code, modifiers)) {
                                    // No binding: Just ignore.
                                    None => {}

                                    // Binding found: Usebutton un-/press.
                                    Some(&button) => {
                                        match game.update(
                                            game.state().time,
                                            Some(Input::Activate(button)),
                                        ) {
                                            Ok(msgs) => {
                                                game_renderer.update_feed(msgs, &self.settings)
                                            }
                                            // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                                            Err(UpdateGameError::TargetTimeInPast) => {}
                                            // Game ended.
                                            Err(UpdateGameError::AlreadyEnded) => {}
                                        }
                                        match game.update(
                                            game.state().time,
                                            Some(Input::Deactivate(button)),
                                        ) {
                                            Ok(msgs) => {
                                                game_renderer.update_feed(msgs, &self.settings)
                                            }
                                            // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                                            Err(UpdateGameError::TargetTimeInPast) => {}
                                            // Game ended.
                                            Err(UpdateGameError::AlreadyEnded) => {}
                                        }
                                    }
                                }

                                // Pause and render.
                                next_paused_with_extra_render_request = Some(true);
                                break 'wait;
                            }

                            // [Ctrl+S]: Store game save.
                            (KeyCode::Char('s' | 'S'), KeyModifiers::CONTROL) => {
                                self.game_saves.selected = 0;
                                self.game_saves.slots = vec![GameSave {
                                    game_meta_data: game_meta_data.clone(),
                                    game_restoration_data: GameRestorationData::new(
                                        game,
                                        game_restoration_data.input_history.clone(),
                                        matches!(
                                            game.phase(),
                                            Phase::GameEnd {
                                                cause: GameEndCause::Forfeit { .. },
                                                ..
                                            }
                                        )
                                        .then_some(game.state().time),
                                    ),
                                    inputs_to_load: inputs_loaded,
                                }];

                                game_renderer.update_feed(
                                    [(
                                        Notification::Custom("(Game save stored)".to_owned()),
                                        game.state().time,
                                    )],
                                    &self.settings,
                                );

                                if paused {
                                    next_paused_with_extra_render_request = Some(true);
                                    break 'wait;
                                }
                            }

                            // [Ctrl+E]: Store seed.
                            (KeyCode::Char('e' | 'E'), KeyModifiers::CONTROL) => {
                                self.settings.game_mode_preferences.custom_config.seed =
                                    Some(game.state_init().seed);

                                game_renderer.update_feed(
                                    [(
                                        Notification::Custom(format!(
                                            "(Seed stored: {}.)",
                                            game.state_init().seed
                                        )),
                                        game.state().time,
                                    )],
                                    &self.settings,
                                );

                                if paused {
                                    next_paused_with_extra_render_request = Some(true);
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
                                let speed_delta = if modifier.contains(KeyModifiers::ALT) {
                                    SPEED_SMALL_STEPPER_DELTA
                                } else {
                                    SPEED_NORMAL_STEPPER_DELTA
                                };

                                if matches!(code, KeyCode::Up | KeyCode::Char('k' | 'K')) {
                                    replay_speed_stepper += speed_delta;
                                } else if replay_speed_stepper > speed_delta {
                                    replay_speed_stepper -= speed_delta;
                                };

                                if paused {
                                    next_paused_with_extra_render_request = Some(true);
                                    break 'wait;
                                }
                            }

                            // [-]: Reset replay speed to 1.
                            (KeyCode::Char('-'), _) => {
                                replay_speed_stepper =
                                    REPLAY_SPEED_STEP_EQUIVALENT_TO_SPEED_MULTIPLIER_1;

                                if paused {
                                    next_paused_with_extra_render_request = Some(true);
                                    break 'wait;
                                }
                            }

                            // [Alt+.]: Skip one *update?* forward.
                            (KeyCode::Char('.'), KeyModifiers::ALT) => {
                                if let Some(mut update_target_time) = game.peek_next_update_time() {
                                    let mut opt_input = None;

                                    let mut do_forfeit = false;

                                    if let Some(forfeit_time) = game_restoration_data.forfeit
                                        && forfeit_time <= update_target_time
                                    {
                                        update_target_time = forfeit_time;
                                        do_forfeit = true;
                                    }

                                    // Note how we use `inputs_loaded` as because this automatically corresponds to the *index* of the next desired input.
                                    if let Some((next_input_time, input)) = game_restoration_data
                                        .input_history
                                        .inputs
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

                                    match game.update(update_target_time, opt_input) {
                                        Ok(msgs) => game_renderer.update_feed(msgs, &self.settings),
                                        // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                                        Err(UpdateGameError::TargetTimeInPast) => {}
                                        // Game ended, no more inputs.
                                        Err(UpdateGameError::AlreadyEnded) => {}
                                    }

                                    if do_forfeit {
                                        match game.forfeit() {
                                            Ok(msgs) => {
                                                game_renderer.update_feed(msgs, &self.settings)
                                            }

                                            // We do not care if game ended or time is in past here.
                                            Err(
                                                UpdateGameError::AlreadyEnded
                                                | UpdateGameError::TargetTimeInPast,
                                            ) => {}
                                        };
                                    }

                                    next_paused_with_extra_render_request = Some(true);
                                    break 'wait;
                                }
                            }

                            // [.]: Skip one input forward.
                            (KeyCode::Char('.'), _) => {
                                if let Some((next_input_time, input)) = game_restoration_data
                                    .input_history
                                    .inputs
                                    .get(inputs_loaded)
                                {
                                    // FIXME: We do not handle degenerate cases where input is available even though game should also be forfeit.

                                    match game.update(*next_input_time, Some(*input)) {
                                        Ok(msgs) => game_renderer.update_feed(msgs, &self.settings),
                                        // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                                        Err(UpdateGameError::TargetTimeInPast) => {}
                                        // Game ended, no more inputs.
                                        Err(UpdateGameError::AlreadyEnded) => {}
                                    }
                                    inputs_loaded += 1;
                                    next_paused_with_extra_render_request = Some(true);
                                    break 'wait;
                                }
                            }

                            // [←][→]: Skip to anchor save.
                            (
                                KeyCode::Left
                                | KeyCode::Char('h' | 'H')
                                | KeyCode::Right
                                | KeyCode::Char('l' | 'L'),
                                KeyModifiers::NONE,
                            ) => {
                                let mut anchor_index = (game.state().time.as_secs_f64()
                                    / REPLAY_ANCHOR_INTERVAL.as_secs_f64())
                                .floor()
                                    as usize;

                                if matches!(code, KeyCode::Left | KeyCode::Char('h' | 'H')) {
                                    if game.state().time.saturating_sub(
                                        REPLAY_ANCHOR_INTERVAL * (anchor_index as u32),
                                    ) < REPLAY_ANCHOR_INTERVAL / 2
                                    {
                                        anchor_index = anchor_index.saturating_sub(1);
                                    }
                                } else {
                                    anchor_index += 1;
                                }

                                jump_to_anchor = Some(anchor_index);

                                break 'wait;
                            }

                            // [Home]: Skip to 0% anchor save.
                            (KeyCode::Home, _) => {
                                jump_to_anchor = Some(0);
                            }

                            // [End]: Skip to ~100% anchor save.
                            (KeyCode::End, _) => {
                                jump_to_anchor = Some(
                                    (replay_length.as_secs_f64()
                                        / REPLAY_ANCHOR_INTERVAL.as_secs_f64())
                                    .floor() as usize,
                                );
                            }

                            // [0]-[9]: Skip to X0% anchor save.
                            (KeyCode::Char(c @ '0'..='9'), _) => {
                                let n = c.to_string().parse::<u32>().unwrap();

                                // n/10 * (No.anchors := replen/anchor_interval)
                                let anchor_index = (f64::from(n) / 10.0
                                    * (replay_length.as_secs_f64()
                                        / REPLAY_ANCHOR_INTERVAL.as_secs_f64()))
                                .floor()
                                    as usize;

                                jump_to_anchor = Some(anchor_index);

                                break 'wait;
                            }

                            // [Enter]: Start playable game from here!
                            (KeyCode::Enter | KeyCode::Char('e' | 'E'), _) if !game.has_ended() => {
                                // We yank the *exact* gamestate. Leave some dummy in its place that shouldn't be used/relevant...
                                let the_game = std::mem::replace(game, Game::builder().build());

                                let mut the_meta_data = game_meta_data.clone();
                                increment_game_mode_derivative(&mut the_meta_data.title);

                                // FIXME: Instead clone renderer when entering live game from here?
                                let the_game_renderer =
                                    MiscGameRenderers::with_num(self.temp_data.renderer_used);

                                self.statistics.new_games_started += 1;

                                break 'update_and_render MenuUpdate::Push(Menu::PlayGame {
                                    game: Box::new(the_game),
                                    raw_input_history: game_restoration_data
                                        .input_history
                                        .inputs
                                        .iter()
                                        .take(inputs_loaded)
                                        .copied()
                                        .collect::<Vec<_>>()
                                        .into(),
                                    game_meta_data: the_meta_data,
                                    game_renderer: Box::new(the_game_renderer),
                                    selection_id_for_game_retry: None,
                                });
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
                                game_renderer.reset_viewport_state(
                                    (0, 0),
                                    terminal::size().unwrap_or_default(),
                                    TermCell {
                                        bg: self.settings.tui_coloring().bg_tui,
                                        ..TermCell::BLANK
                                    },
                                );

                                if paused {
                                    next_paused_with_extra_render_request = Some(true);
                                }
                            }

                            // [Ctrl+L]: Loop replay.
                            (KeyCode::Char('l' | 'L'), KeyModifiers::CONTROL) => {
                                loop_replay ^= true;

                                game_renderer.update_feed(
                                    [(
                                        Notification::Custom(format!(
                                            "(Replay loop toggled {}.)",
                                            loop_replay.on_off()
                                        )),
                                        game.state().time,
                                    )],
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

                                if paused {
                                    next_paused_with_extra_render_request = Some(true);
                                }
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

                    event::Event::Mouse(_) => {}
                    event::Event::Paste(_) => {}
                    event::Event::FocusGained => {}
                    // Do not do anything special on focus lost.
                    // This should be like leaving a video playing even though the player isn't focused right now.
                    event::Event::FocusLost => {}
                    event::Event::Resize(cols, rows) => {
                        // Need to redraw screen for proper centering etc.
                        game_renderer.reset_viewport_state(
                            (0, 0),
                            (cols, rows),
                            TermCell {
                                bg: self.settings.tui_coloring().bg_tui,
                                ..TermCell::BLANK
                            },
                        );

                        if paused {
                            next_paused_with_extra_render_request = Some(true);
                        }

                        break 'wait;
                    }
                }
            }

            // Outside of render wait loop. Begin render.

            let now = Instant::now();

            if let Some(anchor_index) = jump_to_anchor.take() {
                // We don't allow skipping beyond last anchor.
                if anchor_index
                    > ((replay_length.as_secs_f64() / REPLAY_ANCHOR_INTERVAL.as_secs_f64()).floor()
                        as usize)
                {
                    continue 'update_and_render;
                }

                // Remember: We convene on logically setting the 'refresh point' to before the update and render happens.
                time_last_refresh = now;

                // Actually jump to position.
                if let Some(game_save_anchors) = &replay_anchors {
                    if let Some(GameSaveAnchor {
                        game: anchor_game,
                        inputs_loaded: anchor_inputs_loaded,
                    }) = game_save_anchors.get(anchor_index)
                    {
                        *game = anchor_game.try_clone().unwrap();
                        inputs_loaded = *anchor_inputs_loaded;
                    }
                } else {
                    // Workaround for if we o not have a game anchor available: Restore the game from scratch actually.
                    // Note that this is currently only required for modded games.
                    let tgt_time = REPLAY_ANCHOR_INTERVAL.mul_f64(anchor_index as f64);
                    let idx = match game_restoration_data
                        .input_history
                        .inputs
                        .binary_search_by_key(&tgt_time, |d_bc| d_bc.0)
                    {
                        Ok(idx) | Err(idx) => idx,
                    };
                    *game = game_restoration_data.restore(idx);
                    match game.update(tgt_time, None) {
                        Ok(msgs) => game_renderer.update_feed(msgs, &self.settings),
                        // FIXME: Handle UpdateGameError? If not, why not?
                        Err(_e) => {}
                    }
                    inputs_loaded = idx;
                }

                // Reset renderer's state associated with game (since we could be at any other game state now).
                game_renderer.reset_veffects_state();

                // Re-render full state.
                game_renderer.render(
                    &mut self.term,
                    game,
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

                if let Some(forfeit_time) = game_restoration_data.forfeit
                    && forfeit_time <= update_target_time
                {
                    update_target_time = forfeit_time;
                    do_forfeit = true;
                }

                'feed_inputs: loop {
                    // Note how we use `inputs_loaded` as because this automatically corresponds to the *index* of the next desired input.
                    let Some((next_input_time, input)) = game_restoration_data
                        .input_history
                        .inputs
                        .get(inputs_loaded)
                    else {
                        // No more inputs.
                        break 'feed_inputs;
                    };

                    if *next_input_time > update_target_time {
                        // Next input would be beyond target, stop loading in inputs.
                        break 'feed_inputs;
                    }

                    match game.update(*next_input_time, Some(*input)) {
                        Ok(msgs) => game_renderer.update_feed(msgs, &self.settings),
                        // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                        Err(UpdateGameError::TargetTimeInPast) => {}
                        // Game ended? Do not attempt to feed more inputs.
                        Err(UpdateGameError::AlreadyEnded) => break 'feed_inputs,
                    }

                    inputs_loaded += 1;
                }

                match game.update(update_target_time, None) {
                    // Update.
                    Ok(msgs) => game_renderer.update_feed(msgs, &self.settings),

                    // We do not care if game ended or time is in past here:
                    // We just care about best-effort updating state to show it to player.
                    Err(UpdateGameError::AlreadyEnded | UpdateGameError::TargetTimeInPast) => {}
                }

                if do_forfeit {
                    match game.forfeit() {
                        Ok(msgs) => game_renderer.update_feed(msgs, &self.settings),

                        // We do not care if game ended or time is in past here.
                        Err(UpdateGameError::AlreadyEnded | UpdateGameError::TargetTimeInPast) => {}
                    };
                }
            } else if game.has_ended() && loop_replay {
                // Loop replay.
                *game = game_restoration_data.restore(0);
                inputs_loaded = 0;
                game_renderer.reset_veffects_state();
                game_renderer.update_feed(
                    [(
                        Notification::Custom("(Looping replay.)".to_owned()),
                        game.state().time,
                    )],
                    &self.settings,
                );
            }

            // Render frame only if not paused or paused but render requested
            if !paused || next_paused_with_extra_render_request == Some(true) {
                // Render current state of the game.
                game_renderer.render(
                    &mut self.term,
                    game,
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
                self.term.execute(PrintStyledContent(
                    "Replay Paused..."
                        .italic()
                        .with(self.settings.tui_coloring().fg_accent)
                        .on(self.settings.tui_coloring().bg_tui),
                ))?;

            // Remove 'paused' message.
            } else if paused && next_paused_with_extra_render_request.is_none() {
                self.term.execute(MoveTo(0, 0))?;
                self.term.execute(PrintStyledContent(
                    "                "
                        .italic()
                        .with(self.settings.tui_coloring().fg_accent)
                        .on(self.settings.tui_coloring().bg_tui),
                ))?;

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
            // NOTE: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm doesn't like operating on Windows.
            let _r = self.term.execute(event::PopKeyboardEnhancementFlags);
        }

        Ok(menu_update)
    }
}

pub const REPLAY_ANCHOR_INTERVAL: Duration = Duration::from_millis(1000);

// FIXME: Improve error detection for fail cases, possibly use renderer feed.
// We currently do not treat degenerate games that end immediately (total time = 0).
pub fn calculate_game_and_replay_anchors(
    term: &mut impl Write,
    game_restoration_data: &GameRestorationData<RawInputHistory>,
    anchor_interval: Duration,
    replay_length: InGameTime,
) -> io::Result<(Game, Option<Vec<GameSaveAnchor>>)> {
    let initial_game = game_restoration_data.restore(0);

    let mut game = match initial_game.try_clone() {
        Ok(game) => game,
        // We don't have replay anchors for modded games, because we can't even attempt to clone the mods' internal states at time of writing.
        Err(_e) => return Ok((initial_game, None)),
    };
    let mut inputs_loaded = 0usize;

    let mut game_save_anchors = vec![GameSaveAnchor {
        game: game.try_clone().unwrap(),
        inputs_loaded,
    }];

    let mut next_anchor_time = game.state().time + anchor_interval;

    'calculate_anchors: loop {
        term.execute(MoveTo(0, 0))?;
        term.execute(PrintStyledContent(Stylize::italic(format!(
            "Loading replay... (precalculated {}/{})",
            fmt_duration(game.state().time),
            fmt_duration(replay_length)
        ))))?;

        'feed_inputs: loop {
            let Some((next_input_time, input)) = game_restoration_data
                .input_history
                .inputs
                .get(inputs_loaded)
            else {
                // No more inputs.
                break 'feed_inputs;
            };

            if next_anchor_time < *next_input_time {
                // Anchor reached.
                break 'feed_inputs;
            }

            match game.update(*next_input_time, Some(*input)) {
                Ok(_msgs) => {}
                // FIXME: Handle UpdateGameError::TargetTimeInPast? If not, why not?
                Err(UpdateGameError::TargetTimeInPast) => {}
                // Game ended, no more anchors.
                Err(UpdateGameError::AlreadyEnded) => break 'calculate_anchors,
            }

            inputs_loaded += 1;
        }

        // Game was forfeit before anchor.
        if let Some(forfeit_time) = game_restoration_data.forfeit
            && forfeit_time <= next_anchor_time
        {
            break 'calculate_anchors;
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
            game: game.try_clone().unwrap(),
            inputs_loaded,
        });

        next_anchor_time = game.state().time + anchor_interval;

        // Stop generation.
        if next_anchor_time > replay_length {
            break 'calculate_anchors;
        }
    }

    Ok((initial_game, Some(game_save_anchors)))
}
