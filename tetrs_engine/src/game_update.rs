/*!
This module handles what happens when [`Game::update`] is called.
*/

use super::*;

impl Game {
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
        target_time: GameTime,
        mut button_changes: Option<ButtonChange>,
    ) -> Result<FeedbackMessages, UpdateGameError> {
//let/*TODO:dbg*/s=format!("# IN___ update {target_time:?}, {button_changes:?}, {:?} {:?}\n", self.phase, self.state);if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
        if target_time < self.state.time {
            // Do not allow updating if target time lies in the past.
            return Err(UpdateGameError::TargetTimeInPast);
        } else if self.result().is_some() {
            // Do not allow updating a game that has already ended.
            return Err(UpdateGameError::GameEnded);
        }

        // Prepare new button state.
        let mut new_state_buttons_pressed = self.state.buttons_pressed;
        match button_changes {
            Some(ButtonChange::Press(button)) => new_state_buttons_pressed[button] = Some(target_time),
            Some(ButtonChange::Release(button)) => new_state_buttons_pressed[button] = None,
            None => {},
        }

        let mut feedback_msgs = Vec::new();

        // We linearly process all events until we reach the targeted update time.
        loop {
            // Maybe move on to game over if an end condition is met now.
            if let Some(new_phase) = self.try_end_game_if_end_condition_met() {
                self.phase = new_phase;
            }
            self.run_mods(UpdatePoint::MainLoopHead(&mut button_changes), &mut feedback_msgs);

            match (self.phase, button_changes) {
                // Game ended by now.
                // Return accumulated messages.
                (Phase::GameEnded(_), _) => {
//let/*TODO:dbg*/s=format!("# OUTOF update {target_time:?}, {button_changes:?}, {:?} {:?}\n", self.phase, self.state);if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                    return Ok(feedback_msgs)
                }
                
                // Lines clearing.
                // Move on to spawning.
                (Phase::LinesClearing { lines_cleared_time }, _) if lines_cleared_time <= target_time => {
//let/*TODO:dbg*/s=format!("INTO do_line_clearing ({lines_cleared_time:?})\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                    self.phase = do_line_clearing(&mut self.state, &self.config, lines_cleared_time);
                    self.state.time = lines_cleared_time;
                    
                    self.run_mods(UpdatePoint::LinesCleared, &mut feedback_msgs);
                }

                // Piece spawning.
                // - May move on to game over (BlockOut).
                // - Normally: Move on to piece-in-play.
                (Phase::Spawning { spawn_time }, _) if spawn_time <= target_time => {
                    self.phase = do_spawn(&mut self.state, &self.config, spawn_time);
                    self.state.time = spawn_time;

                    self.run_mods(UpdatePoint::PieceSpawned, &mut feedback_msgs);
                }

                // Piece autonomously moving / falling / locking.
                // - Locking may move on to game over (LockOut).
                (Phase::PieceInPlay { piece_data }, _) if (
                    piece_data.fall_or_lock_time <= target_time ||
                    piece_data.auto_move_scheduled.is_some_and(|move_time| move_time <= target_time)
                ) => {
                    let mut flag = false;
                    if let Some(move_time) = piece_data.auto_move_scheduled {
                        if move_time <= piece_data.fall_or_lock_time && move_time <= target_time {
                            // Piece is moving autonomously and before next fall/lock.
                            flag = true;

//let/*TODO:dbg*/s=format!("INTO do_autonomous_move ({move_time:?})\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                            self.phase = do_autonomous_move(&mut self.state, &self.config, piece_data, move_time);
                            self.state.time = move_time;
                            
                            self.run_mods(UpdatePoint::PieceAutoMoved, &mut feedback_msgs);
                        }
                    }
                    // else: Piece is not moving autonomously and instead falls or locks
                    if !flag {
                        if piece_data.is_fall_not_lock {
//let/*TODO:dbg*/s=format!("INTO do_fall ({:?})\n", piece_data.fall_or_lock_time);if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                            self.phase = do_fall(&mut self.state, &self.config, piece_data, piece_data.fall_or_lock_time);
                            self.state.time = piece_data.fall_or_lock_time;
                            
                            self.run_mods(UpdatePoint::PieceFell, &mut feedback_msgs);
                        } else {
//let/*TODO:dbg*/s=format!("INTO do_lock ({:?})\n", piece_data.fall_or_lock_time);if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                            self.phase = do_lock(&mut self.state, &self.config, piece_data.piece, piece_data.fall_or_lock_time, &mut feedback_msgs);
                            self.state.time = piece_data.fall_or_lock_time;
                            
                            self.run_mods(UpdatePoint::PieceLocked, &mut feedback_msgs);
                        }
                    }
                }

                (Phase::PieceInPlay { piece_data }, Some(button_change)) => {
                    button_changes.take();
//let/*TODO:dbg*/s=format!("INTO do_player_button_update ({target_time:?})\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                    self.phase = do_player_button_update(&mut self.state, &self.config, piece_data, button_change, new_state_buttons_pressed, target_time, &mut feedback_msgs);
                    self.state.time = target_time;
                    self.state.buttons_pressed = new_state_buttons_pressed;

                    self.run_mods(UpdatePoint::PiecePlayed(button_change), &mut feedback_msgs);
                }

                // No actions within update target horizon, stop updating.
                _ => {
                    // Ensure states are updated.
                    // NOTE: This *might* be redundant in some cases.
                    
                    // NOTE: Ensure time is updated as requested, even when none of above cases triggered.
                    self.state.time = target_time;

                    // NOTE: Ensure button state is updated as requested, even when `PieceInPlay` case is not triggered.
                    self.state.buttons_pressed = new_state_buttons_pressed;
                    
//let/*TODO:dbg*/s=format!("# OUTOF update {target_time:?}, {button_changes:?}, {:?} {:?}\n", self.phase, self.state);if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                    return Ok(feedback_msgs)
                }
            }
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
        mut update_point: UpdatePoint<&mut Option<ButtonChange>>,
        feedback_msgs: &mut FeedbackMessages,
    ) {
        if self.config.feedback_verbosity == FeedbackVerbosity::Debug {
            use UpdatePoint as UP;
            let update_point = match &update_point {
                UP::MainLoopHead(x) => UP::MainLoopHead(format!("{x:?}")),
                UP::PiecePlayed(b) => UP::PiecePlayed(*b),
                UP::LinesCleared => UP::LinesCleared,
                UP::PieceSpawned => UP::PieceSpawned,
                UP::PieceAutoMoved => UP::PieceAutoMoved,
                UP::PieceFell => UP::PieceFell,
                UP::PieceLocked => UP::PieceLocked,
            };
            feedback_msgs.push((self.state.time, Feedback::Debug(update_point)));
        }
        for modifier in &mut self.modifiers {
            (modifier.mod_function)(
                &mut update_point,
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
//let/*TODO:dbg*/s=format!("IN do_spawn\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
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
    let raw_spawn_piece = Piece {
        tetromino: spawn_tet,
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
    let rotated_spawn_piece = config.rotation_system.rotate(
        &raw_spawn_piece,
        &state.board,
        turns,
    );

    // Try finding `Some` valid spawn piece from the provided options in order.
    let spawn_piece = [
        rotated_spawn_piece,
        raw_spawn_piece.fits(&state.board).then_some(raw_spawn_piece),
    ]
    .into_iter()
    .find_map(|option| option);

    // Return new piece-in-play state if piece can spawn, otherwise blockout (couldn't spawn).
    if let Some(piece) = spawn_piece {
        // We're falling if piece could move down.
        let is_fall_not_lock = piece.fits_at(&state.board, (0, -1)).is_some();
        // Standard fall or lock delay.
        let fall_or_lock_time = spawn_time + if is_fall_not_lock {
            fall_delay(state.gravity, button_ds.then_some(config.soft_drop_factor))
        } else {
            lock_delay(state.gravity, None)
        };
        // Piece just spawned, lowest y = initial y.
        let lowest_y = piece.position.1;
        // Piece just spawned, standard full lock time max.
        let binding_lock_time = spawn_time + lock_delay(state.gravity, Some(config.lock_time_max_factor));
        // Schedule immediate move after spawning, if any move button held.
        // NOTE: We have no Initial Move System for (mechanics, code) simplicity reasons.
        let auto_move_scheduled = if button_ml || button_mr {
            Some(spawn_time)
        } else {
            None
        };
        
        Phase::PieceInPlay {
            piece_data: PieceData {
                piece,
                fall_or_lock_time,
                is_fall_not_lock,
                auto_move_scheduled,
                lowest_y,
                binding_lock_time,
            },
        }
    } else {
        Phase::GameEnded(Err(GameOver::BlockOut))
    }
}

fn do_line_clearing(state: &mut State, config: &Configuration, lines_cleared_time: GameTime) -> Phase {
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
    Phase::Spawning{ spawn_time: lines_cleared_time + config.appearance_delay }
}

fn do_autonomous_move (
    state: &mut State,
    config: &Configuration,
    previous_piece_data: PieceData,
    auto_move_time: GameTime,
) -> Phase {
    // Move piece and update all appropriate piece-related values.
    // NOTE: This should give non-zero `dx`.
    let (dx, next_move_time) = calc_move_dx_and_next_move_time(&state.buttons_pressed, auto_move_time, config);

    let mut new_piece = previous_piece_data.piece;
    let new_auto_move_scheduled = if let Some(moved_piece) = previous_piece_data.piece.fits_at(&state.board, (dx, 0)) {
        new_piece = moved_piece;
        Some(next_move_time) // Able to do relevant move; Insert autonomous movement.
    } else {
        None // Unable to move; Remove autonomous movement.
    };

    let (new_lowest_y, new_binding_lock_time) = if new_piece.position.1 < previous_piece_data.lowest_y {
        (new_piece.position.1, auto_move_time + lock_delay(state.gravity, Some(config.lock_time_max_factor)))
    } else {
        (previous_piece_data.lowest_y, previous_piece_data.binding_lock_time)
    };

    let new_is_fall_not_lock = new_piece.fits_at(&state.board, (0, -1)).is_some();

    let new_fall_or_lock_time =  if new_is_fall_not_lock {
        // Calculate scheduled fall time.
        // This implements (¹).
        let was_airborne = previous_piece_data.piece.fits_at(&state.board, (0,-1)).is_some();
        if !was_airborne {
            // Refresh fall timer if we *started* falling.
            let soft_drop_factor = state.buttons_pressed[Button::DropSoft].is_some().then_some(config.soft_drop_factor);
            auto_move_time + fall_delay(state.gravity, soft_drop_factor)

        } else {
            // Falling as before.
            previous_piece_data.fall_or_lock_time
        }
    } else {
        (auto_move_time + lock_delay(state.gravity, None)).min(new_binding_lock_time)
    };
    
    // Update 'ActionState';
    // Return it to the main state machine with the latest acquired piece data.
    Phase::PieceInPlay {
        piece_data: PieceData {
            piece: new_piece,
            fall_or_lock_time: new_fall_or_lock_time,
            is_fall_not_lock: new_is_fall_not_lock,
            auto_move_scheduled: new_auto_move_scheduled,
            lowest_y: new_lowest_y,
            binding_lock_time: new_binding_lock_time,
        }
    }
}

fn do_fall(
    state: &mut State,
    config: &Configuration,
    previous_piece_data: PieceData,
    fall_time: GameTime
) -> Phase {
    // # Overview
    //
    // The complexity of various subparts in this function are ranked roughly:
    //    1. Falling - due to how it is sometimes falling *and* moving *and then* updating falling/locking info.
    //    2. Moving - due to how it is mostly a single movement + updating falling/locking info.
    //    3. Locking - due to how simple it is if it happens.
    //
    // # Analysis of nontrivial autonomous-event updates (`PieceData.fall_or_lock_time`, `PieceData.move_scheduled`).
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
    let new_auto_move_scheduled = if let Some((moved_piece, new_move_scheduled)) = check_piece_became_movable_get_moved_piece_and_move_scheduled(previous_piece_data.piece, new_piece, &state.board, calc_move_dx_and_next_move_time(&state.buttons_pressed, fall_time, config)) {
        // Naïvely, movement direction should be kept;
        // But due to the system mentioned in (⁴), we do need to check
        // if the piece was stuck and became unstuck, and manually do a move in this case! 
        new_piece = moved_piece;
        new_move_scheduled

    } else {
        // No changes need to be made.
        previous_piece_data.auto_move_scheduled
    };

    let (new_lowest_y, new_binding_lock_time) = if new_piece.position.1 < previous_piece_data.lowest_y {
        (new_piece.position.1, fall_time + lock_delay(state.gravity, Some(config.lock_time_max_factor)))
    } else {
        (previous_piece_data.lowest_y, previous_piece_data.binding_lock_time)
    };

    let new_is_fall_not_lock = new_piece.fits_at(&state.board, (0, -1)).is_some();

    let new_fall_or_lock_time =  if new_is_fall_not_lock {
        let soft_drop_factor = state.buttons_pressed[Button::DropSoft].is_some().then_some(config.soft_drop_factor);
        fall_time + fall_delay(state.gravity, soft_drop_factor)

    } else {
        (fall_time + lock_delay(state.gravity, None)).min(new_binding_lock_time)
    };
    
    // 'Update' ActionState;
    // Return it to the main state machine with the latest acquired piece data.
    Phase::PieceInPlay {
        piece_data: PieceData {
            piece: new_piece,
            fall_or_lock_time: new_fall_or_lock_time,
            is_fall_not_lock: new_is_fall_not_lock,
            auto_move_scheduled: new_auto_move_scheduled,
            lowest_y: new_lowest_y,
            binding_lock_time: new_binding_lock_time,
        }
    }
}

fn do_player_button_update(
    state: &mut State,
    config: &Configuration,
    previous_piece_data: PieceData,
    button_change: ButtonChange,
    new_state_buttons_pressed: [Option<GameTime>; Button::VARIANTS.len()],
    button_update_time: GameTime,
    feedback_msgs: &mut FeedbackMessages,
) -> Phase {
    // # Overview
    //
    // The complexity of various subparts in this function are ranked roughly:
    //    1. Figuring out movement and future movement (scheduling / preparing autonomous piece updates).
    //    2. Figuring out falling and locking (scheduling / preparing autonomous piece updates).
    //    3. All other immediate button changes (easy).
    //
    // # Analysis of nontrivial autonomous-event updates (`PieceData.fall_or_lock_time`, `PieceData.move_scheduled`).
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

    // Pre-compute new direction of movement and projected next movement time.
    let (dx, next_move_time) = calc_move_dx_and_next_move_time(&new_state_buttons_pressed, button_update_time, config);

    // Prepare to maybe change the move_scheduled.
    let mut maybe_override_auto_move: Option<Option<GameTime>> = None;

    let mut new_piece = previous_piece_data.piece;
    use {ButtonChange as BC, Button as B};
    match button_change {
        // Hold.
        // - If succeeds, changes game action state to spawn different piece.
        // - Otherwise does nothing.
        BC::Press(B::HoldPiece) => {
            if let Some(new_phase) = try_hold(state, new_piece.tetromino, button_update_time) {
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
                feedback_msgs.push((button_update_time, Feedback::HardDrop(previous_piece_data.piece, new_piece)));
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
            maybe_override_auto_move = if rel_one || prs_ml || prs_mr {
//let/*TODO:dbg*/s=format!(" - moving\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                if let Some(moved_piece) = new_piece.fits_at(&state.board, (dx, 0)) {
                    new_piece = moved_piece;
                    Some(Some(next_move_time)) // Able to do relevant move; Insert autonomous movement.
                } else {
                    Some(None) // Unable to move; Remove autonomous movement.
                }
            } else if rel_mr || rel_ml {
                Some(None) // Buttons unpressed: Remove autonomous movement.
            } else {
                None // No relevant button state changes: Do not change autonomous movement.
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
    let new_auto_move_scheduled = if let Some(move_scheduled) = maybe_override_auto_move {
        // If we were in a case where movement was explicitly changed, do so.
        // This implements (³).
        move_scheduled

    } else if let Some((moved_piece, new_auto_move_scheduled)) = check_piece_became_movable_get_moved_piece_and_move_scheduled(previous_piece_data.piece, new_piece, &state.board, (dx, next_move_time)) {
        // Naïvely, movement direction should be kept;
        // But due to the system mentioned in (⁴), we do need to check
        // if the piece was stuck and became unstuck, and manually do a move in this case! 
        // (Also note: We use `(dx, next_move_time)` as computed from the *new* button state - but should not change, since this route is only triggered if the piece is able to move again and NOT because of a player move (`maybe_override_auto_move` is `None`).)
        new_piece = moved_piece;
        new_auto_move_scheduled

    } else {
        // All checks passed, no changes need to be made.
        // This is the case where neither (³) or (⁴) apply.
        previous_piece_data.auto_move_scheduled
    };

    // Update `lowest_y`, re-set `binding_lock_time` if applicable.
    let (new_lowest_y, new_binding_lock_time) = if new_piece.position.1 < previous_piece_data.lowest_y {
        (new_piece.position.1, button_update_time + lock_delay(state.gravity, Some(config.lock_time_max_factor)))
    } else {
        (previous_piece_data.lowest_y, previous_piece_data.binding_lock_time)
    };

    // Update `is_fall_not_lock`, i.e., whether we are falling (otherwise locking) now.
    // `new_is_fall_not_lock` is needed below.
    let new_is_fall_not_lock = new_piece.fits_at(&state.board, (0,-1)).is_some();

    // Update falltimer and locktimer.
    // See also (¹) and (²).
    let new_fall_or_lock_time = if new_is_fall_not_lock {
        // Calculate scheduled fall time.
        // This implements (¹).
        let was_airborne = previous_piece_data.piece.fits_at(&state.board, (0,-1)).is_some();
        if !was_airborne || matches!(button_change, BC::Press(B::DropSoft | B::DropHard)) {
            // Refresh fall timer if we *started* falling, or soft drop just pressed, or soft drop just released.
            let soft_drop_factor = matches!(button_change, BC::Press(B::DropSoft)).then_some(config.soft_drop_factor);
            button_update_time + fall_delay(state.gravity, soft_drop_factor)

        } else {
            // Falling as before.
            previous_piece_data.fall_or_lock_time
        }

    } else {
        // Calculate scheduled lock time.
        // This implements (²).
        if matches!(button_change, BC::Press(B::DropSoft | B::DropHard)) {
            // We are on the ground - if soft drop or hard drop pressed, lock immediately.
            button_update_time

        } else if new_piece != previous_piece_data.piece {
            // On the ground - Refresh lock time if piece moved.
            (button_update_time + lock_delay(state.gravity, None)).min(new_binding_lock_time)

        } else {
            // Previous lock time.
            previous_piece_data.fall_or_lock_time
        }
    };

    // 'Update' ActionState;
    // Return it to the main state machine with the latest acquired piece data.
    Phase::PieceInPlay {
        piece_data: PieceData {
            piece: new_piece,
            fall_or_lock_time: new_fall_or_lock_time,
            is_fall_not_lock: new_is_fall_not_lock,
            auto_move_scheduled: new_auto_move_scheduled,
            lowest_y: new_lowest_y,
            binding_lock_time: new_binding_lock_time,
        }
    }
}

fn calc_move_dx_and_next_move_time(buttons_pressed: &[Option<GameTime>; Button::VARIANTS.len()], move_time: GameTime, config: &Configuration) -> (isize, GameTime) {
    let (dx, how_long_relevant_direction_pressed) = match (
        buttons_pressed[Button::MoveLeft],
        buttons_pressed[Button::MoveRight],
    ) {
        (Some(time_prsd_left), Some(time_prsd_right)) =>
            match time_prsd_left.cmp(&time_prsd_right) {
                // 'Right' was pressed more recently, go right.
                std::cmp::Ordering::Less => (1, move_time.saturating_sub(time_prsd_right)),
                // Both pressed at exact same time, don't move.
                std::cmp::Ordering::Equal => (0, Duration::ZERO),
                // 'Left' was pressed more recently, go left.
                std::cmp::Ordering::Greater => (-1, move_time.saturating_sub(time_prsd_left)),
            }
        // Only 'left' pressed.
        (Some(time_prsd_left), None) => (-1, move_time.saturating_sub(time_prsd_left)),
        // Only 'right' pressed.
        (None, Some(time_prsd_right)) => (1, move_time.saturating_sub(time_prsd_right)),
        // None pressed. No movement.
        (None, None) => (0, Duration::ZERO),
    };

    let next_move_scheduled = move_time + if how_long_relevant_direction_pressed >= config.delayed_auto_shift {
        config.auto_repeat_rate
    } else {
        config.delayed_auto_shift
    };

//let/*TODO:dbg*/s=format!("OUTOF calc_move_dx_and_next_move_time ({dx:?}, {next_move_scheduled:?})\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
    (dx, next_move_scheduled)
}

fn check_piece_became_movable_get_moved_piece_and_move_scheduled(old_piece: Piece, new_piece: Piece, board: &Board, (dx, next_move_time): (isize, GameTime)) -> Option<(Piece, Option<GameTime>)> {
//let/*TODO:dbg*/s=format!("IN check_new_move_get_piece_and_move_scheduled ({next_move_time:?})\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
    // Do not check 'no movement'.
    if dx == 0 {
        return None;
    }
    let old_piece_moved = old_piece.fits_at(board, (dx, 0));
    let new_piece_moved = new_piece.fits_at(board, (dx, 0));
    if let (None, Some(moved_piece)) = (old_piece_moved, new_piece_moved) {
//let/*TODO:dbg*/s=format!(" - success\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
        Some((moved_piece, Some(next_move_time)))
    
    // All checks passed, no changes need to be made.
    // This is the case where neither (³) or (⁴) apply.
    } else {
//let/*TODO:dbg*/s=format!(" - fail\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
        None
    }
}

fn try_hold(state: &mut State, tetromino: Tetromino, hold_spawn_time: GameTime) -> Option<Phase> {
//let/*TODO:dbg*/s=format!("IN try_hold\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
    match state.hold_piece {
        // Nothing held yet, just hold spawned tetromino.
        None => {
//let/*TODO:dbg*/s=format!(" - success\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
            state.hold_piece = Some((tetromino, false));
            // Issue a spawn.
            Some(Phase::Spawning { spawn_time: hold_spawn_time })
        }
        // Swap spawned tetromino, push held back into next pieces queue.
        Some((held_tet, true)) => {
//let/*TODO:dbg*/s=format!(" - success\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
            state.hold_piece = Some((tetromino, false));
            // Cause the next spawn to specially be the piece we held.
            state.next_pieces.push_front(held_tet);
            // Issue a spawn.
            Some(Phase::Spawning { spawn_time: hold_spawn_time })
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
    state.pieces_locked[piece.tetromino as usize] += 1;

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
                    tetromino: piece.tetromino,
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

    // 'Update' ActionState;
    // Return it to the main state machine with all newly acquired piece data.
    if n_lines_cleared == 0 {
//let/*TODO:dbg*/s=format!("OUTOF do_lock {:?}\n", lock_time + config.appearance_delay);if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
        // No lines cleared, directly proceed to spawn.
        Phase::Spawning { spawn_time: lock_time + config.appearance_delay }
    } else {
        // Lines cleared, enter line clearing state.
        Phase::LinesClearing { lines_cleared_time: lock_time + config.line_clear_delay }
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
