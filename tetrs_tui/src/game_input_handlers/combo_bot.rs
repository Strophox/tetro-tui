#![allow(clippy::just_underscores_and_digits)]

use std::{
    collections::{HashSet, VecDeque},
    fmt::Debug,
    fs::File,
    io::Write,
    sync::mpsc::{self, Receiver, RecvError, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
    vec,
};

use tetrs_engine::{Button, Game, Tetromino};

use crate::game_input_handlers::InputOrInterrupt;

type ButtonInstructions = &'static [Button];
type Layout = (Pat, bool);

const GRAPHVIZ: bool = cfg!(feature = "graphviz");
const GRAPHVIZ_FILENAME: &str = "combot_graphviz_log.txt";

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
enum Pat {
    /// `█▀  `
    _200,
    /// `█  ▄`
    _137,
    /// `█▄  `
    _140,
    /// `▄▄▄ `
    _14,
    /// `▄   `++`█   `
    _2184,
    /// `▄▄ ▄`
    _13,
    /// `▄▄ ▀`
    _28,
    /// `▀█  `
    _196,
    /// `█ ▄ `
    _138,
    /// `▄█  `
    _76,
    /// `▀▄▄ `
    _134,
    /// `▀▄ ▄`
    _133,
    /// `▄▀ ▄`
    _73,
    /// `▄▀▀ `
    _104,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug)]
pub struct ComboState {
    layout: Layout,
    active: Option<Tetromino>,
    hold: Option<(Tetromino, bool)>,
    next_pieces: u128,
    depth: usize,
}

#[derive(Debug)]
pub struct ComboBotHandler {
    _handle: JoinHandle<()>,
}

impl ComboBotHandler {
    pub fn new(
        button_sender: &Sender<InputOrInterrupt>,
        action_idle_time: Duration,
    ) -> (Self, Sender<ComboState>) {
        let (state_sender, state_receiver) = mpsc::channel();
        let join_handle = Self::spawn(state_receiver, button_sender.clone(), action_idle_time);
        let combo_bot_handler = ComboBotHandler {
            _handle: join_handle,
        };
        (combo_bot_handler, state_sender)
    }

    pub fn encode(game: &Game) -> Result<ComboState, String> {
        let row0 = &game.state().board[0][3..=6];
        let row1 = &game.state().board[1][3..=6];
        let row2 = &game.state().board[2][3..=6];
        let pattern_bits = row2
            .iter()
            .chain(row1.iter())
            .chain(row0.iter())
            .fold(0, |bits, cell| bits << 1 | i32::from(cell.is_some()));
        let pattern = match pattern_bits {
            200 | 49 => Pat::_200,
            137 | 25 => Pat::_137,
            140 | 19 => Pat::_140,
            14 | 7 => Pat::_14,
            2184 | 273 => Pat::_2184,
            13 | 11 => Pat::_13,
            28 | 131 => Pat::_28,
            196 | 50 => Pat::_196,
            138 | 21 => Pat::_138,
            76 | 35 => Pat::_76,
            134 | 22 => Pat::_134,
            133 | 26 => Pat::_133,
            73 | 41 => Pat::_73,
            104 | 97 => Pat::_104,
            _ => return Err(format!("row0 = {row0:?}, row1 = {row1:?}, row2 = {row2:?}, pattern_bits = {pattern_bits:?}")),
        };
        let flipped = ![
            200, 137, 140, 14, 2184, 13, 28, 196, 138, 76, 134, 133, 73, 104,
        ]
        .contains(&pattern_bits);
        const MAX_LOOKAHEAD: usize = 42;
        if game.state().next_pieces.len() > MAX_LOOKAHEAD {
            return Err(format!(
                "game.state().next_pieces.len()={} > MAX_LOOKAHEAD={}",
                game.state().next_pieces.len(),
                MAX_LOOKAHEAD
            ));
        }
        Ok(ComboState {
            layout: (pattern, flipped),
            active: Some(game.state().active_piece_data.unwrap().0.shape),
            hold: game.state().hold_piece,
            next_pieces: Self::encode_next_queue(
                game.state().next_pieces.iter().take(MAX_LOOKAHEAD),
            ),
            depth: 0,
        })
    }

