use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};
use falling_tetromino_engine::Button;

pub type Keybinds = HashMap<(KeyCode, KeyModifiers), Button>;

pub fn normalize((mut code, mut modifiers): (KeyCode, KeyModifiers)) -> (KeyCode, KeyModifiers) {
    match code {
        KeyCode::Modifier(modifier_key_code) => {
            // If a *modifier-as-keycode* is being handled, remove 'unnecessary'/duplicate modifier flag.
            // (It's just duplicate information that might unintuitively influence keybind detection.)
            use crossterm::event::ModifierKeyCode as MKC;
            let modifier = match modifier_key_code {
                MKC::LeftShift | MKC::RightShift => KeyModifiers::SHIFT,
                MKC::LeftControl | MKC::RightControl => KeyModifiers::CONTROL,
                MKC::LeftAlt | MKC::RightAlt => KeyModifiers::ALT,
                MKC::LeftSuper | MKC::RightSuper => KeyModifiers::SUPER,
                MKC::LeftHyper | MKC::RightHyper => KeyModifiers::HYPER,
                MKC::LeftMeta | MKC::RightMeta => KeyModifiers::META,
                MKC::IsoLevel3Shift | MKC::IsoLevel5Shift => KeyModifiers::NONE,
            };

            modifiers.remove(modifier);
        }

        // Normalize character enum to store a lowercase `char`.
        // FIXME: Could this somehow have undesirable effects?
        KeyCode::Char(ref mut char) => {
            *char = char.to_ascii_lowercase();
        }

        // No changes for other keycodes.
        _ => {}
    }

    (code, modifiers)
}

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

pub fn tetrs_finesse_keybinds() -> Keybinds {
    let keybinds_tetrs: [((KeyCode, KeyModifiers), Button); 11] = [
        (KeyCode::Left, Button::MoveLeft),
        (KeyCode::Right, Button::MoveRight),
        (KeyCode::Char('a'), Button::RotateLeft),
        (KeyCode::Char('d'), Button::RotateRight),
        (KeyCode::Char('s'), Button::RotateAround),
        (KeyCode::Down, Button::DropSoft),
        (KeyCode::Up, Button::DropHard),
        (KeyCode::Char('w'), Button::TeleDown),
        (KeyCode::Char('q'), Button::TeleLeft),
        (KeyCode::Char('e'), Button::TeleRight),
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
