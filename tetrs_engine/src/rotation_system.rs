/*!
This module handles rotation of [`ActivePiece`]s.
*/

use crate::{ActivePiece, Board, Orientation, Tetromino};

/// Handles the logic of how to rotate a tetromino in play.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RotationSystem {
    /// The self-developed 'Ocular' rotation system.
    Ocular,
    /// The right-handed variant of the classic, kick-less rotation system used in NES Tetris.
    Classic,
    /// The Super Rotation System as used in the modern standard.
    Super,
}

impl RotationSystem {
    /// Tries to rotate a piece with the chosen `RotationSystem`.
    ///
    /// This will return `None` if the rotation is not possible, and `Some(p)` if the rotation
    /// succeeded with `p` as the new state of the piece.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tetrs_engine::*;
    /// # let game = Game::new(GameMode::marathon());
    /// # let empty_board = &game.state().board;
    /// let i_piece = ActivePiece { shape: Tetromino::I, orientation: Orientation::N, position: (0, 0) };
    ///
    /// // Rotate left once.
    /// let i_rotated = RotationSystem::Ocular.rotate(&i_piece, empty_board, -1);
    ///
    /// let i_expected = ActivePiece { shape: Tetromino::I, orientation: Orientation::W, position: (1, 0) };
    /// assert_eq!(i_rotated, Some(i_expected));
    /// ```
    pub fn rotate(
        &self,
        piece: &ActivePiece,
        board: &Board,
        right_turns: i8,
    ) -> Option<ActivePiece> {
        match self {
            RotationSystem::Ocular => ocular_rotate(piece, board, right_turns),
            RotationSystem::Classic => classic_rotate(piece, board, right_turns),
            RotationSystem::Super => super_rotate(piece, board, right_turns),
        }
    }
}

