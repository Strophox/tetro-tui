/*!
This module handles what happens when [`Game::update`] is called.
*/

use super::*;

impl Game {
    /// A main function used to advance the game state.
    /// 
    /// See [`Game::update`] for the version where absolute in-game time (and not delta time) is used.
    pub fn update_delta(
        &mut self,
        time_elapsed: Duration,
        button_changes: &[ButtonChange],
    ) -> (FeedbackMessages, bool) {
        let update_target_time = self.state.time + time_elapsed;
        self.update(update_target_time, button_changes).unwrap()
    }

    /// The main function used to advance the game state.
    ///
    /// This will cause an internal update of the game's state up to and including the given
    /// `update_target_time` requested.
    /// If `button_changes` is nonempty, then the same thing happens except that the `button_changes`
    /// Will be used at `update_target_time` to update the state (which might cause some
    /// further events that are handled at `update_time`) before finally returning.
    ///
    /// Unless an error occurs, this function will return all [`FeedbackMessages`] caused between the
    /// previous and the current `update` call, in chronological order;
    /// It will also return a `bool` flag to signify whether game can be continued (i.e. has not ended).
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
        update_target_time: GameTime,
        mut button_changes: &[ButtonChange],
    ) -> Result<(FeedbackMessages, bool), UpdateGameError> {
        if update_target_time < self.state.time {
            return Err(UpdateGameError);
        }
        let mut feedback_msgs = Vec::new();
        // We linearly process all events until we reach the targeted update time.
        loop {
            // Maybe move on to game over if an end condition is met now.
            if let Some(new_phase) = self.try_end_game_if_end_condition_met() {
                self.phase = new_phase;
            }
            self.run_mods(UpdatePoint::MainLoop(&mut button_changes), false, &mut feedback_msgs);

            self.phase = match self.phase {
                // Game ended.
                // Return accumulated msgs and signal game ended.
                Phase::GameEnded(_) => {
                    return Ok((feedback_msgs, false))
                }
                
                // Lines clearing.
                // Move on to spawning.
                Phase::LinesClearing { line_clears_done_time } if line_clears_done_time <= update_target_time => {
                    self.run_mods(UpdatePoint::LinesClear, false, &mut feedback_msgs);
                    let new_phase = do_line_clears(&mut self.state, &self.config, line_clears_done_time);
                    self.run_mods(UpdatePoint::LinesClear, true, &mut feedback_msgs);
                    new_phase
                }

                // Piece spawning.
                // - May move on to game over (BlockOut).
                // - Normally: Move on to piece-in-play.
                Phase::Spawning { spawn_time } if spawn_time <= update_target_time => {
                    self.run_mods(UpdatePoint::PieceSpawn, false, &mut feedback_msgs);
                    let new_phase = do_spawn(&mut self.state, &self.config, spawn_time);
                    self.run_mods(UpdatePoint::PieceSpawn, true, &mut feedback_msgs);
                    new_phase
                }

                // Piece autonomously moving / falling / locking.
                // - Locking may move on to game over (LockOut).
                Phase::PieceInPlay { piece_data } if (
                    piece_data.fall_or_lock_scheduled <= update_target_time ||
                    piece_data.move_scheduled.is_some_and(|move_time| move_time <= update_target_time)
                ) => {
                    'workaround: {
                        if let Some(move_time) = piece_data.move_scheduled {
                            if move_time <= piece_data.fall_or_lock_scheduled && move_time <= update_target_time {
                                // Piece is moving autonomously and before next fall/lock.
                                self.run_mods(UpdatePoint::PieceAutoMove, false, &mut feedback_msgs);
                                let new_phase = do_autonomous_move(&mut self.state, &self.config, piece_data, move_time);
                                self.run_mods(UpdatePoint::PieceAutoMove, true, &mut feedback_msgs);
                                break 'workaround new_phase
                            }
                        }
                        // Piece is not moving autonomously and instead falls or locks
                        if piece_data.is_fall_not_lock {
                            self.run_mods(UpdatePoint::PieceFall, false, &mut feedback_msgs);
                            let new_phase = do_fall(&mut self.state, &self.config, piece_data);
                            self.run_mods(UpdatePoint::PieceFall, true, &mut feedback_msgs);
                            new_phase
                        } else {
                            self.run_mods(UpdatePoint::PieceLock, false, &mut feedback_msgs);
                            let new_phase = do_lock(&mut self.state, &self.config, piece_data.piece, piece_data.fall_or_lock_scheduled, &mut feedback_msgs);
                            self.run_mods(UpdatePoint::PieceLock, true, &mut feedback_msgs);
                            new_phase
                        }
                    }
                }

                Phase::PieceInPlay { piece_data } if !button_changes.is_empty() => {
                    let button_change = button_changes.first().unwrap().clone();
                    self.run_mods(UpdatePoint::PiecePlay(button_change), false, &mut feedback_msgs);
                    let new_phase = do_player_button_update(&mut self.state, &self.config, piece_data, button_change, update_target_time, &mut feedback_msgs);
                    self.run_mods(UpdatePoint::PiecePlay(button_change), true, &mut feedback_msgs);
                    button_changes = &button_changes[1..];
                    new_phase
                }

                // No actions within update target horizon, return from update call.
                _ => {
                    return Ok((feedback_msgs, true))
                }
            };
        }
    }

    /// Updates the internal `self.state.end` state, checking whether any [`Limits`] have been reached.
    fn try_end_game_if_end_condition_met(&self) -> Option<Phase> {
        // Game already ended.
        if self.result().is_some() {
            None

        // Not ended yet, so check whether any end conditions have been met now. 
        } else if let Some(game_result) = self.config.end_conditions.iter().find_map(|(c, good)| {
            self.check_stat_met(c)
                .then_some(if *good { Ok(()) } else { Err(GameOver::Limit) })
        }) {
            Some(Phase::GameEnded(game_result))
        
        } else {
            None
        }
    }

    /// Goes through all internal 'game mods' and applies them sequentially at the given [`ModifierPoint`].
    fn run_mods(
        &mut self,
        mut update_point: UpdatePoint<&mut &[ButtonChange]>,
        is_called_after: bool,
        feedback_msgs: &mut FeedbackMessages,
    ) {
        if self.config.feedback_verbosity == FeedbackVerbosity::Debug && is_called_after {
            use UpdatePoint as UP;
            let update_point = match &update_point {
                UP::MainLoop(x) => UP::MainLoop(format!("{x:?}")),
                UP::PiecePlay(b) => UP::PiecePlay(*b),
                UP::LinesClear => UP::LinesClear,
                UP::PieceSpawn => UP::PieceSpawn,
                UP::PieceAutoMove => UP::PieceAutoMove,
                UP::PieceFall => UP::PieceFall,
                UP::PieceLock => UP::PieceLock,
            };
            feedback_msgs.push((self.state.time, Feedback::Debug(update_point)));
        }
        for modifier in &mut self.modifiers {
            (modifier.mod_function)(
                &mut update_point,
                is_called_after,
                &mut self.config,
                &mut self.init_vals,
                &mut self.state,
                &mut self.phase,
                feedback_msgs,
            );
        }
    }
}

