use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};
use tetrs_engine::Button;

pub type Keybinds = HashMap<(KeyCode, KeyModifiers), Button>;

pub fn tetrs_default_keybinds() -> Keybinds {
    let keybinds_tetrs: [((KeyCode, KeyModifiers), Button); 7] = [
        (KeyCode::Left, Button::MoveLeft),
        (KeyCode::Right, Button::MoveRight),
        (KeyCode::Char('a'), Button::RotateLeft),
        (KeyCode::Char('d'), Button::RotateRight),
        //(KeyCode::Char('s'), Button::RotateAround),
        (KeyCode::Down, Button::DropSoft),
        (KeyCode::Up, Button::DropHard),
        //(KeyCode::Char('w'), Button::TeleDown),
        //(KeyCode::Char('q'), Button::TeleLeft),
        //(KeyCode::Char('e'), Button::TeleRight),
        (KeyCode::Char(' '), Button::HoldPiece),
    ]
    .map(|(k, b)| ((k, KeyModifiers::NONE), b));
    HashMap::from(keybinds_tetrs)
}

pub fn vim_keybinds() -> Keybinds {
    let keybinds_vim: [((KeyCode, KeyModifiers), Button); 7] = [
        (KeyCode::Char('h'), Button::MoveLeft),
        (KeyCode::Char('l'), Button::MoveRight),
        (KeyCode::Char('a'), Button::RotateLeft),
        (KeyCode::Char('d'), Button::RotateRight),
        (KeyCode::Char('j'), Button::DropSoft),
        (KeyCode::Char('k'), Button::DropHard),
        (KeyCode::Char(' '), Button::HoldPiece),
    ]
    .map(|(k, b)| ((k, KeyModifiers::NONE), b));
    HashMap::from(keybinds_vim)
}

pub fn guideline_keybinds() -> Keybinds {
    use crossterm::event::ModifierKeyCode as M;
    let keybinds_guidelinle: [((KeyCode, KeyModifiers), Button); 13] = [
        (KeyCode::Left, Button::MoveLeft),
        (KeyCode::Right, Button::MoveRight),
        (KeyCode::Char('z'), Button::RotateLeft),
        (KeyCode::Char('y'), Button::RotateLeft), // 'Branch-predicting' European keyboards.
        (KeyCode::Modifier(M::LeftControl), Button::RotateLeft),
        (KeyCode::Modifier(M::RightControl), Button::RotateLeft),
        (KeyCode::Char('x'), Button::RotateRight),
        (KeyCode::Up, Button::RotateRight),
        (KeyCode::Down, Button::DropSoft),
        (KeyCode::Char(' '), Button::DropHard),
        (KeyCode::Char('c'), Button::HoldPiece),
        (KeyCode::Modifier(M::LeftShift), Button::HoldPiece),
        (KeyCode::Modifier(M::RightShift), Button::HoldPiece),
    ]
    .map(|(k, b)| ((k, KeyModifiers::NONE), b));
    HashMap::from(keybinds_guidelinle)
}
