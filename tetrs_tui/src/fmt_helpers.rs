use std::time::Duration;

use crossterm::event::{KeyCode, KeyModifiers};
use tetrs_engine::{Button, ButtonChange, ExtNonNegF64, Tetromino};

use crate::keybinds_presets::Keybinds;

pub fn fmt_duration(dur: Duration) -> String {
    format!(
        "{}min {}.{:02}s",
        dur.as_secs() / 60,
        dur.as_secs() % 60,
        dur.as_millis() % 1000 / 10
    )
}

pub fn fmt_hertz(f: ExtNonNegF64) -> String {
    const LOWERBOUND: f64 = 0.1e-6;
    const UPPERBOUND: f64 = 0.1e+6;
    if f.get() <= LOWERBOUND {
        "0 Hz".to_owned()
    } else if UPPERBOUND <= f.get() {
        "∞ Hz".to_owned()
    } else {
        format!("{:.01} Hz", f.get())
    }
}

pub fn fmt_tet_small(t: Tetromino) -> &'static str {
    use Tetromino::*;
    match t {
        O => "██",
        I => "▄▄▄▄",
        S => "▄█▀",
        Z => "▀█▄",
        T => "▄█▄",
        L => "▄▄█",
        J => "█▄▄",
    }
}

pub fn fmt_tet_mini(t: Tetromino) -> &'static str {
    use Tetromino::*;
    match t {
        O => "⠶", //"⠶",
        I => "⡇", //"⠤⠤",
        S => "⠳", //"⠴⠂",
        Z => "⠞", //"⠲⠄",
        T => "⠗", //"⠴⠄",
        L => "⠧", //"⠤⠆",
        J => "⠼", //"⠦⠄",
    }
}

pub fn fmt_tetromino_counts(counts: &[u32; Tetromino::VARIANTS.len()]) -> String {
    counts
        .iter()
        .zip(Tetromino::VARIANTS)
        .map(|(n, t)| format!("{n}{}", fmt_tet_mini(t)))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn fmt_button(b: Button) -> &'static str {
    use Button as B;
    match b {
        B::MoveLeft => "←",
        B::MoveRight => "→",
        B::RotateLeft => "↺",
        B::RotateRight => "↻",
        B::RotateAround => "↔",
        B::DropSoft => "↓",
        B::DropHard => "⤓",
        B::TeleDown => "⇓",
        B::TeleLeft => "⇐",
        B::TeleRight => "⇒",
        B::HoldPiece => "h",
    }
}

pub fn fmt_button_change(button_change: ButtonChange) -> String {
    match button_change {
        ButtonChange::Press(b) => format!("++[{}]", fmt_button(b)),
        ButtonChange::Release(b) => format!("--[{}]", fmt_button(b)),
    }
}

#[allow(dead_code)]
pub fn fmt_button_state(button_state: &[bool; Button::VARIANTS.len()]) -> String {
    let s = button_state
        .iter()
        .zip(Button::VARIANTS)
        .filter(|(p, _)| **p)
        .map(|(_, b)| fmt_button(b))
        .collect::<Vec<_>>()
        .join(" ");

    format!("[{s}]")
}

pub fn fmt_key(key: KeyCode) -> String {
    use crossterm::event::ModifierKeyCode as M;
    use KeyCode as K;
    match key {
        K::Backspace => "Back",
        //K::Enter => "Enter",
        K::Left => "←",
        K::Right => "→",
        K::Up => "↑",
        K::Down => "↓",
        //K::Home => "Home",
        //K::End => "End",
        //K::Insert => "Insert",
        K::Delete => "Del",
        //K::Menu => "Menu",
        K::PageUp => "PgUp",
        K::PageDown => "PgDn",
        //K::Tab => "Tab",
        //K::CapsLock => "CapsLock",
        K::F(k) => return format!("F{k}"),
        K::Char(' ') => "Space",
        K::Char(c) => {
            return c /*FIXME: Remove?: .to_uppercase()*/
                .to_string();
        }
        //K::Esc => "Esc",
        K::Modifier(M::LeftAlt) => "LAlt",
        K::Modifier(M::RightAlt) => "RAlt",
        K::Modifier(M::LeftShift) => "LShift",
        K::Modifier(M::RightShift) => "RShift",
        K::Modifier(M::LeftControl) => "LCtrl",
        K::Modifier(M::RightControl) => "RCtrl",
        K::Modifier(M::IsoLevel3Shift) => "AltGr",
        K::Modifier(M::IsoLevel5Shift) => "Iso5",
        K::Modifier(M::LeftSuper) => "LSuper",
        K::Modifier(M::RightSuper) => "RSuper",
        K::Modifier(M::LeftHyper) => "LHyper",
        K::Modifier(M::RightHyper) => "RHyper",
        K::Modifier(M::LeftMeta) => "LMeta",
        K::Modifier(M::RightMeta) => "RMeta",
        k => return format!("{:?}", k),
    }
    .to_string()
}

pub fn fmt_keymods(keymod: KeyModifiers) -> String {
    use KeyModifiers as KMs;
    [
        keymod.contains(KMs::CONTROL).then_some("Ctrl"),
        keymod.contains(KMs::SHIFT).then_some("Shift"),
        keymod.contains(KMs::ALT).then_some("Alt"),
        keymod.contains(KMs::SUPER).then_some("Super"),
        keymod.contains(KMs::HYPER).then_some("Hyper"),
        keymod.contains(KMs::META).then_some("Meta"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("+")
}

pub fn fmt_keybinds_of(button: Button, keybinds: &Keybinds) -> String {
    keybinds
        .iter()
        .filter_map(|(&(k, kms), &b)| {
            (b == button).then_some(if kms.is_empty() {
                format!("[{}]", fmt_key(k))
            } else {
                format!("[{}+{}]", fmt_keymods(kms), fmt_key(k))
            })
        })
        .collect::<Vec<_>>()
        .join("")
}