    fn encode_next_queue<'a>(tetrominos: impl DoubleEndedIterator<Item = &'a Tetromino>) -> u128 {
        use Tetromino::*;
        tetrominos.into_iter().rev().fold(0, |bits, tet| {
            bits << 3
                | (match tet {
                    O => 0,
                    I => 1,
                    S => 2,
                    Z => 3,
                    T => 4,
                    L => 5,
                    J => 6,
                } + 1)
        })
    }

    fn spawn(
        state_receiver: Receiver<ComboState>,
        button_sender: Sender<InputOrInterrupt>,
        idle_time: Duration,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            'react_to_game: loop {
                match state_receiver.recv() {
                    Ok(state_lvl0) => {
                        /*TBD: Remove debug: let s=format!("[ main1 REVOYYY zeroth_state = {state_lvl0:?} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
                        let (states_lvl1, states_lvl1_buttons): (
                            Vec<ComboState>,
                            Vec<ButtonInstructions>,
                        ) = neighbors(state_lvl0).into_iter().unzip();
                        /*TBD: Remove debug: let s=format!("[ main2 states_lvl1 = {:?} = {states_lvl1:?} ]\n", states_lvl1.iter().map(|state| fmt_statenode(&(0, *state))).collect::<Vec<_>>());let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
                        // No more options to continue.
                        let Some(branch_choice) =
                            choose_branch(states_lvl1, GRAPHVIZ.then_some(state_lvl0))
                        else {
                            /*TBD: Remove debug: let s=format!("[ main3 uhhhhhh ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
                            let _ = button_sender
                                .send(Err(crate::game_input_handlers::Interrupt::Pause));
                            break 'react_to_game;
                        };
                        for mut button in states_lvl1_buttons[branch_choice].iter().copied() {
                            // Need to manually flip instructions if original position was a flipped one.
                            if state_lvl0.layout.1 {
                                button = match button {
                                    Button::MoveLeft => Button::MoveRight,
                                    Button::MoveRight => Button::MoveLeft,
                                    Button::RotateLeft => Button::RotateRight,
                                    Button::RotateRight => Button::RotateLeft,
                                    Button::RotateAround
                                    | Button::DropSoft
                                    | Button::DropHard
                                    | Button::DropSonic
                                    | Button::Hold => button,
                                };
                            }
                            let _ = button_sender.send(Ok((Instant::now(), button, true)));
                            let _ = button_sender.send(Ok((Instant::now(), button, false)));
                            /*TBD: Remove debug: let s=format!("[ main4 SENT button = {button:?} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
                            thread::sleep(idle_time);
                        }
                    }
                    // No more state updates will be received, stop thread.
                    Err(RecvError) => break 'react_to_game,
                }
            }
        })
    }
}

fn choose_branch(
    states_lvl1: Vec<ComboState>,
    debug_state_lvl0: Option<ComboState>,
) -> Option<usize> {
    /*TBD: Remove debug: let s=format!("[ chbr1 examine states = {states_lvl1:?} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
    if states_lvl1.is_empty() {
        /*TBD: Remove debug: let s=format!("[ chbr2 empty ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        None
    // One option to continue, do not do further analysis.
    } else if states_lvl1.len() == 1 {
        /*TBD: Remove debug: let s=format!("[ chbr single ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        Some(0)
    // Several options to evaluate, do graph algorithm.
    } else {
        /*TBD: Remove debug: let s=format!("[ chbr multianalyze ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        let num_states = states_lvl1.len();
        let mut queue: VecDeque<(usize, ComboState)> =
            states_lvl1.into_iter().enumerate().collect();
        let mut graphviz_str = String::new();
        if let Some(state_lvl0) = debug_state_lvl0 {
            graphviz_str.push_str("strict digraph {\n");
            graphviz_str.push_str(&format!("\"{}\"\n", fmt_statenode(&(0, state_lvl0))));
            for statenode in queue.iter() {
                graphviz_str.push_str(&format!(
                    "\"{}\" -> \"{}\"\n",
                    fmt_statenode(&(0, state_lvl0)),
                    fmt_statenode(statenode)
                ));
            }
        }
        let mut depth_best = queue.iter().map(|(_, state)| state.depth).max().unwrap();
        let mut states_best = queue
            .iter()
            .filter(|(_, state)| state.depth == depth_best)
            .copied()
            .collect::<Vec<_>>();
        /*TBD: Remove debug: let s=format!("[ chbr before-while ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        while let Some(statenode @ (branch, state)) = queue.pop_front() {
            let neighbors: Vec<_> = neighbors(state)
                .into_iter()
                .map(|(state, _)| (branch, state))
                .collect();
            if debug_state_lvl0.is_some() {
                for state in neighbors.iter() {
                    graphviz_str.push_str(&format!(
                        "\"{}\" -> \"{}\"\n",
                        fmt_statenode(&statenode),
                        fmt_statenode(state)
                    ));
                }
            }
            for neighbor in neighbors.iter() {
                let depth = neighbor.1.depth;
                use std::cmp::Ordering::*;
                match depth_best.cmp(&depth) {
                    Less => {
                        depth_best = depth;
                        states_best.clear();
                        states_best.push(*neighbor);
                    }
                    Equal => {
                        states_best.push(*neighbor);
                    }
                    Greater => {}
                }
            }
            queue.extend(neighbors);
        }
        /*TBD: Remove debug: let s=format!("[ chbr depth_best = {depth_best} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        if debug_state_lvl0.is_some() {
            graphviz_str.push_str("\n}");

            let _ = File::options()
                .create(true)
                .append(true)
                .open(GRAPHVIZ_FILENAME)
                .unwrap()
                .write(format!("graphviz: \"\"\"\n{graphviz_str}\n\"\"\"\n").as_bytes());
        }
        /*TBD: Remove debug: let s=format!("[ chbr states_best = {states_best:?} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        //states_lvlx.sort_by_key(|(_, ComboState { layout, .. })| layout.0);
        //let best = states_lvlx.first().unwrap().0;
        let mut sets = vec![HashSet::<Layout>::new(); num_states];
        for (branch, state) in states_best {
            sets[branch].insert(state.layout);
            if let Some((held, true)) = state.hold {
                sets[branch].extend(
                    reachable_with(state.layout, held)
                        .iter()
                        .map(|(layout, _)| layout),
                );
            }
        }
        // let best = (0..num_states).max_by_key(|branch| sets[*branch].len()).unwrap();
        let layout_heuristic = |(pat, _): &Layout| {
            use Pat::*;
            match pat {
                _200 => 8,
                _137 => 8,
                _140 => 7,
                _14 => 6,
                _2184 => 4,
                _13 => 6,
                _28 => 6,
                _196 => 4,
                _138 => 4,
                _76 => 3,
                _134 => 3,
                _133 => 3,
                _73 => 2,
                _104 => 2,
            }
        };
        let best = (0..num_states)
            .max_by_key(|branch| {
                let val = sets[*branch].iter().map(layout_heuristic).sum::<u32>();
                /*TBD: Remove debug: let s=format!("[ chbr branch = {branch}, val = {val}\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
                val
            })
            .unwrap();
        /*NOTE: Old, maybe one should benchmark this again, but seems worse.
        #[rustfmt::skip]
        let layout_heuristic = |(pat, flipped): &Layout| -> (u8, u32) {
            let flip = *flipped;
            use Pat::*;
            match pat {
                _200  => (if flip { 0b011_1111 } else { 0b101_1111 }, 8),
                _137  => (if flip { 0b111_0111 } else { 0b111_1011 }, 8),
                _140  => (if flip { 0b111_1011 } else { 0b111_0111 }, 7),
                _14   => (if flip { 0b111_0110 } else { 0b111_1010 }, 6),
                _2184 => (if flip { 0b111_0010 } else { 0b111_0010 }, 4),
                _13   => (if flip { 0b101_1010 } else { 0b011_0110 }, 6),
                _28   => (if flip { 0b111_1010 } else { 0b111_0110 }, 6),
                _196  => (if flip { 0b001_1011 } else { 0b001_0111 }, 4),
                _138  => (if flip { 0b110_0010 } else { 0b110_0010 }, 4),
                _76   => (if flip { 0b100_0011 } else { 0b010_0011 }, 3),
                _134  => (if flip { 0b110_0010 } else { 0b110_0010 }, 3),
                _133  => (if flip { 0b011_0010 } else { 0b101_0010 }, 3),
                _73   => (if flip { 0b000_0110 } else { 0b000_1010 }, 2),
                _104  => (if flip { 0b010_0010 } else { 0b100_0010 }, 2),
            }
        };
        let best = (0..num_states)
            .max_by_key(|branch| {
                let val = sets[*branch].iter().map(layout_heuristic).reduce(|(piecety0, cont0), (piecety1, cont1)| (piecety0 | piecety1, cont0 + cont1));
                /*TBD: Remove debug: let s=format!("[ chbr branch = {branch}, val = {val}\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
                val
            })
            .unwrap();
        */
        /*TBD: Remove debug: let s=format!("[ chbr best = {best:?} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        Some(best)
    }
}

fn neighbors(
    ComboState {
        depth,
        layout,
        active,
        hold,
        next_pieces,
    }: ComboState,
) -> Vec<(ComboState, ButtonInstructions)> {
    /*TBD: Remove debug: let s=format!("[ nbrs1 entered ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
    let mut neighbors = Vec::new();
    let Some(active) = active else {
        /*TBD: Remove debug: let s=format!("[ nbrs2 early-ret ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        return neighbors;
    };
    let new_active = (next_pieces != 0)
        .then(|| Tetromino::SHAPES[usize::try_from(next_pieces & 0b111).unwrap() - 1]);
    let new_next_pieces = next_pieces >> 3;
    // Add neighbors reachable with just holding / swapping with the active piece.
    if let Some((held, swap_allowed)) = hold {
        if swap_allowed {
            neighbors.push((
                ComboState {
                    layout,
                    active: Some(held),
                    hold: Some((active, false)),
                    next_pieces,
                    depth,
                },
                &[Button::Hold][..],
            ));
        }
    } else {
        neighbors.push((
            ComboState {
                layout,
                active: new_active,
                hold: Some((active, false)),
                next_pieces,
                depth,
            },
            &[Button::Hold][..],
        ));
    }
    neighbors.extend(
        reachable_with(layout, active)
            .into_iter()
            .map(|(next_layout, buttons)| {
                (
                    ComboState {
                        layout: next_layout,
                        active: new_active,
                        hold: hold.map(|(held, _swap_allowed)| (held, true)),
                        next_pieces: new_next_pieces,
                        depth: depth + 1,
                    },
                    buttons,
                )
            }),
    );
    neighbors
}

#[rustfmt::skip]
fn reachable_with((pattern, flip): Layout, mut shape: Tetromino) -> Vec<(Layout, ButtonInstructions)> {
    use Tetromino::*;
    if flip {
        shape = match shape {
            O => O,
            I => I,
            S => Z,
            Z => S,
            T => T,
            L => J,
            J => L,
        };
    }
    use Button::*;
    match pattern {
        // "█▀  "
        Pat::_200 => match shape {
            T => vec![((Pat::_137, !flip), &[RotateLeft, MoveRight, MoveRight, DropHard][..]),
                      ((Pat::_14, flip), &[RotateLeft, MoveRight, MoveRight, DropSonic, RotateRight, DropSoft][..])],
            L => vec![((Pat::_13, flip), &[RotateLeft, MoveRight, MoveRight, DropSonic, RotateRight, DropSoft][..])],
            S => vec![((Pat::_14, flip), &[RotateRight, MoveRight, DropSonic, RotateRight, DropSoft][..]),
                      ((Pat::_73, !flip), &[RotateRight, MoveRight, DropHard][..])],
            Z => vec![((Pat::_133, !flip), &[RotateRight, MoveRight, DropHard][..]),
                      ((Pat::_104, flip), &[MoveRight, DropHard][..])],
            O => vec![((Pat::_13, !flip), &[MoveRight, MoveRight, DropHard][..])],
            I => vec![((Pat::_200, flip), &[DropHard][..])],
            _ => vec![],
        },
        // "█  ▄"
        Pat::_137 => match shape {
            T => vec![((Pat::_13, !flip), &[RotateRight, RotateRight, MoveRight, DropHard][..]),
                      ((Pat::_73, !flip), &[MoveRight, DropHard][..])],
            L => vec![((Pat::_137, !flip), &[MoveRight, DropHard][..]),
                      ((Pat::_13, flip), &[RotateRight, RotateRight, MoveRight, DropHard][..]),
                      ((Pat::_76, flip), &[RotateRight, MoveRight, MoveLeft, DropHard][..])],
            J => vec![((Pat::_73, flip), &[MoveRight, DropHard][..])],
            S => vec![((Pat::_13, !flip), &[MoveRight, DropHard][..])],
            O => vec![((Pat::_14, flip), &[DropHard][..])],
            I => vec![((Pat::_137, flip), &[DropHard][..])],
            _ => vec![],
        },
        // "█▄  "
        Pat::_140 => match shape {
            T => vec![((Pat::_14, flip), &[RotateRight, RotateRight, MoveRight, DropHard][..])],
            L => vec![((Pat::_28, flip), &[MoveRight, DropHard][..])],
            J => vec![((Pat::_137, !flip), &[RotateLeft, MoveRight, MoveRight, DropHard][..]),
                      ((Pat::_13, flip), &[RotateRight, RotateRight, MoveRight, DropHard][..]),
                      ((Pat::_76, flip), &[MoveRight, DropHard][..])],
            Z => vec![((Pat::_14, flip), &[MoveRight, DropHard][..])],
            O => vec![((Pat::_13, !flip), &[MoveRight, DropHard][..])],
            I => vec![((Pat::_140, flip), &[DropHard][..])],
            _ => vec![],
        },
        // "▄▄▄ "
        Pat::_14 => match shape {
            T => vec![((Pat::_140, !flip), &[RotateLeft, MoveRight, MoveRight, DropHard][..])],
            L => vec![((Pat::_200, !flip), &[RotateLeft, MoveRight, MoveRight, DropHard][..])],
            J => vec![((Pat::_14, !flip), &[RotateRight, RotateRight, MoveRight, DropHard][..])],
            S => vec![((Pat::_76, !flip), &[RotateRight, MoveRight, DropHard][..])],
            I => vec![((Pat::_2184, !flip), &[RotateRight, MoveRight, DropHard][..]),
                      ((Pat::_14, flip), &[DropHard][..])],
            _ => vec![],
        },
        // "▄ "++"█   "
        Pat::_2184 => match shape {
            T => vec![((Pat::_138, flip), &[MoveRight, DropHard][..])],
            L => vec![((Pat::_137, flip), &[MoveRight, DropHard][..]),
                      ((Pat::_140, flip), &[RotateRight, RotateRight, MoveRight, DropHard][..])],
            J => vec![((Pat::_137, flip), &[RotateRight, RotateRight, MoveRight, DropHard][..]),
                      ((Pat::_140, flip), &[MoveRight, DropHard][..])],
            I => vec![((Pat::_2184, flip), &[DropHard][..])],
            _ => vec![],
        },
        // "▄▄ ▄"
        Pat::_13 => match shape {
            T => vec![((Pat::_14, !flip), &[RotateRight, RotateRight, MoveRight, DropHard][..]),
                      ((Pat::_76, !flip), &[RotateRight, MoveRight, DropHard][..])],
            J => vec![((Pat::_14, flip), &[RotateRight, RotateRight, MoveLeft/***/, DropHard][..]),
                      ((Pat::_196, !flip), &[RotateRight, MoveRight, DropHard][..])],
            Z => vec![((Pat::_140, !flip), &[RotateRight, MoveRight, DropHard][..])],
            I => vec![((Pat::_13, flip), &[DropHard][..])],
            _ => vec![],
        },
        // "▄▄ ▀"
        Pat::_28 => match shape {
            T => vec![((Pat::_76, flip), &[MoveLeft/***/, DropHard][..])],
            L => vec![((Pat::_76, !flip), &[RotateLeft, MoveRight, MoveRight, MoveLeft, DropSonic, RotateAround, DropSoft][..])], // SPECIAL: 180°
            J => vec![((Pat::_140, flip), &[MoveLeft/***/, DropHard][..]),
                      ((Pat::_14, flip), &[RotateRight, RotateRight, MoveLeft/***/, DropHard][..])],
            Z => vec![((Pat::_14, !flip), &[RotateRight, MoveRight, DropSonic, RotateLeft, DropSoft][..])],
            I => vec![((Pat::_28, flip), &[DropHard][..])],
            _ => vec![],
        },
        // "▀█  "
        Pat::_196 => match shape {
            T => vec![((Pat::_138, !flip), &[RotateLeft, MoveRight, MoveRight, DropHard][..])],
            Z => vec![((Pat::_134, !flip), &[RotateRight, MoveRight, DropHard][..])],
            O => vec![((Pat::_14, !flip), &[MoveRight, DropHard][..])],
            I => vec![((Pat::_196, flip), &[DropHard][..])],
            _ => vec![],  
        },
        // "█ ▄ "
        Pat::_138 => match shape {
            L => vec![((Pat::_14, flip), &[RotateRight, RotateRight, MoveRight, DropHard][..]),
                      ((Pat::_133, !flip), &[MoveRight, DropHard][..])],
            J => vec![((Pat::_13, !flip), &[RotateRight, RotateRight, MoveRight, DropHard][..])],
            I => vec![((Pat::_138, flip), &[DropHard][..])],
            _ => vec![], 
        },
        // "▄█  "
        Pat::_76 => match shape {
            J => vec![((Pat::_138, !flip), &[RotateLeft, MoveRight, MoveRight, DropHard][..])],
            O => vec![((Pat::_14, !flip), &[MoveRight, DropHard][..])],
            I => vec![((Pat::_76, flip), &[DropHard][..])],
            _ => vec![],
        },
        // "▀▄▄ "
        Pat::_134 => match shape {
            L => vec![((Pat::_134, !flip), &[MoveRight, DropHard][..])],
            J => vec![((Pat::_14, !flip), &[RotateRight, RotateRight, MoveRight, DropHard][..])],
            I => vec![((Pat::_134, flip), &[DropHard][..])],
            _ => vec![],
        },
        // "▀▄ ▄"
        Pat::_133 => match shape {
            T => vec![((Pat::_14, !flip), &[RotateRight, RotateRight, MoveRight, DropHard][..])],
            L => vec![((Pat::_138, !flip), &[MoveRight, DropHard][..])],
            I => vec![((Pat::_133, flip), &[DropHard][..])],
            _ => vec![],
        },
        // "▄▀ ▄"
        Pat::_73 => match shape {
            S => vec![((Pat::_14, !flip), &[RotateRight, MoveRight, MoveLeft, DropSonic, RotateRight, DropSoft][..])],
            I => vec![((Pat::_73, flip), &[DropHard][..])],
            _ => vec![],
        },
        // "▄▀▀ "
        Pat::_104 => match shape {
            L => vec![((Pat::_14, !flip), &[RotateLeft, MoveRight, MoveRight, DropSonic, RotateRight, DropHard][..])],
            I => vec![((Pat::_104, flip), &[DropHard][..])],
            _ => vec![],
        },
    }
}

