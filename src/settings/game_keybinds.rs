use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};
use falling_tetromino_engine::Button;

use crate::settings::SlotMachine;

#[derive(PartialEq, Eq, Clone, Debug)]
#[serde_with::serde_as] // Do **NOT** place this after #[derive(..)] !!
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct GameKeybinds {
    // Note: the alternative has ugly doubly-escaped quotation marks: #[serde_as(as = "std::collections::HashMap<serde_with::json::JsonString, _>")]
    // #[serde_as(as = "Vec<(_, _)>")]
    keymap: HashMap<(KeyCode, KeyModifiers), Button>,
}

pub fn game_keybinds_presets() -> SlotMachine<GameKeybinds> {
    let slots = vec![
        ("Default".to_owned(), GameKeybinds::default_tetro()),
        ("Guideline".to_owned(), GameKeybinds::guideline()),
        ("Control+".to_owned(), GameKeybinds::extra_control()),
        ("Terminal finesse".to_owned(), GameKeybinds::terminal_fin()),
        ("Vim".to_owned(), GameKeybinds::vim()),
        // ("Blank slate".to_owned(), GameKeybinds::blank_slate()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Keybinds".to_owned())
}

impl GameKeybinds {
    pub fn get(&self, (code, modifiers): (KeyCode, KeyModifiers)) -> Option<&Button> {
        self.keymap.get(&Self::normalize((code, modifiers)))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&(KeyCode, KeyModifiers), &Button)> {
        self.keymap.iter()
    }

    /// This provides unstable but direct access to the internal representation for special purposes.
    pub fn unstable_access(&mut self) -> &mut HashMap<(KeyCode, KeyModifiers), Button> {
        &mut self.keymap
    }

    pub fn normalize(
        (mut code, mut modifiers): (KeyCode, KeyModifiers),
    ) -> (KeyCode, KeyModifiers) {
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
            // FIXME: Undesirable effects possible from this?
            KeyCode::Char(ref mut char) => {
                *char = char.to_ascii_lowercase();
            }

            // No changes for other keycodes.
            _ => {}
        }

        (code, modifiers)
    }

    pub fn default_tetro() -> GameKeybinds {
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

        GameKeybinds {
            keymap: keys.into(),
        }
    }

    pub fn extra_control() -> GameKeybinds {
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

        GameKeybinds {
            keymap: keys.into(),
        }
    }

    pub fn guideline() -> GameKeybinds {
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

        GameKeybinds {
            keymap: keys.into(),
        }
    }

    pub fn vim() -> GameKeybinds {
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

        GameKeybinds {
            keymap: keys.into(),
        }
    }

    /// This layout is an attempt at optimized finesse for terminal use.
    /// It makes use of teleport left/right instead of 0 ARR for compatibility,
    /// and distributes keys for efficient finesse.
    ///
    /// Reasoning:
    /// - `MIN-FINGER-MVMT` Maximize home row usage and generally minimize finger movement.
    /// - `COMMON-ON-RESTING-POS` Use homerow for most common actions [R180 TR ML RL - DH RR MR TL].
    /// - `MOST-COMMON-ON-INDEX` Use right index for hard drop.
    /// - `ROLL-FINGERS` Arrange in a way where fingers can be rolled on keyboard to execute finesse, e.g. TR->ML->RL->HD is one handstroke.
    /// - `TELE-COMPAT-WITH-ANY` Use spacebar which is accessible from both hands indepently of other fingers to enable rare but possible TD (sonic drop) which might want to be used with any other common actions.
    /// - `HOLD-TRADEOFF-WITH-HARDDROP` Use same finger for hard drop for hold, because most often a player will want to *either* hard drop a piece, *or* hold it.
    /// - `SOFTDROP-TRADEOFF-WITH-HARDDROP` Use same finger for hard drop for soft drop, because most often a player will want to *either* hard drop a piece, *or* soft drop it and apply further actions (tuck) before once again soft(/hard) dropping it to lock it.
    ///
    pub fn terminal_fin() -> GameKeybinds {
        let keys = [
            (KeyCode::Char('a'), Button::Rotate180),
            (KeyCode::Char('s'), Button::TeleRight),
            (KeyCode::Char('d'), Button::MoveLeft),
            (KeyCode::Char('f'), Button::RotateLeft),
            (KeyCode::Char(' '), Button::TeleDown),
            (KeyCode::Char('n'), Button::DropSoft),
            (KeyCode::Char('h'), Button::HoldPiece),
            (KeyCode::Char('j'), Button::DropHard),
            (KeyCode::Char('k'), Button::RotateRight),
            (KeyCode::Char('l'), Button::MoveRight),
            (KeyCode::Char(';'), Button::TeleLeft), // US.
            (KeyCode::Char('ö'), Button::TeleLeft), // German.
            (KeyCode::Char('ø'), Button::TeleLeft), // Nordic.
            (KeyCode::Char('ç'), Button::TeleLeft), // Italian and Portuguese.
            (KeyCode::Char('ñ'), Button::TeleLeft), // Spanish.
                                                    // (KeyCode::Char('æ'), Button::TeleLeft), // Icelandic?
                                                    // (KeyCode::Char('é'), Button::TeleLeft), // Hungarian?
                                                    // (KeyCode::Char('ů'), Button::TeleLeft), // Czech?
                                                    // (KeyCode::Char('ł'), Button::TeleLeft), // Polish?
                                                    // (KeyCode::Char('ș'), Button::TeleLeft), // Romanian?
                                                    // (KeyCode::Char('+'), Button::TeleLeft), // Danish?
        ]
        .map(|(k, b)| ((k, KeyModifiers::NONE), b));

        GameKeybinds {
            keymap: keys.into(),
        }
    }

    // pub fn blank_slate() -> GameKeybinds {
    //     GameKeybinds { keymap: [].into() }
    // }
}
