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
        target_time: InGameTime,
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
            Some(ButtonChange::Press(button)) => {
                new_state_buttons_pressed[button] = Some(target_time)
            }
            Some(ButtonChange::Release(button)) => new_state_buttons_pressed[button] = None,
            None => {}
        }

        let mut feedback_msgs = Vec::new();

        // We linearly process all events until we reach the targeted update time.
        loop {
            // Maybe move on to game over if an end condition is met now.
            if let Some(new_phase) = self.try_end_game_if_end_condition_met() {
                self.phase = new_phase;
            }
            self.run_mods(
                UpdatePoint::MainLoopHead(&mut button_changes),
                &mut feedback_msgs,
            );

            match self.phase {
                // Game ended by now.
                // Return accumulated messages.
                Phase::GameEnd { .. } => {
                    //let/*TODO:dbg*/s=format!("# OUTOF update {target_time:?}, {button_changes:?}, {:?} {:?}\n", self.phase, self.state);if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                    return Ok(feedback_msgs);
                }

                // Lines clearing.
                // Move on to spawning.
                Phase::LinesClearing {
                    line_clears_finish_time,
                } if line_clears_finish_time <= target_time => {
                    //let/*TODO:dbg*/s=format!("INTO do_line_clearing ({line_clears_finish_time:?})\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                    self.phase = do_line_clearing(
                        &self.config,
                        &self.init_vals,
                        &mut self.state,
                        line_clears_finish_time,
                    );
                    self.state.time = line_clears_finish_time;

                    self.run_mods(UpdatePoint::LinesCleared, &mut feedback_msgs);
                }

                // Piece spawning.
                // - May move on to game over (BlockOut).
                // - Normally: Move on to piece-in-play.
                Phase::Spawning { spawn_time } if spawn_time <= target_time => {
                    self.phase = do_spawn(&self.config, &mut self.state, spawn_time);
                    self.state.time = spawn_time;

                    self.run_mods(UpdatePoint::PieceSpawned, &mut feedback_msgs);
                }

                // Piece autonomously moving / falling / locking.
                // - Locking may move on to game over (LockOut).
                Phase::PieceInPlay { piece_data }
                    if (piece_data.fall_or_lock_time <= target_time
                        || piece_data
                            .auto_move_scheduled
                            .is_some_and(|move_time| move_time <= target_time)) =>
                {
                    let mut flag = false;
                    if let Some(move_time) = piece_data.auto_move_scheduled {
                        if move_time <= piece_data.fall_or_lock_time && move_time <= target_time {
                            // Piece is moving autonomously and before next fall/lock.
                            flag = true;

                            //let/*TODO:dbg*/s=format!("INTO do_autonomous_move ({move_time:?})\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                            self.phase = do_autonomous_move(
                                &self.config,
                                &mut self.state,
                                piece_data,
                                move_time,
                            );
                            self.state.time = move_time;

                            self.run_mods(UpdatePoint::PieceAutoMoved, &mut feedback_msgs);
                        }
                    }
                    // else: Piece is not moving autonomously and instead falls or locks
                    if !flag {
                        if piece_data.is_fall_not_lock {
                            //let/*TODO:dbg*/s=format!("INTO do_fall ({:?})\n", piece_data.fall_or_lock_time);if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                            self.phase = do_fall(
                                &self.config,
                                &mut self.state,
                                piece_data,
                                piece_data.fall_or_lock_time,
                            );
                            self.state.time = piece_data.fall_or_lock_time;

                            self.run_mods(UpdatePoint::PieceFell, &mut feedback_msgs);
                        } else {
                            //let/*TODO:dbg*/s=format!("INTO do_lock ({:?})\n", piece_data.fall_or_lock_time);if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                            self.phase = do_lock(
                                &self.config,
                                &mut self.state,
                                piece_data.piece,
                                piece_data.fall_or_lock_time,
                                &mut feedback_msgs,
                            );
                            self.state.time = piece_data.fall_or_lock_time;

                            self.run_mods(UpdatePoint::PieceLocked, &mut feedback_msgs);
                        }
                    }
                }

                Phase::PieceInPlay { piece_data } if button_changes.is_some() => {
                    let Some(button_change) = button_changes.take() else {
                        unreachable!()
                    };
                    //let/*TODO:dbg*/s=format!("INTO do_player_button_update ({target_time:?})\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
                    self.phase = do_player_button_update(
                        &self.config,
                        &mut self.state,
                        piece_data,
                        button_change,
                        new_state_buttons_pressed,
                        target_time,
                        &mut feedback_msgs,
                    );
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
                    return Ok(feedback_msgs);
                }
            }
        }
    }

    /// Updates the internal `self.state.end` state, checking whether any [`Limits`] have been reached.
    #[allow(clippy::manual_map)]
    fn try_end_game_if_end_condition_met(&self) -> Option<Phase> {
        // Game already ended.
        if self.result().is_some() {
            None

        // Not ended yet, so check whether any end conditions have been met now and return appropriate phase if yes.
        } else if let Some(result) = self.config.end_conditions.iter().find_map(|(stat, good)| {
            self.check_stat_met(stat).then_some(if *good {
                Ok(*stat)
            } else {
                Err(GameOver::Limit(*stat))
            })
        }) {
            Some(Phase::GameEnd { result })
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

fn do_spawn(config: &Configuration, state: &mut State, spawn_time: InGameTime) -> Phase {
    //let/*TODO:dbg*/s=format!("IN do_spawn\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
    let [button_ml, button_mr, button_rl, button_rr, button_ra, _ds, _dh, _td, _tl, _tr, button_h] =
        state
            .buttons_pressed
            .map(|keydowntime| keydowntime.is_some());

    // Take a tetromino.
    let spawn_tet = state.piece_preview.pop_front().unwrap_or_else(|| {
        state
            .piece_generator
            .with_rng(&mut state.rng)
            .next()
            .expect("piece generator empty before game end")
    });

    // Only put back in if necessary (e.g. if piece_preview_count < next_pieces.len()).
    state.piece_preview.extend(
        state.piece_generator.with_rng(&mut state.rng).take(
            config
                .piece_preview_count
                .saturating_sub(state.piece_preview.len()),
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
        Tetromino::O => (4, Game::SKYLINE_HEIGHT),
        _ => (3, Game::SKYLINE_HEIGHT),
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
        if button_rr {
            turns += 1;
        }
        if button_ra {
            turns += 2;
        }
        if button_rl {
            turns += 3;
        }
    }

    // Rotation of 'raw' spawn piece.
    let rotated_spawn_piece = if turns != 0 {
        config
            .rotation_system
            .rotate(&raw_spawn_piece, &state.board, turns)
    } else {
        None
    };

    // Try finding `Some` valid spawn piece from the provided options in order.
    let spawn_piece = [
        rotated_spawn_piece,
        raw_spawn_piece
            .fits(&state.board)
            .then_some(raw_spawn_piece),
    ]
    .into_iter()
    .find_map(|option| option);

    // Return new piece-in-play state if piece can spawn, otherwise blockout (couldn't spawn).
    if let Some(piece) = spawn_piece {
        // We're falling if piece could move down.
        let is_fall_not_lock = piece.fits_at(&state.board, (0, -1)).is_some();

        let fall_or_lock_time = spawn_time.saturating_add(if is_fall_not_lock {
            // Fall immediately.
            Duration::ZERO
        } else {
            state.lock_delay.saturating_duration()
        });

        // Piece just spawned, lowest y = initial y.
        let lowest_y = piece.position.1;

        // Piece just spawned, standard full lock time max.
        let capped_lock_time = spawn_time.saturating_add(
            state
                .lock_delay
                .mul_ennf64(config.capped_lock_time_factor)
                .saturating_duration(),
        );

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
                capped_lock_time,
            },
        }
    } else {
        Phase::GameEnd {
            result: Err(GameOver::BlockOut),
        }
    }
}

fn do_line_clearing(
    config: &Configuration,
    init_vals: &InitialValues,
    state: &mut State,
    line_clears_finish_time: InGameTime,
) -> Phase {
    for y in (0..Game::HEIGHT).rev() {
        // Full line: move it to the cleared lines storage and push an empty line to the board.
        if state.board[y].iter().all(|tile| tile.is_some()) {
            // Starting from the offending line, we move down all others, then default the uppermost.
            state.board[y..].rotate_left(1);
            state.board[Game::HEIGHT - 1] = Line::default();
            state.lineclears += 1;

            // Increment level if update requested.
            if state.lineclears % config.update_delays_every_n_lineclears == 0 {
                // Calculate new fall- and lock delay for game state.
                (state.fall_delay, state.lock_delay) = calc_fall_and_lock_delay(
                    config,
                    init_vals,
                    state.fall_delay_lowerbound_hit_at_n_lineclears,
                    state.lineclears,
                );

                // Remember the first time fall delay hit zero.
                if state.fall_delay == config.fall_delay_lowerbound
                    && state.fall_delay_lowerbound_hit_at_n_lineclears.is_none()
                {
                    state.fall_delay_lowerbound_hit_at_n_lineclears = Some(state.lineclears);
                }
            }
        }
    }

    Phase::Spawning {
        spawn_time: line_clears_finish_time.saturating_add(config.spawn_delay),
    }
}

fn check_piece_became_movable_get_moved_piece_and_move_scheduled(
    old_piece: Piece,
    new_piece: Piece,
    board: &Board,
    (dx, next_move_time): (isize, InGameTime),
) -> Option<(Piece, Option<InGameTime>)> {
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

fn do_autonomous_move(
    config: &Configuration,
    state: &mut State,
    previous_piece_data: PieceData,
    auto_move_time: InGameTime,
) -> Phase {
    // Move piece and update all appropriate piece-related values.
    // NOTE: This should give non-zero `dx`.
    let (dx, next_move_time) =
        calc_move_dx_and_next_move_time(&state.buttons_pressed, auto_move_time, config);

    let mut new_piece = previous_piece_data.piece;
    let new_auto_move_scheduled =
        if let Some(moved_piece) = previous_piece_data.piece.fits_at(&state.board, (dx, 0)) {
            new_piece = moved_piece;
            Some(next_move_time) // Able to do relevant move; Insert autonomous movement.
        } else {
            None // Unable to move; Remove autonomous movement.
        };

    // Horizontal move could not have affected height, so it stays the same!
    let new_lowest_y = previous_piece_data.lowest_y;
    let new_capped_lock_time = previous_piece_data.capped_lock_time;

    let new_is_fall_not_lock = new_piece.fits_at(&state.board, (0, -1)).is_some();

    let new_fall_or_lock_time = if new_is_fall_not_lock {
        // Calculate scheduled fall time.
        // This implements (¹).
        let was_grounded = previous_piece_data
            .piece
            .fits_at(&state.board, (0, -1))
            .is_none();

        if was_grounded {
            // Refresh fall timer if we *started* falling.
            auto_move_time.saturating_add(
                if state.buttons_pressed[Button::DropSoft].is_some() {
                    state.fall_delay.div_ennf64(config.soft_drop_divisor)
                } else {
                    state.fall_delay
                }
                .saturating_duration(),
            )
        } else {
            // Falling as before.
            previous_piece_data.fall_or_lock_time
        }
    } else {
        // NOTE: capped_lock_time may actually lie in the past, so we first need to cap *it* from below (current time)!
        auto_move_time
            .max(new_capped_lock_time)
            .min(auto_move_time.saturating_add(state.lock_delay.saturating_duration()))
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
            capped_lock_time: new_capped_lock_time,
        },
    }
}

fn do_fall(
    config: &Configuration,
    state: &mut State,
    previous_piece_data: PieceData,
    fall_time: InGameTime,
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
    let new_auto_move_scheduled = if let Some((moved_piece, new_move_scheduled)) =
        check_piece_became_movable_get_moved_piece_and_move_scheduled(
            previous_piece_data.piece,
            new_piece,
            &state.board,
            calc_move_dx_and_next_move_time(&state.buttons_pressed, fall_time, config),
        ) {
        // Naïvely, movement direction should be kept;
        // But due to the system mentioned in (⁴), we do need to check
        // if the piece was stuck and became unstuck, and manually do a move in this case!
        new_piece = moved_piece;
        new_move_scheduled
    } else {
        // No changes need to be made.
        previous_piece_data.auto_move_scheduled
    };

    let (new_lowest_y, new_capped_lock_time) =
        if new_piece.position.1 < previous_piece_data.lowest_y {
            // Refresh position and capped_lock_time.
            (
                new_piece.position.1,
                fall_time.saturating_add(
                    state
                        .lock_delay
                        .mul_ennf64(config.capped_lock_time_factor)
                        .saturating_duration(),
                ),
            )
        } else {
            (
                previous_piece_data.lowest_y,
                previous_piece_data.capped_lock_time,
            )
        };

    let new_is_fall_not_lock = new_piece.fits_at(&state.board, (0, -1)).is_some();

    let new_fall_or_lock_time = if new_is_fall_not_lock {
        fall_time.saturating_add(
            if state.buttons_pressed[Button::DropSoft].is_some() {
                state.fall_delay.div_ennf64(config.soft_drop_divisor)
            } else {
                state.fall_delay
            }
            .saturating_duration(),
        )
    } else {
        // NOTE: capped_lock_time may actually lie in the past, so we first need to cap *it* from below (current time)!
        fall_time
            .max(new_capped_lock_time)
            .min(fall_time.saturating_add(state.lock_delay.saturating_duration()))
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
            capped_lock_time: new_capped_lock_time,
        },
    }
}