fn do_spawn(state: &mut State, config: &Configuration, spawn_time: GameTime) -> Phase {
    let [button_ml, button_mr, button_rl, button_rr, button_ra, button_ds, _dh, _td, _tl, _tr, button_h] = state.buttons_pressed.map(|keydowntime| keydowntime.is_some());

    // Take a tetromino.
    let spawn_tet = state.next_pieces.pop_front().unwrap_or_else(|| {
        state
            .piece_generator
            .with_rng(&mut state.rng)
            .next()
            .expect("piece generator empty before game end")
    });

    // Only put back in if necessary (e.g. if piece_preview_count < next_pieces.len()).
    state.next_pieces.extend(
        state
            .piece_generator
            .with_rng(&mut state.rng)
            .take(
                config
                    .piece_preview_count
                    .saturating_sub(state.next_pieces.len()),
            ),
    );
    
    // "Initial Hold" system.
    if button_h && config.allow_prespawn_actions {
        if let Some(new_phase) = try_hold(state, spawn_tet, spawn_time) {
            return new_phase;
        }
    }

    // Prepare data of spawned piece.
    let raw_pos = match spawn_tet {
        Tetromino::O => (4, 20),
        _ => (3, 20),
    };

    // 'Raw' spawn piece, before remaining prespawn_actions are applied.
    let raw_piece = Piece {
        shape: spawn_tet,
        orientation: Orientation::N,
        position: raw_pos,
    };

    // "Initial Rotation" system.
    let mut turns = 0;
    if config.allow_prespawn_actions {
        if button_rr { turns += 1; }
        if button_ra { turns += 2; }
        if button_rl { turns += 3; }
    }

    // Rotation of 'raw' spawn piece.
    let rotated_piece = config.rotation_system.rotate(
        &raw_piece,
        &state.board,
        turns,
    );

    // Try finding `Some` valid spawn piece from the provided options in order.
    let spawn_piece_opt = [
        rotated_piece,
        raw_piece.fits(&state.board).then_some(raw_piece),
    ]
    .into_iter()
    .find_map(|piece| piece);

    // Return new piece-in-play state if piece can spawn, otherwise blockout (couldn't spawn).
    if let Some(spawn_piece) = spawn_piece_opt {
        // We're falling if piece could move down.
        let is_fall_not_lock = spawn_piece.fits_at(&state.board, (0, -1)).is_some();
        // Standard fall or lock delay.
        let fall_or_lock_scheduled = if is_fall_not_lock {
            fall_delay(state.gravity, button_ds.then_some(config.soft_drop_factor))
        } else {
            lock_delay(state.gravity, None)
        };
        // Piece just spawned, lowest y = initial y.
        let lowest_y = spawn_piece.position.1;
        // Piece just spawned, standard full lock time max.
        let latest_lock_scheduled = spawn_time + lock_delay(state.gravity, Some(config.lock_time_max_factor));
        // Schedule immediate move after spawning, if any move button held.
        // NOTE: We have no Initial Move System for (mechanics, code) simplicity reasons.
        let move_scheduled = if button_ml || button_mr {
            Some(spawn_time)
        } else {
            None
        };
        
        Phase::PieceInPlay {
            piece_data: PieceData {
                piece: spawn_piece,
                fall_or_lock_scheduled,
                is_fall_not_lock,
                move_scheduled,
                lowest_y,
                latest_lock_scheduled,
            },
        }
    } else {
        Phase::GameEnded(Err(GameOver::BlockOut))
    }
}

