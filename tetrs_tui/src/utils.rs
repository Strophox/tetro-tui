use std::time::Duration;

use crossterm::event::KeyCode;
use tetrs_engine::Button;

use crate::game_input_handlers::live_terminal::Keybinds;

pub fn fmt_duration(dur: &Duration) -> String {
    format!(
        "{}min {}.{:02}s",
        dur.as_secs() / 60,
        dur.as_secs() % 60,
        dur.as_millis() % 1000 / 10
    )
}

pub fn fmt_key(key: KeyCode) -> String {
    use crossterm::event::ModifierKeyCode as M;
    use KeyCode as K;
    format!("[{}]", 'String_not_str: {
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
            K::F(k) => break 'String_not_str format!("F{k}"),
            K::Char(' ') => "Space",
            K::Char(c) => break 'String_not_str c.to_uppercase().to_string(),
            //K::Esc => "Esc",
            K::Modifier(M::LeftShift) => "LShift",
            K::Modifier(M::RightShift) => "RShift",
            K::Modifier(M::LeftControl) => "LCtrl",
            K::Modifier(M::RightControl) => "RCtrl",
            K::Modifier(M::LeftSuper) => "LSuper",
            K::Modifier(M::RightSuper) => "RSuper",
            K::Modifier(M::LeftAlt) => "LAlt",
            K::Modifier(M::RightAlt) => "RAlt",
            K::Modifier(M::IsoLevel3Shift) => "AltGr",
            k => break 'String_not_str format!("{:?}", k),
        }
        .to_string()
    })
}

pub fn fmt_keybinds(button: Button, keybinds: &Keybinds) -> String {
    keybinds
        .iter()
        .filter_map(|(&k, &b)| (b == button).then_some(fmt_key(k)))
        .collect::<Vec<String>>()
        .join("")
}