fn do_player_button_update(
    config: &Configuration,
    state: &mut State,
    previous_piece_data: PieceData,
    button_change: ButtonChange,
    new_state_buttons_pressed: [Option<InGameTime>; Button::VARIANTS.len()],
    button_update_time: InGameTime,
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
    // - zero locktimer  if  (grounded ~> grounded) + soft drop just pressed
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
    let (dx, next_move_time) =
        calc_move_dx_and_next_move_time(&new_state_buttons_pressed, button_update_time, config);

    // Prepare to maybe change the move_scheduled.
    let mut maybe_override_auto_move: Option<Option<InGameTime>> = None;

    let mut new_piece = previous_piece_data.piece;
    use {Button as B, ButtonChange as BC};
    match button_change {
        // Hold.
        // - If succeeds, changes game action state to spawn different piece.
        // - Otherwise does nothing.
        BC::Press(B::HoldPiece) => {
            if let Some(new_phase) = try_hold(state, new_piece.tetromino, button_update_time) {
                return new_phase;
            }
        }

        // Teleports.
        // Just instantly try to move piece all the way into applicable direction.
        BC::Press(dir @ (B::TeleDown | B::TeleLeft | B::TeleRight)) => {
            let offset = match dir {
                B::TeleDown => (0, -1),
                B::TeleLeft => (-1, 0),
                B::TeleRight => (1, 0),
                _ => unreachable!(),
            };
            new_piece = new_piece.teleported(&state.board, offset);
        }

        // Rotates.
        // Just instantly try to rotate piece into applicable direction.
        BC::Press(dir @ (B::RotateLeft | B::RotateRight | B::RotateAround)) => {
            let right_turns = match dir {
                B::RotateLeft => -1,
                B::RotateRight => 1,
                B::RotateAround => 2,
                _ => unreachable!(),
            };
            if let Some(rotated_piece) =
                config
                    .rotation_system
                    .rotate(&new_piece, &state.board, right_turns)
            {
                new_piece = rotated_piece;
            }
        }

        // Hard Drop.
        // Instantly try to move piece all the way down.
        // The locking is handled as part of a different check/system further.
        BC::Press(B::DropHard) => {
            new_piece = new_piece.teleported(&state.board, (0, -1));

            if config.feedback_verbosity != FeedbackVerbosity::Silent {
                feedback_msgs.push((
                    button_update_time,
                    Feedback::HardDrop {
                        old_piece: previous_piece_data.piece,
                        new_piece,
                    },
                ));
            }
        }

        // Soft Drop.
        // Instantly try to move piece one tile down.
        // The locking is handled as part of a different check/system further.
        BC::Press(B::DropSoft) => {
            if let Some(fallen_piece) = new_piece.fits_at(&state.board, (0, -1)) {
                new_piece = fallen_piece;
            }
        }

        // Moves and move releases.
        // These are very complicated and stateful.
        // The logic implemented here is based on (³) and the Karnaugh map in (⁵)
        BC::Press(dir @ (B::MoveLeft | B::MoveRight))
        | BC::Release(dir @ (B::MoveLeft | B::MoveRight)) => {
            let old_ml = state.buttons_pressed[B::MoveLeft].is_some();
            let old_mr = state.buttons_pressed[B::MoveRight].is_some();
            let is_prs = matches!(button_change, BC::Press(_));
            let is_ml = matches!(dir, B::MoveLeft);

            let rel_one = old_ml && old_mr && !is_prs; // ⇆₋←₊ⁱ, ⇆₋→₊ⁱ
            let prs_ml = !old_ml && is_prs && is_ml; // ←₊ⁱ; →₋←₊ⁱ
            let prs_mr = !old_mr && is_prs && !is_ml; // →₊ⁱ; ←₋→₊ⁱ
            let rel_ml = old_ml && !old_mr && !is_prs && is_ml; // ←₋ᵏ
            let rel_mr = !old_ml && old_mr && !is_prs && !is_ml; // →₋ᵏ
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
        }

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
            | B::HoldPiece,
        ) => {}
    }

    // Epilogue. Finalize state updates.

    // Update movetimer and rest of movement stuff.
    // See also (³) and (⁴).
    let new_auto_move_scheduled = if let Some(move_scheduled) = maybe_override_auto_move {
        // If we were in a case where movement was explicitly changed, do so.
        // This implements (³).
        move_scheduled
    } else if let Some((moved_piece, new_auto_move_scheduled)) =
        check_piece_became_movable_get_moved_piece_and_move_scheduled(
            previous_piece_data.piece,
            new_piece,
            &state.board,
            (dx, next_move_time),
        )
    {
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

    // Update `lowest_y`, re-set `capped_lock_time` if applicable.
    let (new_lowest_y, new_capped_lock_time) =
        if new_piece.position.1 < previous_piece_data.lowest_y {
            // Refresh position and capped_lock_time.
            (
                new_piece.position.1,
                button_update_time.saturating_add(
                    state
                        .lock_delay
                        .mul_ennf64(config.capped_lock_time_factor)
                        .saturating_duration(),
                ),
            )
        } else {
            (
                previous_piece_data.lowest_y,
                previous_piece_data.capped_lock_time,
            )
        };

    // Update `is_fall_not_lock`, i.e., whether we are falling (otherwise locking) now.
    // `new_is_fall_not_lock` is needed below.
    let new_is_fall_not_lock = new_piece.fits_at(&state.board, (0, -1)).is_some();

    let was_grounded = previous_piece_data
        .piece
        .fits_at(&state.board, (0, -1))
        .is_none();

    // Update falltimer and locktimer.
    // See also (¹) and (²).
    let new_fall_or_lock_time = if new_is_fall_not_lock {
        // Calculate scheduled fall time.
        // This implements (¹).
        let fall_reset = was_grounded
            || matches!(
                button_change,
                BC::Press(B::DropSoft) | BC::Release(B::DropSoft)
            );
        if fall_reset {
            // Refresh fall timer if we *started* falling, or soft drop just pressed, or soft drop just released.
            //let/*TODO:dbg*/s=format!("YEA\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
            button_update_time.saturating_add(
                if new_state_buttons_pressed[Button::DropSoft].is_some() {
                    state.fall_delay.div_ennf64(config.soft_drop_divisor)
                } else {
                    state.fall_delay
                }
                .saturating_duration(),
            )
        } else {
            // Falling as before.
            previous_piece_data.fall_or_lock_time
        }
    } else {
        // Calculate scheduled lock time.
        // This implements (²).
        let lock_immediately = matches!(button_change, BC::Press(B::DropHard))
            || (was_grounded && matches!(button_change, BC::Press(B::DropSoft)));
        let lock_reset_piecechange = new_piece != previous_piece_data.piece;
        let lock_reset_lenience = config.lenient_lock_delay_reset
            && matches!(
                button_change,
                BC::Press(
                    B::MoveLeft | B::MoveRight | B::RotateLeft | B::RotateAround | B::RotateRight
                )
            );

        if lock_immediately {
            // We are on the ground - if hard drop pressed or soft drop when ground is touched, lock immediately.
            button_update_time
        } else if lock_reset_lenience || lock_reset_piecechange {
            // On the ground - Refresh lock time if piece moved.
            // NOTE: capped_lock_time may actually lie in the past, so we first need to cap *it* from below (current time)!
            button_update_time
                .max(new_capped_lock_time)
                .min(button_update_time.saturating_add(state.lock_delay.saturating_duration()))
        } else {
            // Previous lock time.
            previous_piece_data.fall_or_lock_time
        }
    };
    //let/*TODO:dbg*/s=format!("THEN: {new_fall_or_lock_time:?}\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}

    // 'Update' ActionState;
    // Return it to the main state machine with the latest acquired piece data.
    Phase::PieceInPlay {
        piece_data: PieceData {
            piece: new_piece,
            fall_or_lock_time: new_fall_or_lock_time,
            is_fall_not_lock: new_is_fall_not_lock,
            auto_move_scheduled: new_auto_move_scheduled,
            lowest_y: new_lowest_y,
            capped_lock_time: new_capped_lock_time,
        },
    }
}

