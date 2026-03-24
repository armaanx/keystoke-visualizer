use crate::model::{KeyCell, KeyboardLayout};
use std::collections::BTreeMap;

pub fn keyboard_layout(layout: KeyboardLayout) -> Vec<KeyCell> {
    match layout {
        KeyboardLayout::Ansi104 | KeyboardLayout::Iso105 => standard_layout(),
    }
}

pub fn pretty_key_name(raw: &str) -> String {
    match raw {
        "Return" => "Enter".to_string(),
        "Space" => "Space".to_string(),
        "Tab" => "Tab".to_string(),
        "CapsLock" => "Caps Lock".to_string(),
        "ShiftLeft" => "Left Shift".to_string(),
        "ShiftRight" => "Right Shift".to_string(),
        "ControlLeft" => "Left Ctrl".to_string(),
        "ControlRight" => "Right Ctrl".to_string(),
        "MetaLeft" => "Left Win".to_string(),
        "MetaRight" => "Right Win".to_string(),
        "Alt" => "Alt".to_string(),
        "AltGr" => "AltGr".to_string(),
        "BackQuote" => "`".to_string(),
        "Minus" => "-".to_string(),
        "Equal" => "=".to_string(),
        "LeftBracket" => "[".to_string(),
        "RightBracket" => "]".to_string(),
        "BackSlash" => "\\".to_string(),
        "SemiColon" => ";".to_string(),
        "Quote" => "'".to_string(),
        "Comma" => ",".to_string(),
        "Dot" => ".".to_string(),
        "Slash" => "/".to_string(),
        _ if raw.starts_with("Key") && raw.len() == 4 => raw[3..].to_string(),
        _ if raw.starts_with("Num") && raw.len() == 4 => raw[3..].to_string(),
        _ => raw.to_string(),
    }
}

pub fn sorted_key_usage(counts: &BTreeMap<String, u64>) -> Vec<(String, u64)> {
    let mut items: Vec<_> = counts.iter().map(|(key, count)| (key.clone(), *count)).collect();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    items
}

fn standard_layout() -> Vec<KeyCell> {
    let mut keys = Vec::new();
    let row = |items: &[(&'static str, &'static str, f64)], keys: &mut Vec<KeyCell>| {
        for (id, label, units) in items {
            let _ = units;
            keys.push(KeyCell { id, label });
        }
    };

    row(
        &[
            ("Escape", "Esc", 1.0),
            ("F1", "F1", 1.0),
            ("F2", "F2", 1.0),
            ("F3", "F3", 1.0),
            ("F4", "F4", 1.0),
            ("F5", "F5", 1.0),
            ("F6", "F6", 1.0),
            ("F7", "F7", 1.0),
            ("F8", "F8", 1.0),
            ("F9", "F9", 1.0),
            ("F10", "F10", 1.0),
            ("F11", "F11", 1.0),
            ("F12", "F12", 1.0),
        ],
        &mut keys,
    );
    row(
        &[
            ("BackQuote", "`", 1.0),
            ("Num1", "1", 1.0),
            ("Num2", "2", 1.0),
            ("Num3", "3", 1.0),
            ("Num4", "4", 1.0),
            ("Num5", "5", 1.0),
            ("Num6", "6", 1.0),
            ("Num7", "7", 1.0),
            ("Num8", "8", 1.0),
            ("Num9", "9", 1.0),
            ("Num0", "0", 1.0),
            ("Minus", "-", 1.0),
            ("Equal", "=", 1.0),
            ("Backspace", "Backspace", 2.0),
        ],
        &mut keys,
    );
    row(
        &[
            ("Tab", "Tab", 1.5),
            ("KeyQ", "Q", 1.0),
            ("KeyW", "W", 1.0),
            ("KeyE", "E", 1.0),
            ("KeyR", "R", 1.0),
            ("KeyT", "T", 1.0),
            ("KeyY", "Y", 1.0),
            ("KeyU", "U", 1.0),
            ("KeyI", "I", 1.0),
            ("KeyO", "O", 1.0),
            ("KeyP", "P", 1.0),
            ("LeftBracket", "[", 1.0),
            ("RightBracket", "]", 1.0),
            ("BackSlash", "\\", 1.5),
        ],
        &mut keys,
    );
    row(
        &[
            ("CapsLock", "Caps", 1.75),
            ("KeyA", "A", 1.0),
            ("KeyS", "S", 1.0),
            ("KeyD", "D", 1.0),
            ("KeyF", "F", 1.0),
            ("KeyG", "G", 1.0),
            ("KeyH", "H", 1.0),
            ("KeyJ", "J", 1.0),
            ("KeyK", "K", 1.0),
            ("KeyL", "L", 1.0),
            ("SemiColon", ";", 1.0),
            ("Quote", "'", 1.0),
            ("Return", "Enter", 2.25),
        ],
        &mut keys,
    );
    row(
        &[
            ("ShiftLeft", "Shift", 2.25),
            ("KeyZ", "Z", 1.0),
            ("KeyX", "X", 1.0),
            ("KeyC", "C", 1.0),
            ("KeyV", "V", 1.0),
            ("KeyB", "B", 1.0),
            ("KeyN", "N", 1.0),
            ("KeyM", "M", 1.0),
            ("Comma", ",", 1.0),
            ("Dot", ".", 1.0),
            ("Slash", "/", 1.0),
            ("ShiftRight", "Shift", 2.75),
        ],
        &mut keys,
    );
    row(
        &[
            ("ControlLeft", "Ctrl", 1.25),
            ("MetaLeft", "Win", 1.25),
            ("Alt", "Alt", 1.25),
            ("Space", "Space", 6.25),
            ("AltGr", "AltGr", 1.25),
            ("MetaRight", "Win", 1.25),
            ("Menu", "Menu", 1.25),
            ("ControlRight", "Ctrl", 1.25),
        ],
        &mut keys,
    );

    keys
}