fn do_line_clears(state: &mut State, config: &Configuration, line_clears_done_time: GameTime) -> Phase {
    for y in (0..Game::HEIGHT).rev() {
        // Full line: move it to the cleared lines storage and push an empty line to the board.
        if state.board[y].iter().all(|tile| tile.is_some()) {
            // Starting from the offending line, we move down all others, then default the uppermost.
            state.board[y..].rotate_left(1);
            state.board[Game::HEIGHT - 1] = Line::default();
            state.lines_cleared += 1;
            // Increment level if 10 lines cleared.
            if config.progressive_gravity && state.lines_cleared % 10 == 0 {
                state.gravity = state.gravity.saturating_add(1);
            }
        }
    }
    Phase::Spawning{ spawn_time: line_clears_done_time + config.appearance_delay }
}

fn do_autonomous_move (
    state: &mut State,
    config: &Configuration,
    previous_piece_data: PieceData,
    move_time: GameTime,
) -> Phase {
    // Move piece and update all appropriate piece-related values.
    let (dx, next_move_time) = calculate_movement_dx_and_next_move_time(&state.buttons_pressed, state.time, config);

    let mut new_piece = previous_piece_data.piece;
    if let Some(moved_piece) = previous_piece_data.piece.fits_at(&state.board, (dx, 0)) {
        new_piece = moved_piece;
    }
    
    let new_move_scheduled = Some(next_move_time);

    let new_is_fall_not_lock = new_piece.fits_at(&state.board, (0, -1)).is_some();

    let (new_lowest_y, new_latest_lock_scheduled) = if new_piece.position.1 < previous_piece_data.lowest_y {
        (new_piece.position.1, state.time + lock_delay(state.gravity, Some(config.lock_time_max_factor)))
    } else {
        (previous_piece_data.lowest_y, previous_piece_data.latest_lock_scheduled)
    };

    let new_fall_or_lock_scheduled =  if new_is_fall_not_lock {
        previous_piece_data.fall_or_lock_scheduled
    } else {
        (state.time + lock_delay(state.gravity, None)).min(new_latest_lock_scheduled)
    };

    // Update GameTime.
    state.time = move_time;
    
    // Update 'ActionState';
    // Return it to the main state machine with the latest acquired piece data.
    Phase::PieceInPlay {
        piece_data: PieceData {
            piece: new_piece,
            fall_or_lock_scheduled: new_fall_or_lock_scheduled,
            is_fall_not_lock: new_is_fall_not_lock,
            move_scheduled: new_move_scheduled,
            lowest_y: new_lowest_y,
            latest_lock_scheduled: new_latest_lock_scheduled,
        }
    }
}