pub fn fmt_statenode(
    (
        id,
        ComboState {
            layout,
            active,
            hold,
            next_pieces,
            depth,
        },
    ): &(usize, ComboState),
) -> String {
    let layout = match layout {
        (Pat::_200, false) => "▛ ",
        (Pat::_200, true) => " ▜",
        (Pat::_137, false) => "▌▗",
        (Pat::_137, true) => "▖▐",
        (Pat::_140, false) => "▙ ",
        (Pat::_140, true) => " ▟",
        (Pat::_14, false) => "▄▖",
        (Pat::_14, true) => "▗▄",
        (Pat::_2184, false) => "▌ ",
        (Pat::_2184, true) => " ▐",
        (Pat::_13, false) => "▄▗",
        (Pat::_13, true) => "▖▄",
        (Pat::_28, false) => "▄▝",
        (Pat::_28, true) => "▘▄",
        (Pat::_196, false) => "▜ ",
        (Pat::_196, true) => " ▛",
        (Pat::_138, false) => "▌▖",
        (Pat::_138, true) => "▗▐",
        (Pat::_76, false) => "▟ ",
        (Pat::_76, true) => " ▙",
        (Pat::_134, false) => "▚▖",
        (Pat::_134, true) => "▗▞",
        (Pat::_133, false) => "▚▗",
        (Pat::_133, true) => "▖▞",
        (Pat::_73, false) => "▞▗",
        (Pat::_73, true) => "▖▚",
        (Pat::_104, false) => "▞▘",
        (Pat::_104, true) => "▝▚",
    };
    let mut next_pieces_str = String::new();
    let mut next_pieces = *next_pieces;
    while next_pieces != 0 {
        next_pieces_str.push_str(&format!(
            "{:?}",
            Tetromino::SHAPES[usize::try_from(next_pieces & 0b111).unwrap() - 1]
        ));
        next_pieces >>= 3;
    }
    let active_str = if let Some(tet) = active {
        format!("{tet:?}")
    } else {
        "".to_string()
    };
    let hold_str = if let Some((tet, swap_allowed)) = hold {
        format!("({}{tet:?})", if *swap_allowed { "" } else { "-" })
    } else {
        "".to_string()
    };
    format!("{id}.{depth}{layout}{active_str}{hold_str}{next_pieces_str}")
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, num::NonZeroU32};

    use super::*;
    use tetrs_engine::piece_generation::{TetrominoIterator, TetrominoSource};

    const COMBO_MAX: usize = 1_000_000;

    #[test]
    fn benchmark_simple() {
        let sample_count = 1_000;
        let lookahead = 8;
        let randomizer = (TetrominoSource::bag(), "bag");
        run_analyses_on(sample_count, std::iter::once((lookahead, randomizer)));
    }

    #[test]
    fn benchmark_lookaheads() {
        let sample_count = 100_000;
        let lookaheads = 1..6;
        let randomizer = (TetrominoSource::recency(), "recency");
        run_analyses_on(sample_count, lookaheads.zip(std::iter::repeat(randomizer)));
    }

    #[test]
    fn benchmark_randomizers() {
        let sample_count = 100_000;
        let lookahead = 3;
        #[rustfmt::skip]
        let randomizers = [
            (TetrominoSource::uniform(), "uniform"),
            (TetrominoSource::balance_relative(), "balance-relative"),
            (TetrominoSource::bag(), "bag"),
            (TetrominoSource::stock(NonZeroU32::MIN.saturating_add(1), 0).unwrap(), "bag-2"),
            (TetrominoSource::stock(NonZeroU32::MIN.saturating_add(2), 0).unwrap(), "bag-3"),
            (TetrominoSource::stock(NonZeroU32::MIN.saturating_add(1), 7).unwrap(), "bag-2_restock-on-7"),
            (TetrominoSource::stock(NonZeroU32::MIN.saturating_add(1), 7).unwrap(), "bag-3_restock-on-7"),
            (TetrominoSource::recency_with(0.0), "recency-0.0"),
            (TetrominoSource::recency_with(0.5), "recency-0.5"),
            (TetrominoSource::recency_with(1.0), "recency-1.0"),
            (TetrominoSource::recency_with(1.5), "recency-1.5"),
            (TetrominoSource::recency_with(2.0), "recency-2.0"),
            (TetrominoSource::recency(), "recency"),
            (TetrominoSource::recency_with(3.0), "recency-3.0"),
            (TetrominoSource::recency_with(8.0), "recency-7.0"),
            (TetrominoSource::recency_with(16.0), "recency-16.0"),
            (TetrominoSource::recency_with(32.0), "recency-32.0"),
        ];
        run_analyses_on(sample_count, std::iter::repeat(lookahead).zip(randomizers));
    }

    fn run_analyses_on<'a>(
        sample_count: usize,
        configurations: impl IntoIterator<Item = (usize, (TetrominoSource, &'a str))>,
    ) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let summaries_filename = format!("combot-{timestamp}_SUMMARY.md");
        let mut file = File::options()
            .create(true)
            .append(true)
            .open(summaries_filename)
            .unwrap();
        file.write(
            format!("# Tetrs Combo (4-wide 3-res.) - Bot Statistics Summary\n\n").as_bytes(),
        )
        .unwrap();
        let mut rng = rand::thread_rng();
        for (lookahead, (randomizer, randomizer_name)) in configurations {
            let combos = std::iter::repeat_with(|| {
                run_bot(lookahead, &mut randomizer.clone().with_rng(&mut rng))
            })
            .take(sample_count);
            let filename_svg = format!("combot-{timestamp}_L{lookahead}_{randomizer_name}.svg");
            let summary = run_analysis(combos, lookahead, randomizer_name, &filename_svg);
            file.write(format!("- {summary}\n").as_bytes()).unwrap();
        }
    }

    fn run_bot(lookahead: usize, iter: &mut TetrominoIterator) -> usize {
        let mut next_pieces: VecDeque<_> = iter.take(lookahead).collect();
        let mut state = ComboState {
            layout: (Pat::_200, false),
            active: Some(iter.next().unwrap()),
            hold: None,
            next_pieces: ComboBotHandler::encode_next_queue(next_pieces.iter()),
            depth: 0,
        };
        let mut it: usize = 0;
        loop {
            let states_lvl1 = neighbors(state);
            // No more options to continue.
            let Some(branch) = choose_branch(
                states_lvl1
                    .iter()
                    .map(|(state_lvl1, _)| *state_lvl1)
                    .collect(),
                None,
            ) else {
                break;
            };
            let did_hold = states_lvl1[branch].1.contains(&Button::Hold);
            let mut new_state = states_lvl1[branch].0;
            if new_state.active.is_none() {
                new_state.active = Some(iter.next().unwrap());
            } else if !did_hold || (did_hold && state.hold.is_none()) {
                next_pieces.push_back(iter.next().unwrap());
                next_pieces.pop_front();
            }
            new_state.next_pieces = ComboBotHandler::encode_next_queue(next_pieces.iter());
            state = new_state;
            // Only count if piece was not dropped i.e. used.
            if !did_hold {
                it += 1;
            }
            if it == COMBO_MAX {
                break;
            }
        }
        it
    }

    fn run_analysis(
        combos: impl IntoIterator<Item = usize>,
        lookahead: usize,
        randomizer_name: &str,
        filename_svg: &str,
    ) -> String {
        let mut frequencies = HashMap::<usize, usize>::new();
        let mut sum = 0;
        let mut len = 0;
        for combo in combos {
            *frequencies.entry(combo).or_default() += 1;
            sum += combo;
            len += 1;
        }
        let mut frequencies = frequencies.into_iter().collect::<Vec<_>>();
        frequencies.sort_unstable();
        0;
        let mut tmp = 0;
        let combo_median = 'calc: {
            for (combo, frequency) in frequencies.iter() {
                if tmp > len / 2 {
                    break 'calc combo;
                }
                tmp += frequency;
            }
            unreachable!()
        };
        let combo_max = frequencies.last().unwrap().0;
        let combo_average = sum / len;
        let frequency_max = *frequencies.iter().map(|(_k, v)| v).max().unwrap();
        let summary = format!("samples = {len}, randomizer = '{randomizer_name}', lookahead = {lookahead}; combo_average = {combo_average}, combo_median = {combo_median}, combo_max = {combo_max}, frequency_max = {frequency_max}");

        let font_size = 15;
        let margin_x = 20 * font_size;
        let margin_y = 20 * font_size;
        let gridgranularity_x = 5;
        let gridgranularity_y = 5;
        let chart_max_x = combo_max + (gridgranularity_x - combo_max % gridgranularity_x);
        let chart_max_y = frequency_max + (gridgranularity_y - frequency_max % gridgranularity_y);
        let scale_y = 10;
        let y_0 = margin_y + scale_y * chart_max_y;
        let scale_x = (5).max(scale_y * chart_max_y / chart_max_x);
        let x_0 = margin_x;
        let w_svg = scale_x * chart_max_x + 2 * margin_x;
        let h_svg = scale_y * chart_max_y + 2 * margin_y;

        let file = File::options()
            .create(true)
            .append(true)
            .open(filename_svg)
            .unwrap();
        let mut file = std::io::BufWriter::new(file);

        #[rustfmt::skip] {
        file.write(format!(
r##"<svg
    xmlns="http://www.w3.org/2000/svg"
    width="{w_svg}" height="{h_svg}"
    viewBox="0 0 {w_svg} {h_svg}"
>

"##).as_bytes()).unwrap();

    file.write(format!(
r##"<!-- Background. -->
<rect width="100%" height="100%" fill="#3f3f3f" />
"##).as_bytes()).unwrap();

    file.write(format!(
r##"<!-- Grid lines. -->
<g stroke="#FFFFFF" stroke-opacity=".25" stroke-width="2" stroke-linecap="square">
"##).as_bytes()).unwrap();

    file.write(format!(
r##"    <!-- Horizontal grid lines. -->
    <g>
"##).as_bytes()).unwrap();

    for i in 0 ..= chart_max_y/gridgranularity_y {
        let y = y_0 - scale_y *(i * gridgranularity_y);
        file.write(format!(
r##"        <line x1="{}" y1="{}"  x2="{}" y2="{}" ></line>
"##, x_0, y, x_0 + scale_x *chart_max_x, y).as_bytes()).unwrap();
    }

    file.write(format!(
r##"    </g>
"##).as_bytes()).unwrap(); // <!-- Horizontal grid lines. -->

    file.write(format!(
r##"    <!-- Vertical grid lines. -->
    <g>
"##).as_bytes()).unwrap();

    for j in 0 ..= chart_max_x/gridgranularity_x {
        let x = x_0 + scale_x *(j * gridgranularity_x);
        file.write(format!(
r##"        <line x1="{}" y1="{}"  x2="{}" y2="{}" ></line>
"##, x, y_0, x, y_0 - scale_y *chart_max_y).as_bytes()).unwrap();
    }

    // Combo average indicator.
    file.write(format!(
r##"        <line x1="{}" y1="{}"  x2="{}" y2="{}" stroke="#00FFFF" ></line>
"##, x_0 + scale_x *combo_average, y_0, x_0 + scale_x *combo_average, y_0 - scale_y *chart_max_y).as_bytes()).unwrap();

    // Combo median indicator.
    file.write(format!(
r##"        <line x1="{}" y1="{}"  x2="{}" y2="{}" stroke="#FF7F00" ></line>
"##, x_0 + scale_x *combo_median, y_0, x_0 + scale_x *combo_median, y_0 - scale_y *chart_max_y).as_bytes()).unwrap();

    file.write(format!(
r##"    </g>
"##).as_bytes()).unwrap(); // <!-- Vertical grid lines. -->

    file.write(format!(
r##"</g>
"##).as_bytes()).unwrap(); // <!-- Grid lines. -->

    file.write(format!(
r##"<!-- Labels. -->
<g fill="#FFFFFF" font-size="{}px" font-family="monospace">
"##, font_size).as_bytes()).unwrap();

    file.write(format!(
r##"        <text x="{}" y="{}" font-size="{}px" font-weight="bold" text-anchor="start" fill="#00FFFF" >Tetrs Combo (4-wide 3-res.) - Bot run statistics.</text>
"##, x_0, y_0 + font_size * 3 + font_size / 2, font_size * 5 / 4).as_bytes()).unwrap();

file.write(format!(
r##"        <text x="{}" y="{}" text-anchor="start">{summary}.</text>
"##, x_0, y_0 + font_size * 5).as_bytes()).unwrap();

    file.write(format!(
r##"    <!-- y-axis labels. -->
    <g text-anchor="end">
"##).as_bytes()).unwrap();

    for i in 0 ..= chart_max_y/gridgranularity_y {
        let y = y_0 - scale_y *(i * gridgranularity_y) + font_size / 2;
        file.write(format!(
r##"        <text x="{}" y="{}">{}</text>
"##, x_0 - font_size / 2, y, i*gridgranularity_y).as_bytes()).unwrap();
    }

    file.write(format!(
r##"        <text x="{}" y="{}" text-anchor="middle">Frequency</text>
"##, x_0, margin_y - font_size).as_bytes()).unwrap();

    file.write(format!(
r##"        <text x="{}" y="{}" fill="#00FFFF" text-anchor="middle">Average</text>
"##, x_0 + scale_x* combo_average, margin_y - font_size).as_bytes()).unwrap();

    file.write(format!(
r##"        <text x="{}" y="{}" fill="#FF7F00" text-anchor="middle">Median</text>
"##, x_0 + scale_x* combo_median, margin_y - font_size).as_bytes()).unwrap();

    file.write(format!(
r##"    </g>
"##).as_bytes()).unwrap(); // <!-- y-axis labels. -->

    file.write(format!(
r##"    <!-- x-axis labels. -->
    <g text-anchor="middle">
"##).as_bytes()).unwrap();

    for i in 0 ..= chart_max_x/gridgranularity_x {
        let x = x_0 + scale_x *(i * gridgranularity_x);
        file.write(format!(
r##"        <text transform="translate({},{}) rotate(45)">{}</text>
"##, x - font_size / 2, y_0 + font_size * 3 / 2, i*gridgranularity_x).as_bytes()).unwrap();
    }

    file.write(format!(
r##"        <text x="{}" y="{}" text-anchor="start">Combo Length</text>
"##, x_0 + scale_x* chart_max_x + font_size, y_0 + font_size / 2).as_bytes()).unwrap();

    file.write(format!(
r##"    </g>
"##).as_bytes()).unwrap(); // <!-- x-axis labels. -->*/

    file.write(format!(
r##"</g>
"##).as_bytes()).unwrap(); // <!-- Labels. -->

    file.write(format!(
r##"<!-- Surface graph path. -->
<path
    stroke="#FFFFFF"
    stroke-width="1"
    fill="#FFFFFF"
    fill-opacity=".5"
    d="
        M {x_0},{y_0}
"##).as_bytes()).unwrap();

    for (combo, frequency) in frequencies.iter() {
        file.write(format!(
r##"        L{},{}
"##, x_0 + scale_x* combo, y_0 - scale_y *frequency).as_bytes()).unwrap();
    }

    file.write(format!(
r##"        L{},{}
        M {x_0},{y_0}
    "
/>"##, x_0 + scale_x *combo_max, y_0).as_bytes()).unwrap(); // <!-- Surface graph path. -->

    file.write(format!(
r##"<!-- Graph data points. -->
<g fill="#00FFFF">
"##).as_bytes()).unwrap();

        for (combo, frequency) in frequencies.iter() {
            file.write(format!(
r##"    <circle cx="{}" cy="{}"  r="{}" />
"##, x_0 + scale_x *combo, y_0 - scale_y *frequency, font_size / 5).as_bytes()).unwrap();
        }

        file.write(format!(
r##"</g>
"##).as_bytes()).unwrap(); // <!-- Graph data points.. -->

    file.write(format!(
r##"</svg>
"##).as_bytes()).unwrap();
    };

        summary
    }
}
