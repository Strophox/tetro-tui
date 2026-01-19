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
use tetrs_engine::{Feedback, FeedbackMessages, Game, PressedButtons};

use crate::{
    application::{
        Application, GameMetaData, GameRestorationData, Menu, MenuUpdate, RecordedUserInput,
        ScoreboardEntry,
    },
    game_input_handlers::{
        combo_bot::ComboBotInputHandler, terminal::TerminalInputHandler, InputSignal,
    },
    game_renderers::Renderer,
    utils::{encode_board, encode_buttons},
};

impl<T: Write> Application<T> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::application) fn menu_game(
        &mut self,
        game: &mut Game,
        meta_data: &mut GameMetaData,
        time_started: &Instant,
        time_last_paused: &mut Instant,
        total_pause_duration: &mut Duration,
        recorded_user_input: &mut RecordedUserInput,
        game_renderer: &mut impl Renderer,
    ) -> io::Result<MenuUpdate> {
        if self.runtime_data.kitty_assumed {
            // FIXME: Kinda iffy. Do we need all flags? What undesirable effects might there be?
            let _ = self.term.execute(event::PushKeyboardEnhancementFlags(
                event::KeyboardEnhancementFlags::all(),
                // event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
            ));
        }
        // Prepare channel with which to communicate `Button` inputs / game interrupt.
        let mut buttons_pressed = PressedButtons::default();
        let (button_sender, button_receiver) = mpsc::channel();
        let _input_handler = TerminalInputHandler::new(
            &button_sender,
            self.settings.keybinds(),
            self.runtime_data.kitty_assumed,
        );
        let mut combo_bot_handler = (self.runtime_data.combo_bot_enabled
            && meta_data.title == "Combo")
            .then(|| ComboBotInputHandler::new(&button_sender, Duration::from_millis(100)));
        let mut inform_combo_bot = |game: &Game, evts: &FeedbackMessages| {
            if let Some((_, state_sender)) = &mut combo_bot_handler {
                if evts
                    .iter()
                    .any(|(_, feedback)| matches!(feedback, Feedback::PieceSpawned(_)))
                {
                    let combo_state = ComboBotInputHandler::encode(game).unwrap();
                    if state_sender.send(combo_state).is_err() {
                        combo_bot_handler = None;
                    }
                }
            }
        };
        // Game Loop
        let session_resumed = Instant::now();
        *total_pause_duration += session_resumed.saturating_duration_since(*time_last_paused);
        let mut clean_screen = true;
        let mut f = 0u32;
        let mut fps_counter = 0;
        let mut fps_counter_started = Instant::now();
        let menu_update = 'render: loop {
            // Exit if game ended
            if let Some(game_result) = game.state().result {
                let scoreboard_entry = ScoreboardEntry::new(game, meta_data);
                let game_restoration_data = GameRestorationData::new(game, recorded_user_input);
                self.scoreboard
                    .entries
                    .push((scoreboard_entry.clone(), Some(game_restoration_data)));
                let menu = if game_result.is_ok() {
                    Menu::GameComplete
                } else {
                    Menu::GameOver
                }(Box::new(scoreboard_entry));
                break 'render MenuUpdate::Push(menu);
            }
            // Start next frame
            f += 1;
            fps_counter += 1;
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
                        break 'render MenuUpdate::Push(Menu::Quit(
                            "exited with ctrl-c".to_owned(),
                        ));
                    }
                    Ok(InputSignal::ForfeitGame) => {
                        game.forfeit();
                        let scoreboard_entry = ScoreboardEntry::new(game, meta_data);
                        let game_restoration_data =
                            GameRestorationData::new(game, recorded_user_input);
                        self.scoreboard
                            .entries
                            .push((scoreboard_entry.clone(), Some(game_restoration_data)));
                        break 'render MenuUpdate::Push(Menu::GameOver(Box::new(scoreboard_entry)));
                    }
                    Ok(InputSignal::Pause) => {
                        *time_last_paused = Instant::now();
                        break 'render MenuUpdate::Push(Menu::Pause);
                    }
                    Ok(InputSignal::WindowResize) => {
                        clean_screen = true;
                        continue 'frame_idle;
                    }
                    Ok(InputSignal::StoreSavepoint) => {
                        let _ = self.savepoint.insert((
                            meta_data.clone(),
                            GameRestorationData::new(game, recorded_user_input),
                        ));
                        new_feedback_msgs.push((
                            game.state().time,
                            Feedback::Text("(Savepoint captured.)".to_owned()),
                        ));
                    }
                    Ok(InputSignal::StoreSeed) => {
                        let _ = self.new_game_settings.custom_seed.insert(game.seed());
                        new_feedback_msgs.push((
                            game.state().time,
                            Feedback::Text("(Seed captured.)".to_owned()),
                        ));
                    }
                    Ok(InputSignal::StoreBoard) => {
                        let _ = self
                            .new_game_settings
                            .custom_board
                            .insert(encode_board(&game.state().board));
                        new_feedback_msgs.push((
                            game.state().time,
                            Feedback::Text("(Board captured.)".to_owned()),
                        ));
                    }
                    Ok(InputSignal::ButtonInput(button, button_state, instant)) => {
                        buttons_pressed[button] = button_state;
                        let game_time_userinput = instant.saturating_duration_since(*time_started)
                            - *total_pause_duration;
                        let game_now = std::cmp::max(game_time_userinput, game.state().time);
                        recorded_user_input.push((game_now, encode_buttons(&buttons_pressed)));
                        // FIXME: Handle error?
                        if let Ok(evts) = game.update(Some(buttons_pressed), game_now) {
                            inform_combo_bot(game, &evts);
                            new_feedback_msgs.extend(evts);
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let game_time_now = Instant::now().saturating_duration_since(*time_started)
                            - *total_pause_duration;
                        // FIXME: Handle error?
                        if let Ok(evts) = game.update(None, game_time_now) {
                            inform_combo_bot(game, &evts);
                            new_feedback_msgs.extend(evts);
                        }
                        break 'frame_idle;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // NOTE: We kind of rely on this not happening too often.
                        break 'render MenuUpdate::Push(Menu::Pause);
                    }
                };
            }
            game_renderer.render(self, game, meta_data, new_feedback_msgs, clean_screen)?;
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
        if let Some(finished_state) = game.state().result {
            let h_console = terminal::size()?.1;
            if finished_state.is_ok() {
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
        }
        Ok(menu_update)
    }
}