fn do_fall(
    state: &mut State,
    config: &Configuration,
    previous_piece_data: PieceData,
) -> Phase {
    // # Overview
    //
    // The complexity of various subparts in this function are ranked roughly:
    //    1. Falling - due to how it is sometimes falling *and* moving *and then* updating falling/locking info.
    //    2. Moving - due to how it is mostly a single movement + updating falling/locking info.
    //    3. Locking - due to how simple it is if it happens.
    //
    // # Analysis of nontrivial autonomous-event updates (`PieceData.fall_or_lock_scheduled`, `PieceData.move_scheduled`).
    //
    // ## Falling
    // 
    // The fall timer is influenced as follows¹:
    // - immediate fall + refreshed falltimer  if  fell
    // - refreshed falltimer  if  (grounded ~> airborne)ᵃ
    // - [old falltimer  if  not in above cases]
    //
    // ## Locking
    // 
    // The lock timer is influenced as follows²:
    // - immediate lock  if  locked
    // - refreshed locktimer  if  (airborne ~> grounded)ᵇ
    // - [old locktimer  if  not in above cases]
    //
    // ## Moving
    //
    // The move timer is influenced as follows³:
    // - immediate move + some refreshed movetimer  if  moved
    // - no movetimer  if  move not possible
    // - [old movetimer  if  not in above cases]
    //
    // ### Move Resumption
    //
    // We *also* want to allow a player to hold 'move' while a piece is stuck, in a way where
    // the piece should move immediately as soon as it is unstuck⁴ (e.g. once fallen below the obstruction).
    // However, it has to be computed after another event has been handled that may be cause of unobstruction.

    // Drop piece and update all appropriate piece-related values.
    let mut new_piece = previous_piece_data.piece;
    if let Some(fallen_piece) = previous_piece_data.piece.fits_at(&state.board, (0, -1)) {
        new_piece = fallen_piece;
    }

    // Move resumption.
    let (dx, next_move_time) = calculate_movement_dx_and_next_move_time(&state.buttons_pressed, state.time, config);
    let new_move_scheduled = if let Some((moved_piece, new_move_scheduled)) = try_move_and_refresh_move_scheduled(previous_piece_data.piece, new_piece, &state.board, (dx, next_move_time)) {
        // Naïvely, movement direction should be kept;
        // But due to the system mentioned in (⁴), we do need to check
        // if the piece was stuck and became unstuck, and manually do a move in this case! 
        new_piece = moved_piece;
        new_move_scheduled

    } else {
        // No changes need to be made.
        previous_piece_data.move_scheduled
    };

    let new_is_fall_not_lock = new_piece.fits_at(&state.board, (0, -1)).is_some();

    let (new_lowest_y, new_latest_lock_scheduled) = if new_piece.position.1 < previous_piece_data.lowest_y {
        (new_piece.position.1, state.time + lock_delay(state.gravity, Some(config.lock_time_max_factor)))
    } else {
        (previous_piece_data.lowest_y, previous_piece_data.latest_lock_scheduled)
    };

    let new_fall_or_lock_scheduled =  if new_is_fall_not_lock {
        let soft_drop_factor = state.buttons_pressed[Button::DropSoft].is_some().then_some(config.soft_drop_factor);
        state.time + fall_delay(state.gravity, soft_drop_factor)

    } else {
        (state.time + lock_delay(state.gravity, None)).min(new_latest_lock_scheduled)
    };

    // Update GameTime.
    state.time = previous_piece_data.fall_or_lock_scheduled;
    
    // 'Update' ActionState;
    // Return it to the main state machine with the latest acquired piece data.
    Phase::PieceInPlay {
        piece_data: PieceData {
            piece: new_piece,
            fall_or_lock_scheduled: new_fall_or_lock_scheduled,
            is_fall_not_lock: new_is_fall_not_lock,
            move_scheduled: new_move_scheduled,
            lowest_y: new_lowest_y,
            latest_lock_scheduled: new_latest_lock_scheduled,
        }
    }
}

