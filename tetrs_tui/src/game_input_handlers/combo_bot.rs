use std::{
    collections::{HashSet, VecDeque}, fmt::Debug, fs::File, io::Write, sync::mpsc::{self, Receiver, RecvError, Sender}, thread::{self, JoinHandle}, time::{Duration, Instant}, vec
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
        // TODO: Document only being able to look 128 // 3 == 42 ahead.
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
            next_pieces: Self::encode_next_queue(game.state().next_pieces.iter().take(MAX_LOOKAHEAD)),
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
                        /*TODO: Remove debug: let s=format!("[ main1 REVOYYY zeroth_state = {state_lvl0:?} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
                        let (states_lvl1, states_lvl1_buttons): (
                            Vec<ComboState>,
                            Vec<ButtonInstructions>,
                        ) = neighbors(state_lvl0).into_iter().unzip();
                        /*TODO: Remove debug: let s=format!("[ main2 states_lvl1 = {:?} = {states_lvl1:?} ]\n", states_lvl1.iter().map(|state| fmt_statenode(&(0, *state))).collect::<Vec<_>>());let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
                        // No more options to continue.
                        let Some(branch_choice) = choose_branch(states_lvl1, GRAPHVIZ.then_some(state_lvl0)) else {
                            /*TODO: Remove debug: let s=format!("[ main3 uhhhhhh ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
                            let _ = button_sender.send(Err(crate::game_input_handlers::Interrupt::Pause));
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
                                    Button::RotateAround | Button::DropSoft | Button::DropHard | Button::DropSonic | Button::Hold => button,
                                };
                            }
                            let _ = button_sender.send(Ok((Instant::now(), button, true)));
                            let _ = button_sender.send(Ok((Instant::now(), button, false)));
                            /*TODO: Remove debug: let s=format!("[ main4 SENT button = {button:?} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
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

fn choose_branch(states_lvl1: Vec<ComboState>, debug_state_lvl0: Option<ComboState>) -> Option<usize> {
    /*TODO: Remove debug: let s=format!("[ chbr1 examine states = {states_lvl1:?} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
    if states_lvl1.is_empty() {
        /*TODO: Remove debug: let s=format!("[ chbr2 empty ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        None
    // One option to continue, do not do further analysis.
    } else if states_lvl1.len() == 1 {
        /*TODO: Remove debug: let s=format!("[ chbr single ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        Some(0)
    // Several options to evaluate, do graph algorithm.
    } else {
        /*TODO: Remove debug: let s=format!("[ chbr multianalyze ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        let num_states = states_lvl1.len();
        let mut queue: VecDeque<(usize, ComboState)> = states_lvl1.into_iter().enumerate().collect();
        let mut graphviz_str = String::new();
        if let Some(state_lvl0) = debug_state_lvl0 {
            graphviz_str.push_str("strict digraph {\n");
            graphviz_str.push_str(&format!("\"{}\"\n", fmt_statenode(&(0, state_lvl0))));
            for statenode in queue.iter() {
               graphviz_str.push_str(&format!("\"{}\" -> \"{}\"\n", fmt_statenode(&(0, state_lvl0)), fmt_statenode(statenode)));
            }
        }
        let mut depth_best = queue.iter().map(|(_, state)| state.depth).max().unwrap();
        let mut states_best = queue.iter().filter(|(_, state)| state.depth == depth_best).copied().collect::<Vec<_>>();
        /*TODO: Remove debug: let s=format!("[ chbr before-while ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        while let Some(statenode @ (branch, state)) = queue.pop_front() {
            let neighbors: Vec<_> = neighbors(state)
                .into_iter()
                .map(|(state, _)| (branch, state))
                .collect();
            if debug_state_lvl0.is_some() {
                for state in neighbors.iter() {
                    graphviz_str.push_str(&format!("\"{}\" -> \"{}\"\n", fmt_statenode(&statenode), fmt_statenode(state)));
                }
            }
            for neighbor in neighbors.iter() {
                let depth = neighbor.1.depth;
                if depth > depth_best{
                    depth_best = depth;
                    states_best.clear();
                    states_best.push(*neighbor);
                } else if depth == depth_best {
                    states_best.push(*neighbor);
                }
            }
            queue.extend(neighbors);
        }
        /*TODO: Remove debug: let s=format!("[ chbr depth_best = {depth_best} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        if debug_state_lvl0.is_some() {
            graphviz_str.push_str("\n}");

            File::options()
            .create(true)
            .append(true)
            .open(GRAPHVIZ_FILENAME)
            .unwrap()
            .write(format!("graphviz: \"\"\"\n{graphviz_str}\n\"\"\"\n").as_bytes())
            .unwrap();
            /*TODO: Remove debug: let s=format!("[ chbr graphviz_str = \"\"\"{graphviz_str}\"\"\" ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        }
        /*TODO: Remove debug: let s=format!("[ chbr states_best = {states_best:?} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
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
                /*TODO: Remove debug: let s=format!("[ chbr branch = {branch}, val = {val}\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
                val
            })
            .unwrap();
        /*TODO: Remove debug: let s=format!("[ chbr best = {best:?} ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
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
    /*TODO: Remove debug: let s=format!("[ nbrs1 entered ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
    let mut neighbors = Vec::new();
    let Some(active) = active else {
        /*TODO: Remove debug: let s=format!("[ nbrs2 early-ret ]\n");let _=std::io::Write::write(&mut std::fs::OpenOptions::new().append(true).open("tetrs_tui_error_message_COMBO.txt").unwrap(), s.as_bytes());*/
        return neighbors;
    };
    let new_active = (next_pieces != 0).then(|| Tetromino::SHAPES[usize::try_from(next_pieces & 0b111).unwrap() - 1]);
    let new_next_pieces = next_pieces >> 3;
    // Add neighbors reachable with just holding / swapping with the active piece.
    if let Some((held, swap_allowed)) = hold {
        if swap_allowed {
            neighbors.push(
                (
                    ComboState {
                        layout,
                        active: Some(held),
                        hold: Some((active, false)),
                        next_pieces,
                        depth
                    },
                    &[Button::Hold][..],
                )
            );
        }
    } else {
        neighbors.push(
            (
                ComboState {
                    layout,
                    active: new_active,
                    hold: Some((active, false)),
                    next_pieces,
                    depth
                },
                &[Button::Hold][..],
            )
        );
    }
    neighbors.extend(reachable_with(layout, active)
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
        })
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
    use std::num::NonZeroU32;

    use super::*;
    use tetrs_engine::piece_generation::TetrominoSource;

    #[test]
    fn run_mini_bench() {
        const N_RUNS: usize = 10_000;
        const N_PREVIEW: usize = 6;
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let filename = format!("combot_lookahead-{N_PREVIEW}_{timestamp}.md");
        let mut file = File::options()
            .create(true)
            .append(true)
            .open(filename)
            .unwrap();
        file.write(
            format!("# Benchmark ({timestamp})\nN_RUNS = {N_RUNS}, N_PREVIEW = {N_PREVIEW}\n")
                .as_bytes(),
        )
        .unwrap();
        let generators = [
            TetrominoSource::recency(),
            TetrominoSource::bag(NonZeroU32::MIN),
            TetrominoSource::total_relative(),
            TetrominoSource::uniform(),
        ];
        let mut rng = rand::thread_rng();
        for generator in generators {
            let mut runs = std::iter::repeat_with(|| {
                let mut source = generator.clone();
                let mut next_pieces: VecDeque<_> = source.with_rng(&mut rng).take(N_PREVIEW).collect();
                let mut state = ComboState {
                    layout: (Pat::_200, false),
                    active: Some(source.with_rng(&mut rng).next().unwrap()),
                    hold: None,
                    next_pieces: ComboBotHandler::encode_next_queue(next_pieces.iter()),
                    depth: 0,
                };
                let mut it: u32 = 0;
                loop {
                    let states_lvl1 = neighbors(state);
                    // No more options to continue.
                    let Some(branch) = choose_branch(states_lvl1.iter()
                        .map(|(state_lvl1, _)| *state_lvl1)
                        .collect(), None)
                    else {
                        break;
                    };
                    let did_hold = states_lvl1[branch].1.contains(&Button::Hold);
                    let mut new_state = states_lvl1[branch].0;
                    if new_state.active.is_none() {
                        new_state.active = Some(source.with_rng(&mut rng).next().unwrap());
                    } else if !did_hold || (did_hold && state.hold.is_none()) {
                        next_pieces.push_back(source.with_rng(&mut rng).next().unwrap());
                        next_pieces.pop_front();
                    }
                    new_state.next_pieces = ComboBotHandler::encode_next_queue(next_pieces.iter());
                    state = new_state;
                    // Only count if piece was not dropped i.e. used.
                    if !did_hold {
                        it += 1;
                    }
                }
                it
            })
            .take(N_RUNS)
            .collect::<Vec<_>>();
            runs.sort_unstable();
            let min = runs.iter().min().unwrap();
            let max = runs.iter().max().unwrap();
            let median = if runs.len() % 2 == 0 {
                (runs[runs.len() / 2 - 1] + runs[runs.len() / 2]) as f64 / 2.0
            } else {
                runs[runs.len() / 2] as f64
            };
            let mut counts = std::collections::HashMap::new();
            let mode = runs
                .iter()
                .max_by_key(|n| {
                    let count = counts.entry(*n).or_insert(0);
                    *count += 1;
                    *count
                })
                .unwrap();
            let average = f64::from(runs.iter().sum::<u32>()) / N_RUNS as f64;
            let results = format!("* min {min}, median {median:.01}, max {max}, mode {mode}, average {average} :: {generator:?}\n");
            file.write(results.as_bytes()).unwrap();
        }
    }
}