fn try_hold(
    state: &mut State,
    tetromino: Tetromino,
    new_piece_spawn_time: InGameTime,
) -> Option<Phase> {
    //let/*TODO:dbg*/s=format!("IN try_hold\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
    match state.hold_piece {
        // Nothing held yet, just hold spawned tetromino.
        None => {
            //let/*TODO:dbg*/s=format!(" - success\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
            state.hold_piece = Some((tetromino, false));
            // Issue a spawn.
            Some(Phase::Spawning {
                spawn_time: new_piece_spawn_time,
            })
        }
        // Swap spawned tetromino, push held back into next pieces queue.
        Some((held_tet, true)) => {
            //let/*TODO:dbg*/s=format!(" - success\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
            state.hold_piece = Some((tetromino, false));
            // Cause the next spawn to specially be the piece we held.
            state.piece_preview.push_front(held_tet);
            // Issue a spawn.
            Some(Phase::Spawning {
                spawn_time: new_piece_spawn_time,
            })
        }
        // Else can't hold, don't do anything.
        _ => None,
    }
}

fn do_lock(
    config: &Configuration,
    state: &mut State,
    piece: Piece,
    lock_time: InGameTime,
    feedback_msgs: &mut FeedbackMessages,
) -> Phase {
    if config.feedback_verbosity != FeedbackVerbosity::Silent {
        feedback_msgs.push((lock_time, Feedback::PieceLocked { piece }));
    }

    // Before board is changed, precompute whether a piece was 'spun' into position;
    // - 'Spun' pieces give higher score bonus.
    // - Only locked pieces can yield bonus (i.e. can't possibly move down).
    // - Only locked pieces clearing lines can yield bonus (i.e. can't possibly move left/right).
    // Thus, if a piece cannot move back up at lock time, it must have gotten there by rotation.
    // That's what a 'spin' is.
    let is_spin = piece.fits_at(&state.board, (0, 1)).is_none();

    // Locking.
    let mut entirely_above_skyline = true;
    for ((x, y), tile_type_id) in piece.tiles() {
        if y < Game::SKYLINE_HEIGHT {
            entirely_above_skyline = false;
        }

        // Set tile into board.
        state.board[y][x] = Some(tile_type_id);
    }

    // If all minos of the tetromino were locked entirely outside the `SKYLINE` bounding height, it's game over.
    if entirely_above_skyline {
        return Phase::GameEnd {
            result: Err(GameOver::LockOut),
        };
    }

    // Update tally of pieces_locked.
    state.pieces_locked[piece.tetromino as usize] += 1;

    // Score bonus calculation.

    // Find lines which might get cleared by piece locking. (actual clearing done later).
    let mut cleared_y_coords = Vec::<usize>::with_capacity(4);
    for y in (0..Game::HEIGHT).rev() {
        if state.board[y].iter().all(|mino| mino.is_some()) {
            cleared_y_coords.push(y);
        }
    }

    let lineclears = u32::try_from(cleared_y_coords.len()).unwrap();

    if lineclears == 0 {
        // If no lines cleared, no score bonus and combo is reset.
        state.consecutive_line_clears = 0;
    } else {
        // Increase combo.
        state.consecutive_line_clears += 1;

        let combo = state.consecutive_line_clears;

        let is_perfect_clear = state.board.iter().all(|line| {
            line.iter().all(|tile| tile.is_none()) || line.iter().all(|tile| tile.is_some())
        });

        // Compute main Score Bonus.
        let score_bonus =
            lineclears * if is_spin { 2 } else { 1 } * if is_perfect_clear { 4 } else { 1 } * 2 - 1
                + (combo - 1);

        // Update score.
        state.score += score_bonus;

        if config.feedback_verbosity != FeedbackVerbosity::Silent {
            feedback_msgs.push((
                lock_time,
                Feedback::LinesClearing {
                    y_coords: cleared_y_coords,
                    line_clear_start: config.line_clear_duration,
                },
            ));

            feedback_msgs.push((
                lock_time,
                Feedback::Accolade {
                    score_bonus,
                    tetromino: piece.tetromino,
                    is_spin,
                    lineclears,
                    is_perfect_clear,
                    combo,
                },
            ));
        }
    }

    // Update ability to hold piece.
    if let Some((_held_tet, swap_allowed)) = &mut state.hold_piece {
        *swap_allowed = true;
    }

    // 'Update' ActionState;
    // Return it to the main state machine with all newly acquired piece data.
    if lineclears == 0 {
        //let/*TODO:dbg*/s=format!("OUTOF do_lock {:?}\n", lock_time + config.spawn_delay);if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
        // No lines cleared, directly proceed to spawn.
        Phase::Spawning {
            spawn_time: lock_time.saturating_add(config.spawn_delay),
        }
    } else {
        // Lines cleared, enter line clearing state.
        Phase::LinesClearing {
            line_clears_finish_time: lock_time.saturating_add(config.line_clear_duration),
        }
    }
}