fn do_player_button_update(
    state: &mut State,
    config: &Configuration,
    previous_piece_data: PieceData,
    button_change: ButtonChange,
    update_time: GameTime,
    feedback_msgs: &mut FeedbackMessages,
) -> Phase {
    // # Overview
    //
    // The complexity of various subparts in this function are ranked roughly:
    //    1. Figuring out movement and future movement (scheduling / preparing autonomous piece updates).
    //    2. Figuring out falling and locking (scheduling / preparing autonomous piece updates).
    //    3. All other immediate button changes (easy).
    //
    // # Analysis of nontrivial autonomous-event updates (`PieceData.fall_or_lock_scheduled`, `PieceData.move_scheduled`).
    //
    // ## Falling
    // 
    // The fall timer is influenced as follows¹:
    // - refreshed falltimer  if  (grounded ~> airborne)ᵃ
    // - refreshed falltimer  if  (_ ~> airborne) + soft drop just pressed
    // - refreshed falltimer  if  (_ ~> airborne) + soft drop just released
    // - [old falltimer  if  (airborne ~> airborne) = not in above cases, c.f. (ᵃ)]
    //
    // ## Locking
    // 
    // The lock timer is influenced as follows²:
    // - zero locktimer  if  (_ ~> grounded) + soft drop just pressed
    // - zero locktimer  if  (_ ~> grounded) + hard drop just pressed
    // - refreshed locktimer  if  (_ ~> grounded) + (position|orientation) just changedᵇ
    // - [old locktimer  if  (grounded ~> grounded) = not in above cases, c.f. (ᵇ)]
    //
    // ## Moving
    //
    // We analyze cases:
    //
    // Table 1.: Karnaugh map⁵.
    // +-----------+-------------------------------------+ 
    // |           |   Rel.   Rel.   Prs.   Prs.   Any
    // | Old state |    mr     ml     ml     mr    Other
    // +           +-------------------------------------+
    // |   ¬ml ¬mr |    |-|    |-|     ←₊ⁱ    →₊ⁱ   |-|
    // |   ¬ml  mr |   →₋ᵏ     |→|   →₋←₊ⁱ   |→|    |→|
    // |    ml  mr |   ⇆₋←₊ⁱ  ⇆₋→₊ⁱ   |⇆|    |⇆|    |⇆|
    // |    ml ¬mr |    |←|   ←₋ᵏ     |←|   ←₋→₊ⁱ   |←|
    // +-----------+-------------------------------------+ 
    // |  '-' = not moving      'X₋  ' = stop X
    // |  '←' = moving left     '  X₊' = start X
    // |  '→' = moving right    ' |X|' = keep X
    // |  '⇆' = moving <direction depending
    // |                on previous timing>
    // |  ᵏ: cease autonomous moves 
    // |  ⁱ: immediate move + refresh autonomous moves
    // +-------------------------------------------------+
    //
    // Table 2.: Comparing with `dx` instead.
    // +--------+-------------------------------------+
    // | Old dx |   Rml    Rmr    Pml    Pmr    Other
    // +        +-------------------------------------+
    // | ← = -1 |  ←₋¿→₊    |←|    |←|   ←₋→₊     |←|
    // | - =  0 |    | |    | |     ←₊     →₊     | |
    // | → =  1 |    |→|  →₋¿←₊   →₋←₊    |→|     |→|
    // +--------+-------------------------------------+XXXXXXXXXXXXXx
    //
    // ### Moving
    //
    // The (ⁱ)/(ᵏ)-entries of Table 1 are the major effects of move inputs to be implemented³:
    // - immediate move + refreshed movetimer  if  (ⁱ)
    // - removed movetimer  if  (ᵏ)
    //
    // Otherwise we should implement:
    // - old movetimer  if   no change like in (ⁱ)/(ᵏ)
    //
    // ### Move Resumption
    //
    // We *also* want to allow a player to hold 'move' while a piece is stuck, in a way where
    // the piece should move immediately as soon as it is unstuck (e.g. once fallen below the obstruction)⁴.
    // This system takes effect in the non-(ⁱ)/(ᵏ)-entries of Table 1.
    // However, it has to be computed after another event has been handled that may be cause of unobstruction.

    // Prepare to maybe change the move_scheduled.
    let mut maybe_override_move_scheduled: Option<Option<GameTime>> = None;
    // Pre-compute current direction of movement and projected next movement time.
    let (dx, next_move_time) = calculate_movement_dx_and_next_move_time(&state.buttons_pressed, update_time, config);

    let mut new_piece = previous_piece_data.piece;
    use {ButtonChange as BC, Button as B};
    match button_change {
        // Hold.
        // - If succeeds, changes game action state to spawn different piece.
        // - Otherwise does nothing.
        BC::Press(B::HoldPiece) => {
            if let Some(new_phase) = try_hold(state, new_piece.shape, update_time) {
                return new_phase;
            }
        },

        // Teleports.
        // Just instantly try to move piece all the way into applicable direction.
        BC::Press(dir @ (B::TeleDown | B::TeleLeft | B::TeleRight)) => {
            let offset = match dir {
                B::TeleDown => (0, -1), B::TeleLeft => (-1, 0), B::TeleRight => (1, 0), _=> unreachable!()
            };
            new_piece = new_piece.teleported(&state.board, offset);
        },

        // Rotates.
        // Just instantly try to rotate piece into applicable direction.
        BC::Press(dir @ (B::RotateLeft | B::RotateRight | B::RotateAround)) => {
            let right_turns = match dir {
                B::RotateLeft => -1, B::RotateRight => 1, B::RotateAround => 2, _=> unreachable!()
            };
            if let Some(rotated_piece) = config.rotation_system.rotate(&new_piece, &state.board, right_turns) {
                new_piece = rotated_piece;
            }
        },

        // Hard Drop.
        // Instantly try to move piece all the way down.
        // The locking is handled as part of a different check/system further.
        BC::Press(B::DropHard) => {
            new_piece = new_piece.teleported(&state.board, (0, -1));

            if config.feedback_verbosity != FeedbackVerbosity::Silent {
                feedback_msgs.push((update_time, Feedback::HardDrop(previous_piece_data.piece, new_piece)));
            }
        },
        
        // Soft Drop.
        // Instantly try to move piece one tile down.
        // The locking is handled as part of a different check/system further.
        BC::Press(B::DropSoft) => {
            if let Some(fallen_piece) = new_piece.fits_at(&state.board, (0, -1)) {
                new_piece = fallen_piece;
            }
        },

        // Moves and move releases.
        // These are very complicated and stateful.
        // The logic implemented here is based on (³) and the Karnaugh map in (⁵)
        BC::Press(dir @ (B::MoveLeft | B::MoveRight)) | BC::Release(dir @ (B::MoveLeft | B::MoveRight)) => {
            let ml = state.buttons_pressed[B::MoveLeft].is_some();
            let mr = state.buttons_pressed[B::MoveRight].is_some();
            let is_prs = matches!(button_change, BC::Press(_));
            let is_ml = matches!(dir, B::MoveLeft);

            let rel_one = ml && mr && !is_prs;  // ⇆₋←₊ⁱ, ⇆₋→₊ⁱ
            let prs_ml = !ml && is_prs && is_ml;  // ←₊ⁱ; →₋←₊ⁱ
            let prs_mr = !mr && is_prs && !is_ml; // →₊ⁱ; ←₋→₊ⁱ
            let rel_ml = ml && !mr && !is_prs && is_ml;  // ←₋ᵏ
            let rel_mr = !ml && mr && !is_prs && !is_ml; // →₋ᵏ
            maybe_override_move_scheduled = if rel_one || prs_ml || prs_mr {
                if let Some(moved_piece) = new_piece.fits_at(&state.board, (dx, 0)) {
                    new_piece = moved_piece;
                }
                Some(Some(next_move_time)) // Refresh autonomous movement.
            } else if rel_mr || rel_ml {
                Some(None) // Remove any autonomous movement.
            } else {
                None // Do not change autonomous movement.
            };
        },

        // Various button releases.
        // These don't have any direct effect (move, rotate) on the `piece` in themselves.
        BC::Release(
            B::RotateLeft
            | B::RotateRight
            | B::RotateAround
            | B::DropSoft
            | B::DropHard
            | B::TeleDown
            | B::TeleLeft
            | B::TeleRight
            | B::HoldPiece
        ) => {},
    }

    // Epilogue. Finalize state updates.

    // Update movetimer and rest of movement stuff.
    // See also (³) and (⁴).
    let new_move_scheduled = if let Some(move_scheduled) = maybe_override_move_scheduled {
        // If we were in a case where movement was explicitly changed, do so.
        // This implements (³).
        move_scheduled

    } else if let Some((moved_piece, new_move_scheduled)) = try_move_and_refresh_move_scheduled(previous_piece_data.piece, new_piece, &state.board, (dx, next_move_time)) {
        // Naïvely, movement direction should be kept;
        // But due to the system mentioned in (⁴), we do need to check
        // if the piece was stuck and became unstuck, and manually do a move in this case! 
        new_piece = moved_piece;
        new_move_scheduled

    } else {
        // All checks passed, no changes need to be made.
        // This is the case where neither (³) or (⁴) apply.
        previous_piece_data.move_scheduled
    };

    // Update `lowest_y`, re-set `latest_lock_scheduled` if applicable.
    // `latest_lock_scheduled` is needed below.
    let (new_lowest_y, new_latest_lock_scheduled) = if new_piece.position.1 < previous_piece_data.lowest_y {
        (new_piece.position.1, state.time + lock_delay(state.gravity, Some(config.lock_time_max_factor)))
    } else {
        (previous_piece_data.lowest_y, previous_piece_data.latest_lock_scheduled)
    };

    // Update `is_fall_not_lock`, i.e., whether we are falling (otherwise locking) now.
    // `new_is_fall_not_lock` is needed below.
    let new_is_fall_not_lock = new_piece.fits_at(&state.board, (0,-1)).is_some();

    // Update falltimer and locktimer.
    // See also (¹) and (²).
    let new_fall_or_lock_scheduled = if new_is_fall_not_lock {
        // Calculate scheduled fall time.
        // This implements (¹).
        let was_falling = previous_piece_data.piece.fits_at(&state.board, (0,-1)).is_some();
        if !was_falling || matches!(button_change, BC::Press(B::DropSoft | B::DropHard)) {
            // If we *started* falling, or soft drop just pressed, or soft drop just released.
            let soft_drop_factor = matches!(button_change, BC::Press(B::DropSoft)).then_some(config.soft_drop_factor);
            state.time + fall_delay(state.gravity, soft_drop_factor)

        } else {
            // Falling as before.
            previous_piece_data.fall_or_lock_scheduled
        }

    } else {
        // Calculate scheduled lock time.
        // This implements (²).
        if matches!(button_change, BC::Press(B::DropSoft | B::DropHard)) {
            // We are on the ground - if soft drop or hard drop pressed, lock immediately.
            state.time

        } else if new_piece != previous_piece_data.piece {
            // On the ground - Refresh lock time if piece moved.
            (state.time + lock_delay(state.gravity, None)).min(new_latest_lock_scheduled)

        } else {
            // Previous lock time.
            previous_piece_data.fall_or_lock_scheduled
        }
    };

    // Update PressedButtons.
    match button_change {
        BC::Press(button) => state.buttons_pressed[button] = Some(update_time),
        BC::Release(button) => state.buttons_pressed[button] = None,
    };

    // Update GameTime.
    // This might be redundant if we process several player button updates which 'took place at the same time' (but are still processed sequentially).
    state.time = update_time;

    // 'Update' ActionState;
    // Return it to the main state machine with the latest acquired piece data.
    Phase::PieceInPlay {
        piece_data: PieceData {
            piece: new_piece,
            fall_or_lock_scheduled: new_fall_or_lock_scheduled,
            is_fall_not_lock: new_is_fall_not_lock,
            move_scheduled: new_move_scheduled,
            lowest_y: new_lowest_y,
            latest_lock_scheduled: new_latest_lock_scheduled,
        }
    }
}

