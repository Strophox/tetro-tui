/*!
This module handles what happens when [`Game::update`] is called.
*/

use super::*;

impl Game {
    /// The main function used to advance the game state.
    ///
    /// This will cause an internal update of all [`GameEvent`]s up to and including the given
    /// `update_time` requested.
    /// If `new_button_state.is_some()` then the same thing happens, except that the very last
    /// 'event' will be the change of [`ButtonsPressed`] at `update_time` (which might cause some
    /// further events that are handled at `update_time` before finally returning).
    ///
    /// Unless an error occurs, this function will return all [`FeedbackMessages`] caused between the
    /// previous and the current `update` call, in chronological order.
    ///
    /// # Errors
    ///
    /// This function may error with:
    /// - [`GameUpdateError::GameEnded`] if `game.ended()` is `true`, indicating that no more updates
    ///   can change the game state, or
    /// - [`GameUpdateError::DurationPassed`] if `update_time < game.state().time`, indicating that
    ///   the requested update lies in the past.
    pub fn update(
        &mut self,
        mut new_button_state: Option<PressedButtons>,
        update_time: GameTime,
    ) -> Result<FeedbackMessages, UpdateGameError> {
        /*
        Order:
        - if game already ended, return immediately
        * find next event
        - event less-or-equal update point:
            - allow modifiers
            - handle event
            - allow modifiers
            - update game end state, possibly return immediately
            - goto *
        - update point reached:
            - try adding input events, goto *
            - else return immediately
         */
        // Invalid call: return immediately.
        if update_time < self.state.time {
            return Err(UpdateGameError::DurationPassed);
        }
        if self.ended() {
            return Err(UpdateGameError::GameEnded);
        };
        // NOTE: Returning an empty Vec is efficient because it won't even allocate (as by Rust API).
        let mut feedback_msgs = Vec::new();
        self.run_modifier_updates(&mut feedback_msgs, &ModificationPoint::UpdateStart);
        // We linearly process all events until we reach the update time.
        'event_simulation: loop {
            // Peek the next closest event.
            // SAFETY: `Game` invariants guarantee there's some event.
            let next_event = self
                .state
                .events
                .iter()
                .min_by_key(|(&event, &event_time)| (event_time, event));
            match next_event {
                // Next event within requested update time, handle event first.
                Some((&event, &event_time)) if event_time <= update_time => {
                    self.run_modifier_updates(
                        &mut feedback_msgs,
                        &ModificationPoint::BeforeEvent(event),
                    );
                    // Remove next event and handle it.
                    self.state.events.remove_entry(&event);
                    if self.config.feedback_verbosity == FeedbackVerbosity::Debug {
                        feedback_msgs.push((event_time, Feedback::EngineEvent(event)));
                    }
                    let event_feedback_msgs = self.handle_event(event, event_time);
                    self.state.time = event_time;
                    feedback_msgs.extend(event_feedback_msgs);
                    self.run_modifier_updates(
                        &mut feedback_msgs,
                        &ModificationPoint::AfterEvent(event),
                    );
                    // Stop simulation early if event or modifier ended game.
                    self.run_game_result_update();
                    if self.ended() {
                        break 'event_simulation;
                    }
                }
                // FIXME(Strophox): Are we not 'unintentionally' catching the `None` case too?
                _ => {
                    // Possibly process user input events now or break out.
                    // NOTE: We should be able to update the time here because `self.process_input(...)` does not access it.
                    self.state.time = update_time;
                    // Update button inputs.
                    if let Some(pressed_buttons) = new_button_state.take() {
                        self.run_modifier_updates(
                            &mut feedback_msgs,
                            &ModificationPoint::BeforeInput,
                        );
                        if self.config.feedback_verbosity == FeedbackVerbosity::Debug {
                            feedback_msgs.push((
                                update_time,
                                Feedback::EngineInput(
                                    self.state.buttons_pressed.map(|x| x.is_some()),
                                    pressed_buttons,
                                ),
                            ));
                        }
                        self.run_input_update(pressed_buttons, update_time);
                        self.run_modifier_updates(
                            &mut feedback_msgs,
                            &ModificationPoint::AfterInput,
                        );
                    } else {
                        self.run_game_result_update();
                        break 'event_simulation;
                    }
                }
            };
        }
        Ok(feedback_msgs)
    }
    /// Updates the internal `self.state.end` state, checking whether any [`Limits`] have been reached.
    fn run_game_result_update(&mut self) {
        if self.state.result.is_some() {
            return;
        }
        self.state.result = self.rules.end_conditions.iter().find_map(|(c, good)| {
            self.check_stat_met(c).then_some(if *good {
                Ok(())
            } else {
                Err(GameOver::ModeLimit)
            })
        });
    }

    /// Goes through all internal 'game mods' and applies them sequentially at the given [`ModifierPoint`].
    fn run_modifier_updates(
        &mut self,
        feedback_msgs: &mut FeedbackMessages,
        modifier_point: &ModificationPoint,
    ) {
        for modifier in &mut self.modifiers {
            (modifier.mod_function)(
                &mut self.config,
                &mut self.rules,
                &mut self.state,
                modifier_point,
                feedback_msgs,
            );
        }
    }

    /// Computes and adds to the internal event queue any relevant [`GameEvent`]s caused by the
    /// player in form of a change of button states.
    #[allow(clippy::bool_comparison, clippy::comparison_chain)]
    fn run_input_update(&mut self, pressed_new: PressedButtons, update_time: GameTime) {
        let pressed_old = self.state.buttons_pressed.map(|x| x.is_some());
        if self.state.active_piece_data.is_some() {
            let [ml0, mr0, rl0, rr0, ra0, ds0, dh0, dc0, h0] = pressed_old;
            let [ml1, mr1, rl1, rr1, ra1, ds1, dh1, dc1, h1] = pressed_new;

            // Single new button has been pressed, remove repeat moves and add initial move.
            /*
            Karnaugh map:
            |           !mR0 !mR0  mR0  mR0
            |           !mR1  mR1  mR1 !mR1
            | !mL0 !mL1   -    r    -    0
            | !mL0  mL1   l    -    l    l
            |  mL0  mL1   -    r    -    l?
            |  mL0 !mL1   0    r    r?   0
            */
            let one_m_pressed = (ml0 < ml1) != (mr0 < mr1);
            let revert_m_left = (ml0 && ml1)
                && (mr0 > mr1)
                && self.state.buttons_pressed[Button::MoveLeft]
                    < self.state.buttons_pressed[Button::MoveRight];
            let revert_m_right = (mr0 && mr1)
                && (ml0 > ml1)
                && self.state.buttons_pressed[Button::MoveRight]
                    < self.state.buttons_pressed[Button::MoveLeft];
            let any_m_unpressed = (ml0 || mr0) && (!ml1 && !mr1);
            if one_m_pressed || revert_m_left || revert_m_right {
                self.state.events.remove(&GameEvent::Move(false));
                self.state.events.insert(GameEvent::Move(true), update_time);
            // Both move buttons unpressed, remove repeat moves.
            } else if any_m_unpressed {
                self.state.events.remove(&GameEvent::Move(false));
            }
            // Count number of turns using newly pressed rotation buttons.
            let mut turns = 0;
            if rl0 < rl1 {
                turns -= 1;
            }
            if rr0 < rr1 {
                turns += 1;
            }
            if ra0 < ra1 {
                turns += 2;
            }
            if turns != 0 {
                self.state
                    .events
                    .insert(GameEvent::Rotate(turns), update_time);
            }
            // Soft drop button pressed, add event.
            if ds0 < ds1 {
                self.state.events.insert(GameEvent::SoftDrop, update_time);
            // Soft drop button released: Reset fall timer.
            } else if ds0 > ds1 {
                if let Ok(level) = NonZeroU32::try_from(self.state.gravity) {
                    self.state
                        .events
                        .insert(GameEvent::Fall, update_time + Self::drop_delay(level, None));
                }
            }
            // Hard drop button pressed.
            if dh0 < dh1 {
                self.state.events.insert(GameEvent::HardDrop, update_time);
            }
            // Sonic drop button pressed
            if dc0 < dc1 {
                self.state.events.insert(GameEvent::SonicDrop, update_time);
            }
            // Hold button pressed
            if h0 < h1 {
                self.state.events.insert(GameEvent::Hold, update_time);
            }
        }
        // Update internal button state.
        #[allow(clippy::bool_comparison)]
        for ((button, old), new) in Button::VARIANTS.iter().zip(pressed_old).zip(pressed_new) {
            if old < new {
                self.state.buttons_pressed[*button] = Some(update_time);
            } else if old > new {
                self.state.buttons_pressed[*button] = None;
            }
        }
    }

    /// Try holding a tetromino in the game state and report success.
    fn attempt_hold(&mut self, tetromino: Tetromino, event_time: GameTime) -> bool {
        match self.state.hold_piece {
            None | Some((_, true)) => {
                if let Some((held_piece, _)) = self.state.hold_piece {
                    self.state.next_pieces.push_front(held_piece);
                } else {
                    self.state.next_pieces.extend(
                        self.config
                            .tetromino_generator
                            .with_rng(&mut self.state.rng)
                            .take(1),
                    );
                }
                self.state.hold_piece = Some((tetromino, false));
                self.state.events.clear();
                self.state.events.insert(GameEvent::Spawn, event_time);
                true
            }
            _ => false,
        }
    }

    /// Given an event, update the internal game state, possibly adding new future events.
    ///
    /// This function is likely the most important part of a game update as it handles the logic of
    /// spawning, dropping, moving, locking the active piece, etc.
    /// It also returns some feedback events caused by clearing lines, locking the piece, etc.
    fn handle_event(&mut self, event: GameEvent, event_time: GameTime) -> FeedbackMessages {
        // Active piece touches the ground before update (or doesn't exist, counts as not touching).
        let mut feedback_msgs = Vec::new();
        let prev_piece_data = self.state.active_piece_data;
        let prev_piece = prev_piece_data.unzip().0;
        let next_piece = match event {
            // We generate a new piece above the skyline, and immediately queue a fall event for it.
            GameEvent::Spawn => {
                debug_assert!(
                    prev_piece.is_none(),
                    "spawning event but an active piece is still in play"
                );
                let tetromino = self.state.next_pieces.pop_front().unwrap_or_else(|| {
                    self.config
                        .tetromino_generator
                        .with_rng(&mut self.state.rng)
                        .next()
                        .expect("piece generator ran out before game finished")
                });
                self.state.next_pieces.extend(
                    self.config
                        .tetromino_generator
                        .with_rng(&mut self.state.rng)
                        .take(
                            self.config
                                .preview_count
                                .saturating_sub(self.state.next_pieces.len()),
                        ),
                );
                // Initial Hold System.
                if self.state.buttons_pressed[Button::HoldPiece].is_some()
                    && self.attempt_hold(tetromino, event_time)
                {
                    None
                } else {
                    let pos = match tetromino {
                        Tetromino::O => (4, 20),
                        _ => (3, 20),
                    };
                    let orientation = Orientation::N;
                    let original_piece = ActivePiece {
                        shape: tetromino,
                        orientation,
                        position: pos,
                    };
                    let mut turns = 0;
                    if self.state.buttons_pressed[Button::RotateRight].is_some() {
                        turns += 1;
                    }
                    if self.state.buttons_pressed[Button::RotateAround].is_some() {
                        turns += 2;
                    }
                    if self.state.buttons_pressed[Button::RotateLeft].is_some() {
                        turns -= 1;
                    }
                    // Initial Rotation system.
                    let next_piece = self
                        .config
                        .rotation_system
                        .rotate(&original_piece, &self.state.board, turns)
                        .unwrap_or(original_piece);
                    if self.config.feedback_verbosity != FeedbackVerbosity::Quiet {
                        feedback_msgs.push((event_time, Feedback::PieceSpawned(next_piece)));
                    }
                    // Newly spawned piece conflicts with board - Game over.
                    if !next_piece.fits(&self.state.board) {
                        self.state.result = Some(Err(GameOver::BlockOut));
                        return feedback_msgs;
                    }
                    self.state.events.insert(GameEvent::Fall, event_time);
                    Some(next_piece)
                }
            }
            GameEvent::Hold => {
                let prev_piece = prev_piece.expect("hold piece event but no active piece");
                if self.attempt_hold(prev_piece.shape, event_time) {
                    None
                } else {
                    Some(prev_piece)
                }
            }
            GameEvent::Rotate(turns) => {
                let prev_piece = prev_piece.expect("rotate event but no active piece");
                self.config
                    .rotation_system
                    .rotate(&prev_piece, &self.state.board, turns)
                    .or(Some(prev_piece))
            }
            GameEvent::Move(is_initial) => {
                // Handle move attempt and auto repeat move.
                let prev_piece = prev_piece.expect("move event but no active piece");
                let dx = match (
                    self.state.buttons_pressed[Button::MoveLeft],
                    self.state.buttons_pressed[Button::MoveRight],
                ) {
                    (Some(t_left), Some(t_right)) => {
                        if t_left < t_right {
                            1
                        } else {
                            -1
                        }
                    }
                    (left, right) => {
                        if left < right {
                            1
                        } else {
                            -1
                        }
                    }
                };
                Some(
                    if let Some(next_piece) = prev_piece.fits_at(&self.state.board, (dx, 0)) {
                        let mut move_delay = if is_initial {
                            self.config.delayed_auto_shift
                        } else {
                            self.config.auto_repeat_rate
                        };
                        if let Ok(level) = NonZeroU32::try_from(self.state.gravity) {
                            move_delay = move_delay.min(
                                Self::lock_delay(level).saturating_sub(Duration::from_millis(1)),
                            );
                        }
                        self.state
                            .events
                            .insert(GameEvent::Move(false), event_time + move_delay);
                        next_piece
                    } else {
                        prev_piece
                    },
                )
            }
            GameEvent::Fall => {
                let prev_piece = prev_piece.expect("falling event but no active piece");
                // Try to drop active piece down by one, and queue next fall event.
                Some(
                    if let Some(dropped_piece) = prev_piece.fits_at(&self.state.board, (0, -1)) {
                        // Drop delay is possibly faster due to soft drop button pressed.
                        let soft_drop = self.state.buttons_pressed[Button::DropSoft]
                            .map(|_| self.config.soft_drop_factor);
                        if let Ok(level) = NonZeroU32::try_from(self.state.gravity) {
                            let drop_delay = Self::drop_delay(level, soft_drop);
                            self.state
                                .events
                                .insert(GameEvent::Fall, event_time + drop_delay);
                        }
                        dropped_piece
                    } else {
                        // Otherwise piece could not move down.
                        prev_piece
                    },
                )
            }
            GameEvent::SoftDrop => {
                let prev_piece = prev_piece.expect("softdrop event but no active piece");
                // Try to drop active piece down by one, and queue next fall event.
                Some(
                    if let Some(dropped_piece) = prev_piece.fits_at(&self.state.board, (0, -1)) {
                        let soft_drop = self.state.buttons_pressed[Button::DropSoft]
                            .map(|_| self.config.soft_drop_factor);
                        if let Ok(level) = NonZeroU32::try_from(self.state.gravity) {
                            let drop_delay = Self::drop_delay(level, soft_drop);
                            self.state
                                .events
                                .insert(GameEvent::Fall, event_time + drop_delay);
                        }
                        dropped_piece
                    } else {
                        // Otherwise piece was not able to move down.
                        // Immediately queue lock.
                        self.state.events.insert(GameEvent::LockTimer, event_time);
                        prev_piece
                    },
                )
            }
            GameEvent::SonicDrop => {
                let prev_piece = prev_piece.expect("sonicdrop event but no active piece");
                // Move piece all the way down and nothing more.
                Some(prev_piece.well_piece(&self.state.board))
            }
            GameEvent::HardDrop => {
                let prev_piece = prev_piece.expect("harddrop event but no active piece");
                // Move piece all the way down.
                let dropped_piece = prev_piece.well_piece(&self.state.board);
                if self.config.feedback_verbosity != FeedbackVerbosity::Quiet {
                    feedback_msgs.push((event_time, Feedback::HardDrop(prev_piece, dropped_piece)));
                }
                self.state.events.insert(
                    GameEvent::LockTimer,
                    event_time + self.config.hard_drop_delay,
                );
                Some(dropped_piece)
            }
            GameEvent::LockTimer => {
                self.state.events.insert(GameEvent::Lock, event_time);
                prev_piece
            }
            GameEvent::Lock => {
                let prev_piece = prev_piece.expect("lock event but no active piece");
                if self.config.feedback_verbosity != FeedbackVerbosity::Quiet {
                    feedback_msgs.push((event_time, Feedback::PieceLocked(prev_piece)));
                }
                // Attempt to lock active piece fully above skyline - Game over.
                if prev_piece
                    .tiles()
                    .iter()
                    .all(|((_, y), _)| *y >= Game::SKYLINE)
                {
                    self.state.result = Some(Err(GameOver::LockOut));
                    return feedback_msgs;
                }
                self.state.pieces_locked[prev_piece.shape] += 1;
                // Pre-save whether piece was spun into lock position.
                let is_spin = prev_piece.fits_at(&self.state.board, (0, 1)).is_none();
                // Locking.
                for ((x, y), tile_type_id) in prev_piece.tiles() {
                    self.state.board[y][x] = Some(tile_type_id);
                }
                // Handle line clear counting for score (only do actual clearing in LineClear).
                let mut lines_cleared = Vec::<usize>::with_capacity(4);
                for y in (0..Game::HEIGHT).rev() {
                    if self.state.board[y].iter().all(|mino| mino.is_some()) {
                        lines_cleared.push(y);
                    }
                }
                let n_lines_cleared = u32::try_from(lines_cleared.len()).unwrap();
                if n_lines_cleared == 0 {
                    self.state.consecutive_line_clears = 0;
                } else {
                    self.state.consecutive_line_clears += 1;
                    // Compute score bonus.
                    let n_combo = self.state.consecutive_line_clears;
                    let is_perfect_clear = self.state.board.iter().all(|line| {
                        line.iter().all(|tile| tile.is_none())
                            || line.iter().all(|tile| tile.is_some())
                    });
                    let score_bonus = n_lines_cleared
                        * if is_spin { 2 } else { 1 }
                        * if is_perfect_clear { 4 } else { 1 }
                        * 2
                        - 1
                        + (n_combo - 1);
                    self.state.score += u64::from(score_bonus);
                    let yippie = Feedback::Accolade {
                        score_bonus,
                        tetromino: prev_piece.shape,
                        is_spin,
                        lines_cleared: n_lines_cleared,
                        is_perfect_clear,
                        combo: n_combo,
                    };
                    if self.config.feedback_verbosity != FeedbackVerbosity::Quiet {
                        feedback_msgs.push((
                            event_time,
                            Feedback::LineClears(lines_cleared, self.config.line_clear_delay),
                        ));
                        feedback_msgs.push((event_time, yippie));
                    }
                }
                // Clear all events and only put in line clear / appearance delay.
                self.state.events.clear();
                if n_lines_cleared > 0 {
                    self.state.events.insert(
                        GameEvent::LineClear,
                        event_time + self.config.line_clear_delay,
                    );
                } else {
                    self.state
                        .events
                        .insert(GameEvent::Spawn, event_time + self.config.appearance_delay);
                }
                self.state.hold_piece = self
                    .state
                    .hold_piece
                    .map(|(held_piece, _swap_allowed)| (held_piece, true));
                None
            }
            GameEvent::LineClear => {
                for y in (0..Game::HEIGHT).rev() {
                    // Full line: move it to the cleared lines storage and push an empty line to the board.
                    if self.state.board[y].iter().all(|mino| mino.is_some()) {
                        self.state.board.remove(y);
                        self.state.lines_cleared += 1;
                        // Increment level if 10 lines cleared.
                        if self.rules.progressive_gravity && self.state.lines_cleared % 10 == 0 {
                            self.state.gravity = self.state.gravity.saturating_add(1);
                        }
                    }
                }
                while self.state.board.len() < Game::HEIGHT {
                    self.state.board.push(Line::default());
                }
                self.state
                    .events
                    .insert(GameEvent::Spawn, event_time + self.config.appearance_delay);
                None
            }
        };
        // Piece is different to before.
        if next_piece.is_some() && prev_piece != next_piece {
            // User wants to move in a direction but no move event scheduled; add a move event.
            if (self.state.buttons_pressed[Button::MoveLeft]
                != self.state.buttons_pressed[Button::MoveRight])
                && !self.state.events.contains_key(&GameEvent::Move(false))
            {
                self.state.events.insert(GameEvent::Move(false), event_time);
            }
            // No fall event scheduled but piece might be able to; add fall event.
            #[allow(clippy::map_entry)]
            if !self.state.events.contains_key(&GameEvent::Fall) {
                let soft_drop = self.state.buttons_pressed[Button::DropSoft]
                    .map(|_| self.config.soft_drop_factor);
                if let Ok(level) = NonZeroU32::try_from(self.state.gravity) {
                    let drop_delay = Self::drop_delay(level, soft_drop);
                    self.state
                        .events
                        .insert(GameEvent::Fall, event_time + drop_delay);
                }
            }
        }
        self.state.active_piece_data = next_piece.map(|next_piece| {
            (
                next_piece,
                self.calculate_locking_data(
                    event,
                    event_time,
                    prev_piece_data,
                    next_piece,
                    next_piece.fits_at(&self.state.board, (0, -1)).is_none(),
                ),
            )
        });
        feedback_msgs
    }

    // FIXME: This is really unexpectedly complicated code. This should be reconsidered or at least commented.
    /// Calculates the newest locking details for the main active piece.
    fn calculate_locking_data(
        &mut self,
        event: GameEvent,
        event_time: GameTime,
        prev_piece_data: Option<(ActivePiece, LockingData)>,
        next_piece: ActivePiece,
        touches_ground: bool,
    ) -> LockingData {
        let Ok(level) = NonZeroU32::try_from(self.state.gravity) else {
            // FIXME: This is just a placeholder(!)(?)
            return LockingData {
                touches_ground,
                last_touchdown: None,
                last_liftoff: None,
                ground_time_left: self.config.ground_time_max,
                lowest_y: Game::HEIGHT,
            };
        };
        /*
        Table (touches_ground):
        | ∅t0 !t1  :  [1] init locking data
        | ∅t0  t1  :  [3.1] init locking data, track touchdown etc., add LockTimer
        | !t0 !t1  :  [4]  -
        | !t0  t1  :  [3.2] track touchdown etc., add LockTimer
        |  t0 !t1  :  [2] track liftoff etc., RMV LockTimer
        |  t0  t1  :  [3.3] upon move/rot. add LockTimer
        */
        match (prev_piece_data, touches_ground) {
            // [1] Newly spawned piece does not touch ground.
            (None, false) => LockingData {
                touches_ground: false,
                last_touchdown: None,
                last_liftoff: Some(event_time),
                ground_time_left: self.config.ground_time_max,
                lowest_y: next_piece.position.1,
            },
            // [2] Active piece lifted off the ground.
            (Some((_prev_piece, prev_locking_data)), false) if prev_locking_data.touches_ground => {
                self.state.events.remove(&GameEvent::LockTimer);
                LockingData {
                    touches_ground: false,
                    last_liftoff: Some(event_time),
                    ..prev_locking_data
                }
            }
            // [3] A piece is on the ground. Complex update to locking values.
            (prev_piece_data, true) => {
                let next_locking_data = match prev_piece_data {
                    // If previous piece exists and next piece hasn't reached newest low (i.e. not a reset situation).
                    Some((_prev_piece, prev_locking_data))
                        if next_piece.position.1 >= prev_locking_data.lowest_y =>
                    {
                        // Previously touched ground already, just continue previous data.
                        if prev_locking_data.touches_ground {
                            prev_locking_data
                        } else {
                            // SAFETY: We know we have an active piece that didn't touch ground before, so it MUST have its last_liftoff set.
                            let last_liftoff = prev_locking_data.last_liftoff.unwrap();
                            match prev_locking_data.last_touchdown {
                                /*
                                * `(prev_piece_data, Some((next_piece, true))) = (prev_piece_data, next_piece_dat)` [[NEXT ON GROUND]]
                                * `Some((_prev_piece, prev_locking_data)) if !(next_piece.pos.1 < prev_locking_data.lowest_y) = prev_piece_data` [[ACTIVE EXISTED, NO HEIGHT RESET]]
                                * `!prev_locking_data.touches_ground` [[PREV NOT ON GROUND]]

                                last_TD    notouch    CLOSE touchnow  :  TD = prev_locking_data.last_touchdown
                                -------    notouch    CLOSE touchnow  :  TD = Some(event_time)
                                last_TD    notouch      far touchnow  :  ground_time_left -= prev_stuff...,  TD = Some(event_time)
                                -------    notouch      far touchnow  :  TD = Some(event_time)
                                */
                                // Piece was a afloat before with valid last touchdown as well.
                                Some(last_touchdown) => {
                                    let (last_touchdown, ground_time_left) = if event_time
                                        .saturating_sub(last_liftoff)
                                        <= 2 * Self::drop_delay(level, None)
                                    {
                                        (
                                            prev_locking_data.last_touchdown,
                                            prev_locking_data.ground_time_left,
                                        )
                                    } else {
                                        let elapsed_ground_time =
                                            last_liftoff.saturating_sub(last_touchdown);
                                        (
                                            Some(event_time),
                                            prev_locking_data
                                                .ground_time_left
                                                .saturating_sub(elapsed_ground_time),
                                        )
                                    };
                                    LockingData {
                                        touches_ground: true,
                                        last_touchdown,
                                        last_liftoff: None,
                                        ground_time_left,
                                        lowest_y: prev_locking_data.lowest_y,
                                    }
                                }
                                // Piece existed, was not touching ground, is touching ground now, but does not have a last touchdown. Just set touchdown.
                                None => LockingData {
                                    touches_ground: true,
                                    last_touchdown: Some(event_time),
                                    ..prev_locking_data
                                },
                            }
                        }
                    }
                    // It's a newly generated piece directly spawned on the stack, or a piece that reached new lowest and needs completely reset locking data.
                    _ => LockingData {
                        touches_ground: true,
                        last_touchdown: Some(event_time),
                        last_liftoff: None,
                        ground_time_left: self.config.ground_time_max,
                        lowest_y: next_piece.position.1,
                    },
                };
                // Set lock timer if there isn't one, or refresh it if piece was moved.
                let repositioned = prev_piece_data
                    .map(|(prev_piece, _)| prev_piece != next_piece)
                    .unwrap_or(false);
                #[rustfmt::skip]
                let move_or_rotate = matches!(event, GameEvent::Rotate(_) | GameEvent::Move(_));
                if !self.state.events.contains_key(&GameEvent::LockTimer)
                    || (repositioned && move_or_rotate)
                {
                    // SAFETY: We know this must be `Some` in this case.
                    let current_ground_time =
                        event_time.saturating_sub(next_locking_data.last_touchdown.unwrap());
                    let remaining_ground_time = next_locking_data
                        .ground_time_left
                        .saturating_sub(current_ground_time);
                    let lock_timer = std::cmp::min(Self::lock_delay(level), remaining_ground_time);
                    self.state
                        .events
                        .insert(GameEvent::LockTimer, event_time + lock_timer);
                }
                next_locking_data
            }
            // [4] No change to state (afloat before and after).
            (Some((_prev_piece, prev_locking_data)), _next_piece_dat) => prev_locking_data,
        }
    }

    /// The amount of time left for a piece to fall naturally, purely dependent on level
    /// and an optional soft-drop-factor.
    #[rustfmt::skip]
    fn drop_delay(level: NonZeroU32, soft_drop: Option<f64>) -> Duration {
        let mut drop_delay = Duration::from_nanos(match level.get() {
             0 => unreachable!(),
             1 => 1_000_000_000,
             2 =>   793_000_000,
             3 =>   617_796_000,
             4 =>   472_729_139,
             5 =>   355_196_928,
             6 =>   262_003_550,
             7 =>   189_677_245,
             8 =>   134_734_731,
             9 =>    93_882_249,
            10 =>    64_151_585,
            11 =>    42_976_258,
            12 =>    28_217_678,
            13 =>    18_153_329,
            14 =>    11_439_342,
            15 =>     7_058_616,
            16 =>     4_263_557,
            17 =>     2_520_084,
            18 =>     1_457_139,
            19 =>       823_907, // NOTE: Close to 833'333ns = 1/120 s.
            20.. =>           0, // NOTE: We cap the formula here and call it INSTANT_GRAVITY.
        });
        if let Some(soft_drop_factor) = soft_drop {
            drop_delay = drop_delay.div_f64(0.00001f64.max(soft_drop_factor));
        }
        drop_delay
    }

    /// The amount of time left for an common ground lock timer, purely dependent on level.
    #[rustfmt::skip]
    const fn lock_delay(level: NonZeroU32) -> Duration {
        Duration::from_millis(match level.get() {
            1..=29 => 500,
                30 => 480,
                31 => 460,
                32 => 440,
                33 => 420,
                34 => 400,
                35 => 380,
                36 => 360,
                37 => 340,
                38 => 320,
                39 => 300,
                40 => 280,
                41 => 260,
                42 => 240,
                43 => 220,
                _  => 200,
        })
    }
}
