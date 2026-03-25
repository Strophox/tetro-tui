use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};
use falling_tetromino_engine::Button;

use crate::application::SlotMachine;

#[derive(PartialEq, Eq, Clone, Debug)]
#[serde_with::serde_as] // Do **NOT** place this after #[derive(..)] !!
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Keybinds {
    // Note: the alternative has ugly double-escaped quotation marks: #[serde_as(as = "std::collections::HashMap<serde_with::json::JsonString, _>")]
    #[serde_as(as = "Vec<(_, _)>")]
    mapping: HashMap<(KeyCode, KeyModifiers), Button>,
}

pub fn default_keybinds_slots() -> SlotMachine<Keybinds> {
    let slots = vec![
        ("Default".to_owned(), Keybinds::default_tetro()),
        ("Control+".to_owned(), Keybinds::extra_control()),
        ("Guideline".to_owned(), Keybinds::guideline()),
        ("Vim".to_owned(), Keybinds::vim()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Keybinds".to_owned())
}

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

impl Keybinds {
    pub fn get(&self, (code, modifiers): (KeyCode, KeyModifiers)) -> Option<&Button> {
        self.mapping.get(&normalize((code, modifiers)))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&(KeyCode, KeyModifiers), &Button)> {
        self.mapping.iter()
    }

    /// This provides unstable but direct access to the internal representation for special purposes.
    pub fn unstable_access(&mut self) -> &mut HashMap<(KeyCode, KeyModifiers), Button> {
        &mut self.mapping
    }

    pub fn empty() -> Keybinds {
        Keybinds {
            mapping: Default::default(),
        }
    }

    pub fn default_tetro() -> Keybinds {
        let keys = [
            (KeyCode::Left, Button::MoveLeft),
            (KeyCode::Right, Button::MoveRight),
            (KeyCode::Char('a'), Button::RotateLeft),
            (KeyCode::Char('d'), Button::RotateRight),
            //(KeyCode::Char('s'), Button::Rotate180),
            (KeyCode::Down, Button::DropSoft),
            (KeyCode::Up, Button::DropHard),
            //(KeyCode::Char('w'), Button::TeleDown),
            //(KeyCode::Char('q'), Button::TeleLeft),
            //(KeyCode::Char('e'), Button::TeleRight),
            (KeyCode::Char(' '), Button::HoldPiece),
        ]
        .map(|(k, b)| ((k, KeyModifiers::NONE), b));

        Keybinds {
            mapping: keys.into(),
        }
    }

    pub fn extra_control() -> Keybinds {
        let keys = [
            (KeyCode::Left, Button::MoveLeft),
            (KeyCode::Right, Button::MoveRight),
            (KeyCode::Char('a'), Button::RotateLeft),
            (KeyCode::Char('d'), Button::RotateRight),
            (KeyCode::Char('s'), Button::Rotate180),
            (KeyCode::Down, Button::DropSoft),
            (KeyCode::Up, Button::DropHard),
            (KeyCode::Char('w'), Button::TeleDown),
            (KeyCode::Char('q'), Button::TeleLeft),
            (KeyCode::Char('e'), Button::TeleRight),
            (KeyCode::Char(' '), Button::HoldPiece),
        ]
        .map(|(k, b)| ((k, KeyModifiers::NONE), b));

        Keybinds {
            mapping: keys.into(),
        }
    }

    pub fn guideline() -> Keybinds {
        use crossterm::event::ModifierKeyCode as M;
        let keys = [
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

        Keybinds {
            mapping: keys.into(),
        }
    }

    pub fn vim() -> Keybinds {
        let keys = [
            (KeyCode::Char('h'), Button::MoveLeft),
            (KeyCode::Char('l'), Button::MoveRight),
            (KeyCode::Char('a'), Button::RotateLeft),
            (KeyCode::Char('d'), Button::RotateRight),
            (KeyCode::Char('j'), Button::DropSoft),
            (KeyCode::Char('k'), Button::DropHard),
            (KeyCode::Char(' '), Button::HoldPiece),
        ]
        .map(|(k, b)| ((k, KeyModifiers::NONE), b));

        Keybinds {
            mapping: keys.into(),
        }
    }
}