fn calculate_movement_dx_and_next_move_time(buttons_pressed: &[Option<GameTime>; Button::VARIANTS.len()], time_now: GameTime, config: &Configuration) -> (isize, GameTime) {
    let (dx, how_long_relevant_direction_pressed) = match (
        buttons_pressed[Button::MoveLeft],
        buttons_pressed[Button::MoveRight],
    ) {
        (Some(t_left), Some(t_right)) =>
            match t_left.cmp(&t_right) {
                // 'Right' was pressed more recently, go right.
                std::cmp::Ordering::Less => (1, time_now.saturating_sub(t_right)),
                // Both pressed at exact same time, don't move.
                std::cmp::Ordering::Equal => (0, Duration::ZERO),
                // 'Left' was pressed more recently, go left.
                std::cmp::Ordering::Greater => (-1, time_now.saturating_sub(t_left)),
            }
        // Only 'left' pressed.
        (Some(t_left), None) => (-1, time_now.saturating_sub(t_left)),
        // Only 'right' pressed.
        (None, Some(t_right)) => (1, time_now.saturating_sub(t_right)),
        // None pressed. No movement.
        (None, None) => (0, Duration::ZERO),
    };

    let next_move_scheduled = time_now + if how_long_relevant_direction_pressed >= config.delayed_auto_shift {
        config.auto_repeat_rate
    } else {
        config.delayed_auto_shift
    };

    (dx, next_move_scheduled)
}

