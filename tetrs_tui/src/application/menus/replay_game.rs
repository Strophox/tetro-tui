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
        Application, GameInputHistory, GameMetaData,
        GameRestorationData, Menu, MenuUpdate,
    },
    fmt_helpers::replay_keybinds_legend,
    game_renderers::Renderer,
    live_input_handler::{self, LiveTermSignal},
};

impl<T: Write> Application<T> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::application) fn run_menu_replay_game(
        &mut self,
        game: &mut Game,
        game_meta_data: &mut GameMetaData,
        game_input_history: &mut GameInputHistory,
        game_renderer: &mut impl Renderer,
    ) -> io::Result<MenuUpdate> {/*
        // Prepare everything to enter the game (react & render) loop.

        let keybinds_legend = replay_keybinds_legend();
        let replay_length = game_input_history
            .last()
            .clone()
            .map(|x| x.0)
            .unwrap_or_default();

        // Prepare channel from which to receive terminal inputs.
        let (input_sender, input_receiver) = mpsc::channel();

        // Spawn input handler thread.
        let _join_handle =
            live_input_handler::spawn(input_sender, Default::default());

        // Game Loop

        let session_resumed = Instant::now();
        *total_pause_duration += session_resumed.saturating_duration_since(*time_last_paused);

        let mut render_id = 0u32;

        let mut renders_per_second_counter = 0;
        let mut renders_per_second_counter_start_time = Instant::now();

        // Explicitly tells the renderer if entire screen needs to be re-drawn once.
        let mut refresh_entire_view = true;

        // Replay-specific data.
        let mut update_id = 0u32;
        let mut load_offset = 0usize;
        let mut is_paused = false;
        let mut replay_speed = 1.0f64;
        let mut anchor_saves: Vec<(Game, u32)> = Vec::new();

        let menu_update = 'render_and_input: loop {
            // Start new iteration of [render->input->] loop.

            // Render current state of the game.
            game_renderer.render(
                game,
                game_meta_data,
                &self.settings,
                &keybinds_legend,
                Some(replay_length),
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

            if let Some(game_result) = game.result() {
                // Game ended, cannot actually continue playing;
                is_paused = true;

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

            'frame_idle: loop {
                // Compute time left until we should stop waiting.
                let frame_idle_remaining = next_render_at - Instant::now();

                let recv_result = input_receiver.recv_timeout(frame_idle_remaining);

                match recv_result {
                    Ok((signal, timestamp)) => {
                        match signal {
                            // Found a recognized game input: DON'T use it.
                            LiveTermSignal::RecognizedButton(..) => {}

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
                                            // For the remaining controls we only care about non- key-releases.
                                            continue 'frame_idle;
                                        }

                                        match (code, modifiers) {
                                            // [Esc]: Stop.
                                            (KeyCode::Esc, _) => {
                                                break 'render_and_input MenuUpdate::Pop;
                                            }

                                            // [Ctrl+C]: Abort program.
                                            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                                                break 'render_and_input MenuUpdate::Push(
                                                    Menu::Quit,
                                                );
                                            }

                                            // [Enter]: Play game.
                                            (KeyCode::Enter, _) => {
                                                let now = Instant::now();
                                                // We take out the exact game. Leave some dummy in its place that shouldn't be used/relevant...
                                                let the_game = std::mem::replace(game, Game::builder().build());
                                                break 'render_and_input MenuUpdate::Push(Menu::PlayGame {
                                                    game: Box::new(the_game),
                                                    meta_data: game_meta_data.clone(),
                                                    timestamp_play_started: now - game.state().time,
                                                    last_paused: now,
                                                    total_pause_duration: Duration::ZERO,
                                                    game_input_history: game_input_history.iter().take(load_offset).cloned().collect(),
                                                    game_renderer: Default::default(),
                                                });
                                            }

                                            // [Space]: (Un-)Pause replay.
                                            (KeyCode::Char(' '), _) => {
                                                is_paused ^= true;
                                            }

                                            // [↓][↑]: Adjust replay speed.
                                            (KeyCode::Down | KeyCode::Up, _) => {
                                                if code == KeyCode::Up {
                                                    replay_speed += 0.1;
                                                } else if replay_speed > 0.1 {
                                                    replay_speed -= 0.1;
                                                };
                                            }

                                            // [←][→]: Skip to anchor save.
                                            (KeyCode::Left | KeyCode::Right, _) => {
                                                
                                            }

                                            // [,][.]: Skip to game update. Pauses replay.
                                            (KeyCode::Char(',') | KeyCode::Char('.'), _) => {

                                                
                                                is_paused = true;
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

                            // Input handler thread died... Pop game for now.
                            mpsc::RecvTimeoutError::Disconnected => {
                                break 'render_and_input MenuUpdate::Pop;
                            }
                        }
                    }
                }
            }
        };

        Ok(menu_update)
    */todo!()}
}