fn calc_move_dx_and_next_move_time(
    buttons_pressed: &[Option<InGameTime>; Button::VARIANTS.len()],
    move_time: InGameTime,
    config: &Configuration,
) -> (isize, InGameTime) {
    let (dx, how_long_relevant_direction_pressed) = match (
        buttons_pressed[Button::MoveLeft],
        buttons_pressed[Button::MoveRight],
    ) {
        (Some(time_prsd_left), Some(time_prsd_right)) => match time_prsd_left.cmp(&time_prsd_right)
        {
            // 'Right' was pressed more recently, go right.
            std::cmp::Ordering::Less => (1, move_time.saturating_sub(time_prsd_right)),
            // Both pressed at exact same time, don't move.
            std::cmp::Ordering::Equal => (0, Duration::ZERO),
            // 'Left' was pressed more recently, go left.
            std::cmp::Ordering::Greater => (-1, move_time.saturating_sub(time_prsd_left)),
        },
        // Only 'left' pressed.
        (Some(time_prsd_left), None) => (-1, move_time.saturating_sub(time_prsd_left)),
        // Only 'right' pressed.
        (None, Some(time_prsd_right)) => (1, move_time.saturating_sub(time_prsd_right)),
        // None pressed. No movement.
        (None, None) => (0, Duration::ZERO),
    };

    let next_move_scheduled = move_time.saturating_add(
        if how_long_relevant_direction_pressed >= config.delayed_auto_shift {
            config.auto_repeat_rate
        } else {
            config.delayed_auto_shift
        },
    );

    //let/*TODO:dbg*/s=format!("OUTOF calc_move_dx_and_next_move_time ({dx:?}, {next_move_scheduled:?})\n");if let Ok(f)=&mut std::fs::OpenOptions::new().append(true).open("dbg.txt"){let _=std::io::Write::write(f,s.as_bytes());}
    (dx, next_move_scheduled)
}