fn try_move_and_refresh_move_scheduled(old_piece: Piece, new_piece: Piece, board: &Board, (dx, next_move_time): (isize, GameTime)) -> Option<(Piece, Option<GameTime>)> {
    let try_move_old = old_piece.fits_at(board, (dx, 0));
    let try_move_new = new_piece.fits_at(board, (dx, 0));
    if let (None, Some(moved_piece)) = (try_move_old, try_move_new) {
        Some((moved_piece, Some(next_move_time)))
    
    // All checks passed, no changes need to be made.
    // This is the case where neither (³) or (⁴) apply.
    } else {
        None
    }
}

fn try_hold(state: &mut State, tetromino: Tetromino, spawn_time: GameTime) -> Option<Phase> {
    match state.hold_piece {
        // Nothing held yet, just hold spawned tetromino.
        None => {
            state.hold_piece = Some((tetromino, false));
            // Issue a spawn.
            Some(Phase::Spawning { spawn_time })
        }
        // Swap spawned tetromino, push held back into next pieces queue.
        Some((held_tet, true)) => {
            state.hold_piece = Some((tetromino, false));
            // Cause the next spawn to specially be the piece we held.
            state.next_pieces.push_front(held_tet);
            // Issue a spawn.
            Some(Phase::Spawning { spawn_time })
        }
        // Else can't hold, don't do anything.
        _ => None
    }
}

