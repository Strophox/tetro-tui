use std::{
    io::{self, Write},
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event,
    style::Print,
    terminal::{self, Clear, ClearType},
    ExecutableCommand,
};
use tetrs_engine::{Button, ButtonChange, Feedback, Game, UpdateGameError};

use crate::{
    application::{
        Application, ButtonInputHistory, GameMetaData, GameRestorationData, Menu, MenuUpdate,
        ScoreboardEntry,
    },
    game_input_handlers::{live_terminal::LiveTerminalInputHandler, InputSignal},
    game_renderers::Renderer,
};

impl<T: Write> Application<T> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::application) fn menu_play_game(
        &mut self,
        game: &mut Game,
        game_meta_data: &mut GameMetaData,
        time_started: &Instant,
        time_last_paused: &mut Instant,
        duration_paused_total: &mut Duration,
        button_input_history: &mut ButtonInputHistory,
        game_renderer: &mut impl Renderer,
    ) -> io::Result<MenuUpdate> {
        if self.runtime_data.kitty_assumed {
            // FIXME: Kinda iffy. Do we need all flags? What undesirable effects might there be?
            let _ = self.term.execute(event::PushKeyboardEnhancementFlags(
                event::KeyboardEnhancementFlags::all(),
            ));
        }

        // Prepare channel with which to communicate `Button` inputs / game interrupt.
        let (button_sender, button_receiver) = mpsc::channel();

        let _input_handler = LiveTerminalInputHandler::new(
            &button_sender,
            self.settings.keybinds(),
            self.runtime_data.kitty_assumed,
        );

        // FIXME: Combo Bot.
        // let mut combo_bot_handler = (self.runtime_data.combo_bot_enabled
        //     && game_meta_data.title == "Combo")
        //     .then(|| ComboBotInputHandler::new(&button_sender, Duration::from_millis(100)));
        // let mut inform_combo_bot = |game: &Game, evts: &FeedbackMessages| {
        //     if let Some((_, state_sender)) = &mut combo_bot_handler {
        //         if evts
        //             .iter()
        //             .any(|(_, feedback)| matches!(feedback, Feedback::PieceSpawned(_)))
        //         {
        //             let combo_state = ComboBotInputHandler::encode(game).unwrap();
        //             if state_sender.send(combo_state).is_err() {
        //                 combo_bot_handler = None;
        //             }
        //         }
        //     }
        // };

        // Game Loop
        let session_resumed = Instant::now();
        *duration_paused_total += session_resumed.saturating_duration_since(*time_last_paused);
        let mut clean_screen = true;
        let mut f = 0u32;
        let mut fps_counter = 0;
        let mut fps_counter_started = Instant::now();
        let menu_update = 'play_game: loop {
            // Exit if game ended
            if let Some(game_result) = game.result() {
                let scoreboard_entry = ScoreboardEntry::new(game, game_meta_data);
                let game_restoration_data = GameRestorationData::new(game, button_input_history);
                self.scoreboard
                    .entries
                    .push((scoreboard_entry.clone(), Some(game_restoration_data)));
                let menu = if game_result.is_ok() {
                    Menu::GameComplete
                } else {
                    Menu::GameOver
                }(Box::new(scoreboard_entry));
                break 'play_game MenuUpdate::Push(menu);
            }

            // Start next frame
            f += 1;
            fps_counter += 1;
            // TODO(Strophox): What?
            let next_frame_at = loop {
                let frame_at = session_resumed
                    + Duration::from_secs_f64(f64::from(f) / self.settings.graphics().game_fps);
                if frame_at < Instant::now() {
                    f += 1;
                } else {
                    break frame_at;
                }
            };

            let mut new_feedback_msgs = Vec::new();

            'frame_idle: loop {
                let frame_idle_remaining = next_frame_at - Instant::now();
                match button_receiver.recv_timeout(frame_idle_remaining) {
                    Ok(InputSignal::AbortProgram) => {
                        break 'play_game MenuUpdate::Push(Menu::Quit(
                            "exited with ctrl-c".to_owned(),
                        ));
                    }

                    Ok(InputSignal::ForfeitGame) => {
                        game.forfeit();
                        let scoreboard_entry = ScoreboardEntry::new(game, game_meta_data);
                        let game_restoration_data =
                            GameRestorationData::new(game, button_input_history);
                        self.scoreboard
                            .entries
                            .push((scoreboard_entry.clone(), Some(game_restoration_data)));
                        break 'play_game MenuUpdate::Push(Menu::GameOver(Box::new(
                            scoreboard_entry,
                        )));
                    }

                    Ok(InputSignal::Pause) => {
                        *time_last_paused = Instant::now();
                        break 'play_game MenuUpdate::Push(Menu::Pause);
                    }

                    Ok(InputSignal::WindowResize) => {
                        clean_screen = true;
                        continue 'frame_idle;
                    }

                    Ok(InputSignal::StoreSavepoint) => {
                        let _ = self.game_savepoint.insert((
                            game_meta_data.clone(),
                            GameRestorationData::new(game, button_input_history),
                            button_input_history.0.len(),
                        ));
                        new_feedback_msgs.push((
                            game.state().time,
                            Feedback::Text("(Savepoint stored!)".to_owned()),
                        ));
                    }

                    Ok(InputSignal::StoreSeed) => {
                        let _ = self
                            .settings
                            .new_game
                            .custom_seed
                            .insert(game.init_vals().seed);
                        new_feedback_msgs.push((
                            game.state().time,
                            Feedback::Text(format!("(Seed stored: {})", game.init_vals().seed)),
                        ));
                    }

                    Ok(InputSignal::Blindfold) => {
                        self.settings.graphics_mut().blindfolded ^= true;
                        if self.settings.graphics().blindfolded {
                            new_feedback_msgs.push((
                                game.state().time,
                                Feedback::Text("Blindfolded! [Ctrl+Shift+B]".to_owned()),
                            ));
                        } else {
                            new_feedback_msgs.push((
                                game.state().time,
                                Feedback::Text("Blindfolds removed! [Ctrl+Shift+B]".to_owned()),
                            ));
                        }
                    }

                    Ok(InputSignal::ButtonInput(button_change, instant)) => {
                        let game_time_userinput = instant.saturating_duration_since(*time_started)
                            - *duration_paused_total;
                        // Guarantee update cannot fail because of past input; input user as quickly as possible if it *was* in the (hopefully not so distant) past.
                        let update_target_time =
                            std::cmp::max(game_time_userinput, game.state().time);

                        let result = game.update(update_target_time, Some(button_change));

                        button_input_history.0.push(ButtonInputHistory::encode(
                            update_target_time,
                            button_change,
                        ));

                        // FIXME: Combo Bot.
                        // inform_combo_bot(game, &evts);

                        match result {
                            Ok(msgs) => new_feedback_msgs.extend(msgs),
                            Err(UpdateGameError::GameEnded) => break 'frame_idle,
                            Err(UpdateGameError::TargetTimeInPast) => unreachable!(),
                        }
                    }

                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let update_target_time = Instant::now()
                            .saturating_duration_since(*time_started)
                            - *duration_paused_total;

                        let result = game.update(update_target_time, None);

                        // FIXME: Combo Bot.
                        // inform_combo_bot(game, &evts);

                        if let Ok(msgs) = result {
                            new_feedback_msgs.extend(msgs);
                        }

                        break 'frame_idle;
                    }

                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // NOTE: We kind of rely on this not happening too often.
                        break 'play_game MenuUpdate::Push(Menu::Pause);
                    }
                };
            }
            game_renderer.render(self, game, game_meta_data, new_feedback_msgs, clean_screen)?;
            clean_screen = false;
            // FPS counter.
            if self.settings.graphics().show_fps {
                let now = Instant::now();
                if now.saturating_duration_since(fps_counter_started) >= Duration::from_secs(1) {
                    self.term
                        .execute(MoveTo(0, 0))?
                        .execute(Print(format!("{:_>6}", format!("{fps_counter}fps"))))?;
                    fps_counter = 0;
                    fps_counter_started = now;
                }
            }
        };

        // Console epilogue: De-initialization.
        if self.runtime_data.kitty_assumed {
            let _ = self.term.execute(event::PopKeyboardEnhancementFlags);
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
            // Game not done = we're pausing.
            // Manually release any pressed buttons for safety when pausing.
            let mut to_unpress = Vec::new();
            for (is_pressed, button) in game.state().buttons_pressed.iter().zip(Button::VARIANTS) {
                if is_pressed.is_some() {
                    to_unpress.push(button);
                }
            }
            for button in to_unpress {
                let _ = game.update(game.state().time, Some(ButtonChange::Release(button)));
            }
        }

        Ok(menu_update)
    }
}