#[rustfmt::skip]
fn ocular_rotate(piece: &ActivePiece, board: &Board, right_turns: i8) -> Option<ActivePiece> {
    /*
    Symmetry notation : "OISZTLJ NESW ↺↻", and "-" means "mirror".
    [O N    ↺ ] is given:
        O N  ↻  = -O N  ↺
    [I NE   ↺ ] is given:
        I NE ↻  = -I NE ↺
    [S NE   ↺↻] is given:
        Z NE ↺↻ = -S NE ↻↺
    [T NESW ↺ ] is given:
        T NS ↻  = -T NS ↺
        T EW ↻  = -T WE ↺
    [L NESW ↺↻] is given:
        J NS ↺↻ = -L NS ↻↺
        J EW ↺↻ = -L WE ↻↺
    */
    use Orientation::*;
    match right_turns.rem_euclid(4) {
        // No rotation.
        0 => Some(*piece),
        // 180° rotation.
        2 => {
            let mut mirror = false;
            let mut shape = piece.shape;
            let mut orientation = piece.orientation;
            let mirrored_orientation = match orientation {
                N => N, E => W, S => S, W => E,
            };
            let kick_table = 'lookup_kicks: loop {
                break match shape {
                    Tetromino::O | Tetromino::I => &[( 0, 0)][..],
                    Tetromino::S => match orientation {
                        N | S => &[(-1,-1), ( 0, 0)][..],
                        E | W => &[( 1,-1), ( 0, 0)][..],
                    },
                    Tetromino::Z => {
                        shape = Tetromino::S;
                        mirror = true;
                        continue 'lookup_kicks
                    },
                    Tetromino::T => match orientation {
                        N => &[( 0,-1), ( 0, 0)][..],
                        E => &[(-1, 0), ( 0, 0), (-1,-1)][..],
                        S => &[( 0, 1), ( 0, 0), ( 0,-1)][..],
                        W => {
                            orientation = mirrored_orientation;
                            mirror = true;
                            continue 'lookup_kicks
                        },
                    },
                    Tetromino::L => match orientation {
                        N => &[( 0,-1), ( 1,-1), (-1,-1), ( 0, 0), ( 1, 0)][..],
                        E => &[(-1, 0), (-1,-1), ( 0, 0), ( 0,-1)][..],
                        S => &[( 0, 1), ( 0, 0), (-1, 1), (-1, 0)][..],
                        W => &[( 1, 0), ( 0, 0), ( 1,-1), ( 1, 1), ( 0, 1)][..],
                    },
                    Tetromino::J => {
                        shape = Tetromino::L;
                        orientation = mirrored_orientation;
                        mirror = true;
                        continue 'lookup_kicks
                    }
                }
            };
            piece.first_fit(board, kick_table.iter().copied().map(|(x, y)| if mirror { (-x, y) } else { (x, y) }), right_turns)
        }
        // 90° right/left rotation.
        rot => {
            let mut mirror = None;
            let mut shape = piece.shape;
            let mut orientation = piece.orientation;
            let mut left = rot == 3;
            let mirrored_orientation = match orientation {
                N => N, E => W, S => S, W => E,
            };
            let kick_table = 'lookup_kicks: loop {
                match shape {
                    Tetromino::O => {
                        if !left {
                            let mx = 0;
                            (mirror, left) = (Some(mx), !left);
                            continue 'lookup_kicks;
                        } else  {
                            break &[(-1, 0), (-1,-1), (-1, 1), ( 0, 0)][..];
                        }
                    },
                    Tetromino::I => {
                        if !left {
                            let mx = match orientation {
                                N | S => 3, E | W => -3,
                            };
                            (mirror, left) = (Some(mx), !left);
                            continue 'lookup_kicks;
                        } else  {
                            break match orientation {
                                N | S => &[( 1,-1), ( 1,-2), ( 1,-3), ( 0,-1), ( 0,-2), ( 0,-3), ( 1, 0), ( 0, 0), ( 2,-1), ( 2,-2)][..],
                                E | W => &[(-2, 1), (-3, 1), (-2, 0), (-3, 0), (-1, 1), (-1, 0), ( 0, 1), ( 0, 0)][..],
                            };
                        }
                    },
                    Tetromino::S => break match orientation {
                        N | S => if left { &[( 0, 0), ( 0,-1), ( 1, 0), (-1,-1)][..] }
                                    else { &[( 1, 0), ( 1,-1), ( 1, 1), ( 0, 0), ( 0,-1)][..] },
                        E | W => if left { &[(-1, 0), ( 0, 0), (-1,-1), (-1, 1), ( 0, 1)][..] }
                                    else { &[( 0, 0), (-1, 0), ( 0,-1), ( 1, 0), ( 0, 1), (-1, 1)][..] },
                    },
                    Tetromino::Z => {
                        let mx = match orientation {
                            N | S => 1, E | W => -1,
                        };
                        (mirror, shape, left) = (Some(mx), Tetromino::S, !left);
                        continue 'lookup_kicks;
                    },
                    Tetromino::T => {
                        if !left {
                            let mx = match orientation {
                                N | S => 1, E | W => -1,
                            };
                            (mirror, orientation, left) = (Some(mx), mirrored_orientation, !left);
                            continue 'lookup_kicks;
                        } else  {
                            break match orientation {
                                N => &[( 0,-1), ( 0, 0), (-1,-1), ( 1,-1), (-1,-2), ( 1, 0)][..],
                                E => &[(-1, 1), (-1, 0), ( 0, 1), ( 0, 0), (-1,-1), (-1, 2)][..],
                                S => &[( 1, 0), ( 0, 0), ( 1,-1), ( 0,-1), ( 1,-2), ( 2, 0)][..],
                                W => &[( 0, 0), (-1, 0), ( 0,-1), (-1,-1), ( 1,-1), ( 0, 1), (-1, 1)][..],
                            };
                        }
                    },
                    Tetromino::L => break match orientation {
                        N => if left { &[( 0,-1), ( 1,-1), ( 0,-2), ( 1,-2), ( 0, 0), ( 1, 0)][..] }
                                else { &[( 1,-1), ( 1, 0), ( 1,-1), ( 2, 0), ( 0,-1), ( 0, 0)][..] },
                        E => if left { &[(-1, 1), (-1, 0), (-2, 1), (-2, 0), ( 0, 0), ( 0, 1)][..] }
                                else { &[(-1, 0), ( 0, 0), ( 0,-1), (-1,-1), ( 0, 1), (-1, 1)][..] },
                        S => if left { &[( 1, 0), ( 0, 0), ( 1,-1), ( 0,-1), ( 0, 1), ( 1, 1)][..] }
                                else { &[( 0, 0), ( 0,-1), ( 1,-1), (-1,-1), ( 1, 0), (-1, 0), ( 0, 1)][..] },
                        W => if left { &[( 0, 0), (-1, 0), ( 0, 1), ( 1, 0), (-1, 1), ( 1, 1), ( 0,-1), (-1,-1)][..] }
                                else { &[( 0, 1), (-1, 1), ( 0, 0), (-1, 0), ( 0, 2), (-1, 2)][..] },
                    },
                    Tetromino::J => {
                        let mx = match orientation {
                            N | S => 1, E | W => -1,
                        };
                        (mirror, shape, orientation, left) = (Some(mx), Tetromino::L, mirrored_orientation, !left);
                        continue 'lookup_kicks;
                    }
                }
            };
            let kicks = kick_table.iter().copied().map(|(x, y)| if let Some(mx) = mirror { (mx - x, y) } else { (x, y) });
            piece.first_fit(board, kicks, right_turns)
        },
    }
}