fn do_lock(state: &mut State, config: &Configuration, piece: Piece, lock_time: GameTime, feedback_msgs: &mut FeedbackMessages) -> Phase {
    if config.feedback_verbosity != FeedbackVerbosity::Silent {
        feedback_msgs.push((lock_time, Feedback::PieceLocked(piece)));
    }

    // Before board is changed, precompute whether a piece was 'spun' into position;
    // - 'Spun' pieces give higher score bonus.
    // - Only locked pieces can yield bonus (i.e. can't possibly move down).
    // - Only locked pieces clearing lines can yield bonus (i.e. can't possibly move left/right).
    // Thus, if a piece cannot move back up at lock time, it must have gotten there by rotation.
    // That's what a 'spin' is.
    let is_spin = piece.fits_at(&state.board, (0, 1)).is_none();

    // Locking.
    for ((x, y), tile_type_id) in piece.tiles() {
        // Tiles are not allowed above `SKYLINE`, leading to a possible early `LockOut`.
        if y >= Game::SKYLINE {
            return Phase::GameEnded(Err(GameOver::LockOut));
        }

        // Set tile into board.
        state.board[y][x] = Some(tile_type_id);
    }

    // Update tally of pieces_locked.
    state.pieces_locked[piece.shape as usize] += 1;

    // Score bonus calculation.
    
    // Find lines which might get cleared by piece locking. (actual clearing done later).
    let mut lines_cleared = Vec::<usize>::with_capacity(4);
    for y in (0..Game::HEIGHT).rev() {
        if state.board[y].iter().all(|mino| mino.is_some()) {
            lines_cleared.push(y);
        }
    }
    
    let n_lines_cleared = u32::try_from(lines_cleared.len()).unwrap();

    if n_lines_cleared == 0 {
        // If no lines cleared, no score bonus and combo is reset.
        state.consecutive_line_clears = 0;

    } else {
        // Increase combo.
        state.consecutive_line_clears += 1;

        let n_combo = state.consecutive_line_clears;

        let is_perfect_clear = state.board.iter().all(|line| {
            line.iter().all(|tile| tile.is_none())
                || line.iter().all(|tile| tile.is_some())
        });

        // Compute main Score Bonus.
        let score_bonus = n_lines_cleared
            * if is_spin { 2 } else { 1 }
            * if is_perfect_clear { 4 } else { 1 }
            * 2 - 1 + (state.consecutive_line_clears - 1);

        // Update score.
        state.score += u64::from(score_bonus);

        if config.feedback_verbosity != FeedbackVerbosity::Silent {
            feedback_msgs.push((
                lock_time,
                Feedback::LinesClearing(lines_cleared, config.line_clear_delay),
            ));

            feedback_msgs.push((
                lock_time,
                Feedback::Accolade {
                    score_bonus,
                    tetromino: piece.shape,
                    is_spin,
                    lines_cleared: n_lines_cleared,
                    is_perfect_clear,
                    combo: n_combo,
                },
            ));
        }
    }
    
    // Update ability to hold piece.
    state.hold_piece = state.hold_piece.map(|(held_piece, _swap_allowed)| (held_piece, true));

    // Update GameTime.
    state.time = lock_time;

    // 'Update' ActionState;
    // Return it to the main state machine with all newly acquired piece data.
    if n_lines_cleared == 0 {
        // No lines cleared, directly proceed to spawn.
        Phase::Spawning { spawn_time: lock_time + config.appearance_delay }
    } else {
        // Lines cleared, enter line clearing state.
        Phase::LinesClearing { line_clears_done_time: lock_time + config.line_clear_delay }
    }
}

// FIXME: Make this more parametric instead of hardcoded?
/// The amount of time left for a piece to fall naturally, purely dependent on level
/// and an optional soft-drop-factor.
#[rustfmt::skip]
fn fall_delay(gravity: u32, soft_drop_factor: Option<f64>) -> Duration {
    let raw_drop_delay = Duration::from_nanos(match gravity {
        0 => return Duration::MAX / 2,
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
    if let Some(factor) = soft_drop_factor {
        if 0.1e-10 < factor {
            return raw_drop_delay.div_f64(factor)
        }
    }
    raw_drop_delay
}

// FIXME: Make this more parametric instead of hardcoded?
/// The amount of time left for an common ground lock timer, purely dependent on level.
#[rustfmt::skip]
fn lock_delay(gravity: u32, lock_time_max_factor: Option<f64>) -> Duration {
    let raw_lock_delay = Duration::from_millis(match gravity {
        0 => return Duration::MAX / 2,
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
    });
    if let Some(factor) = lock_time_max_factor {
        if 0.0 <= factor && factor.is_finite() {
            return raw_lock_delay.mul_f64(factor)
        }
    }
    raw_lock_delay
}
