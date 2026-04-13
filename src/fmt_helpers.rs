use std::time::Duration;

use crossterm::event::{KeyCode, KeyModifiers};
use falling_tetromino_engine::{Button, ExtNonNegF64, Input, Tetromino};

use crate::settings::GameKeybinds;

pub type KeybindsLegend = Vec<(/*(KeyCode, KeyModifiers)*/ String, &'static str)>;

pub trait BoolAsOnOff {
    fn on_off(self) -> &'static str;
}

impl BoolAsOnOff for bool {
    fn on_off(self) -> &'static str {
        if self {
            "on"
        } else {
            "off"
        }
    }
}

pub fn fmt_duration(dur: Duration) -> String {
    if dur.as_secs() / 60 == 0 {
        format!("{}.{:02}s", dur.as_secs() % 60, dur.as_millis() % 1000 / 10)
    } else {
        format!(
            "{}min {}.{:02}s",
            dur.as_secs() / 60,
            dur.as_secs() % 60,
            dur.as_millis() % 1000 / 10
        )
    }
}

pub fn fmt_hertz(f: ExtNonNegF64) -> String {
    const LOWERBOUND: f64 = 1. / 100.0;
    const UPPERBOUND: f64 = 10000.0; // "20 tiles per (60 Hz) frame."
    if f.is_zero() {
        "0 Hz".to_owned()
    } else if f.get() <= LOWERBOUND {
        "~0 Hz".to_owned()
    } else if f.is_infinite() {
        "∞ Hz".to_owned()
    } else if UPPERBOUND <= f.get() {
        "~∞ Hz".to_owned()
    } else {
        format!("{:.02} Hz", f.get())
    }
}

pub fn fmt_tetromino_counts(
    counts: &[u32; Tetromino::VARIANTS.len()],
    mini_tet_glyphs: &[char; Tetromino::VARIANTS.len()],
) -> String {
    counts
        .iter()
        .zip(Tetromino::VARIANTS)
        .map(|(n, t)| format!("{n}{}", mini_tet_glyphs[t as usize].to_ascii_lowercase()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn fmt_player_input(input: Input, button_glyphs: [char; Button::VARIANTS.len()]) -> String {
    match input {
        Input::Activate(b) => format!("++|{}|", button_glyphs[b]),
        Input::Deactivate(b) => format!("--|{}|", button_glyphs[b]),
    }
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
            return c.to_string();
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
        keycode => return format!("{keycode:?}"),
    }
    .to_string()
}

pub fn fmt_keymods(keymods: KeyModifiers) -> String {
    use KeyModifiers as KMs;
    [
        keymods.contains(KMs::CONTROL).then_some("Ctrl"),
        keymods.contains(KMs::SHIFT).then_some("Shift"),
        keymods.contains(KMs::ALT).then_some("Alt"),
        keymods.contains(KMs::SUPER).then_some("Super"),
        keymods.contains(KMs::HYPER).then_some("Hyper"),
        keymods.contains(KMs::META).then_some("Meta"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("+")
}

pub fn fmt_key_with_keymods((key, keymods): (KeyCode, KeyModifiers)) -> String {
    if keymods.is_empty() {
        format!("[{}]", fmt_key(key))
    } else {
        format!("[{}+{}]", fmt_keymods(keymods), fmt_key(key))
    }
}

pub fn fmt_button_keybinds(button: Button, keybinds: &GameKeybinds) -> String {
    keybinds
        .iter()
        .filter_map(|(key_keymods, b)| (*b == button).then_some(fmt_key_with_keymods(*key_keymods)))
        .collect::<Vec<_>>()
        .join("")
}

pub fn get_game_keybinds_legend(keybinds: &GameKeybinds) -> KeybindsLegend {
    let fk = |k| fmt_key_with_keymods((k, KeyModifiers::NONE));
    let fb = |b| fmt_button_keybinds(b, keybinds);

    let icon_pause = fk(KeyCode::Esc);
    let icons_move = format!("{}{}", fb(Button::MoveLeft), fb(Button::MoveRight));
    let icons_rotate = format!(
        "{}{}{}",
        fb(Button::RotateLeft),
        fb(Button::Rotate180),
        fb(Button::RotateRight)
    );
    let icons_drop = format!("{}{}", fb(Button::DropSoft), fb(Button::DropHard));
    let icons_hold = fb(Button::HoldPiece);

    vec![
        (icon_pause, "pause"),
        (icons_move, "move"),
        (icons_rotate, "rotate"),
        (icons_drop, "drop"),
        (icons_hold, "hold"),
    ]
}

pub fn replay_keybinds_legend() -> KeybindsLegend {
    let fk = |k| fmt_key_with_keymods((k, KeyModifiers::NONE));

    let icon_pause = fk(KeyCode::Char(' '));
    let icons_speed = format!("{}{}", fk(KeyCode::Down), fk(KeyCode::Up));
    let icons_skip = format!("{}{}", fk(KeyCode::Left), fk(KeyCode::Right));
    // let icons_jump = format!("{}-{}", fk(KeyCode::Char('0')), fk(KeyCode::Char('9')));
    let icons_enter = fk(KeyCode::Enter);
    let icon_stop = fk(KeyCode::Esc);

    vec![
        (icon_pause, "pause"),
        (icons_speed, "speed -/+"),
        (icons_skip, "timeskip -/+"),
        // (icons_jump, "timejump #0%"),
        (icons_enter, "take over game"),
        (icon_stop, "stop"),
    ]
}

pub fn to_roman(mut num: u32) -> String {
    // Large roman numerals should be uncommon and have little convention
    // (<https://en.wikipedia.org/wiki/Roman_numerals#Large_numbers>),
    // return fallback decimal representation.
    if 4000 <= num {
        return num.to_string();
    }

    const ADDITIVE_NUMERAL_PARTS: [(&str, u32); 13] = [
        ("M", 1000),
        ("CM", 900),
        ("D", 500),
        ("CD", 400),
        ("C", 100),
        ("XC", 90),
        ("L", 50),
        ("XL", 40),
        ("X", 10),
        ("IX", 9),
        ("V", 5),
        ("IV", 4),
        ("I", 1),
    ];

    let mut string = String::new();
    for (str, value) in ADDITIVE_NUMERAL_PARTS {
        while num >= value {
            num -= value;
            string.push_str(str);
        }
    }

    string
}

// In an ideal world, a char is just a 1-char `str`.
// Unfortunately, converting between `u8`, `char` and `&str` is painful. :-(
//                     char --let mut bs=vec![0;len_utf8()];c.encode_utf8(&mut bs)--> &str
//    u8 --b.into()--> char --c.to_string()--> String ------------------s.as_str()--> &str
//    u8 --------------------------------------------str::from_utf8(&[b]).unwrap()--> &str