// Compute the fall and lock delay corresponding to the current lineclear progress.
fn calc_fall_and_lock_delay(
    config: &Configuration,
    init_vals: &InitialValues,
    fall_delay_lowerbound_hit_at_n_lineclears: Option<u32>,
    lineclears: u32,
) -> (ExtDuration, ExtDuration) {
    // Get some relevant values.
    let Configuration {
        fall_delay_equation,
        fall_delay_lowerbound,
        lock_delay_equation,
        lock_delay_lowerbound,
        ..
    } = config;
    let InitialValues {
        initial_fall_delay,
        initial_lock_delay,
        ..
    } = init_vals;

    if let Some(hit_at_n_lineclears) = fall_delay_lowerbound_hit_at_n_lineclears {
        // Fall delay zero was hit at some point, only decrease lock delay now.
        let lock_lineclears = f64::from(lineclears - hit_at_n_lineclears);
        let DelayEquation {
            factor: multiplier,
            subtrahend,
        } = lock_delay_equation;

        // Actually compute factor from equation.
        let lock_delay_factor =
            multiplier.get().powf(lock_lineclears) - subtrahend.get() * lock_lineclears;
        let lock_delay = initial_lock_delay
            .mul_ennf64(ExtNonNegF64::new(0.0f64.max(lock_delay_factor)).unwrap());

        (
            (*fall_delay_lowerbound),
            (*lock_delay_lowerbound).max(lock_delay),
        )
    } else {
        // Normally decrease fall delay.
        let DelayEquation {
            factor: multiplier,
            subtrahend,
        } = fall_delay_equation;
        let lineclears = f64::from(lineclears);

        // Actually compute factor from equation.
        let fall_delay_factor = multiplier.get().powf(lineclears) - subtrahend.get() * lineclears;
        let fall_delay = initial_fall_delay
            .mul_ennf64(ExtNonNegF64::new(0.0f64.max(fall_delay_factor)).unwrap());

        (
            (*fall_delay_lowerbound).max(fall_delay),
            (*initial_lock_delay),
        )
    }
}