fn super_rotate(piece: &ActivePiece, board: &Board, right_turns: i8) -> Option<ActivePiece> {
    let left = match right_turns.rem_euclid(4) {
        // No rotation occurred.
        0 => return Some(*piece),
        // One right rotation.
        1 => false,
        // Some 180 rotation I came up with.
        2 => {
            #[rustfmt::skip]
            let kick_table = match piece.shape {
                Tetromino::O | Tetromino::I | Tetromino::S | Tetromino::Z => &[(0, 0)][..],
                Tetromino::T | Tetromino::L | Tetromino::J => match piece.orientation {
                    N => &[( 0,-1), ( 0, 0)][..],
                    E => &[(-1, 0), ( 0, 0)][..],
                    S => &[( 0, 1), ( 0, 0)][..],
                    W => &[( 1, 0), ( 0, 0)][..],
                },
            };
            return piece.first_fit(board, kick_table.iter().copied(), 2);
        }
        // One left rotation.
        3 => true,
        _ => unreachable!(),
    };
    use Orientation::*;
    #[rustfmt::skip]
    let kick_table = match piece.shape {
        Tetromino::O => &[(0, 0)][..], // ⠶
        Tetromino::I => match piece.orientation {
            N => if left { &[( 1,-2), ( 0,-2), ( 3,-2), ( 0, 0), ( 3,-3)][..] }
                    else { &[( 2,-2), ( 0,-2), ( 3,-2), ( 0,-3), ( 3, 0)][..] },
            E => if left { &[(-2, 2), ( 0, 2), (-3, 2), ( 0, 3), (-3, 0)][..] }
                    else { &[( 2,-1), (-3, 1), ( 0, 1), (-3, 3), ( 0, 0)][..] },
            S => if left { &[( 2,-1), ( 3,-1), ( 0,-1), ( 3,-3), ( 0, 0)][..] }
                    else { &[( 1,-1), ( 3,-1), ( 0,-1), ( 3, 0), ( 0,-3)][..] },
            W => if left { &[(-1, 1), (-3, 1), ( 0, 1), (-3, 0), ( 0, 3)][..] }
                    else { &[(-1, 2), ( 0, 2), (-3, 2), ( 0, 0), (-3, 3)][..] },
        },
        Tetromino::S | Tetromino::Z | Tetromino::T | Tetromino::L | Tetromino::J => match piece.orientation {
            N => if left { &[( 0,-1), ( 1,-1), ( 1, 0), ( 0,-3), ( 1,-3)][..] }
                    else { &[( 1,-1), ( 0,-1), ( 0, 0), ( 1,-3), ( 0,-3)][..] },
            E => if left { &[(-1, 1), ( 0, 1), ( 0, 0), (-1, 3), ( 0, 3)][..] }
                    else { &[(-1, 0), ( 0, 0), ( 0,-1), (-1, 2), ( 0, 2)][..] },
            S => if left { &[( 1, 0), ( 0, 0), (-1, 1), ( 1,-2), ( 0,-2)][..] }
                    else { &[( 0, 0), ( 1, 0), ( 1, 1), ( 0,-2), ( 1,-2)][..] },
            W => if left { &[( 0, 0), (-1, 0), (-1,-1), ( 0, 2), (-1, 2)][..] }
                    else { &[( 0, 1), (-1, 1), (-1, 0), ( 0, 3), (-1, 3)][..] },
        },
    };
    piece.first_fit(board, kick_table.iter().copied(), right_turns)
}

fn classic_rotate(piece: &ActivePiece, board: &Board, right_turns: i8) -> Option<ActivePiece> {
    let left_rotation = match right_turns.rem_euclid(4) {
        // No rotation occurred.
        0 => return Some(*piece),
        // One right rotation.
        1 => false,
        // Classic didn't define 180 rotation, just check if the "default" 180 rotation fits.
        2 => {
            return piece.fits_at_reoriented(board, (0, 0), 2);
        }
        // One left rotation.
        3 => true,
        _ => unreachable!(),
    };
    use Orientation::*;
    #[rustfmt::skip]
    let kick = match piece.shape {
        Tetromino::O => (0, 0), // ⠶
        Tetromino::I => match piece.orientation {
            N | S => (2, -1), // ⠤⠤ -> ⡇
            E | W => (-2, 1), // ⡇  -> ⠤⠤
        },
        Tetromino::S | Tetromino::Z => match piece.orientation {
            N | S => (1, 0),  // ⠴⠂ -> ⠳  // ⠲⠄ -> ⠞
            E | W => (-1, 0), // ⠳  -> ⠴⠂ // ⠞  -> ⠲⠄
        },
        Tetromino::T | Tetromino::L | Tetromino::J => match piece.orientation {
            N => if left_rotation { ( 0,-1) } else { ( 1,-1) }, // ⠺  <- ⠴⠄ -> ⠗  // ⠹  <- ⠤⠆ -> ⠧  // ⠼  <- ⠦⠄ -> ⠏
            E => if left_rotation { (-1, 1) } else { (-1, 0) }, // ⠴⠄ <- ⠗  -> ⠲⠂ // ⠤⠆ <- ⠧  -> ⠖⠂ // ⠦⠄ <- ⠏  -> ⠒⠆
            S => if left_rotation { ( 1, 0) } else { ( 0, 0) }, // ⠗  <- ⠲⠂ -> ⠺  // ⠧  <- ⠖⠂ -> ⠹  // ⠏  <- ⠒⠆ -> ⠼
            W => if left_rotation { ( 0, 0) } else { ( 0, 1) }, // ⠲⠂ <- ⠺  -> ⠴⠄ // ⠖⠂ <- ⠹  -> ⠤⠆ // ⠒⠆ <- ⠼  -> ⠦⠄
        },
    };
    piece.fits_at_reoriented(board, kick, right_turns)
}
