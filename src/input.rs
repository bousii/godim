use godot::global::Key;

fn apply_mods(raw: String, is_special: bool, ctrl: bool, shift: bool, alt: bool) -> String {
    let mut mods = String::new();
    if ctrl {
        mods.push_str("C-");
    }
    if shift && is_special {
        mods.push_str("S-");
    }
    if alt {
        mods.push_str("A-");
    }
    if mods.is_empty() && !is_special {
        raw
    } else {
        format!("<{}{}>", mods, raw)
    }
}

pub fn keycode_to_nvim(
    key: Key,
    unicode: i64,
    shift: bool,
    ctrl: bool,
    alt: bool,
) -> Option<String> {
    let raw = match key {
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::ESCAPE => "Esc",
        Key::KP_ENTER | Key::ENTER => "CR",
        Key::BACKSPACE => "BS",
        Key::TAB => "Tab",
        Key::UP => "Up",
        Key::DOWN => "Down",
        Key::LEFT => "Left",
        Key::RIGHT => "Right",
        Key::DELETE => "Del",
        Key::HOME => "Home",
        Key::END => "End",
        Key::PAGEUP => "PageUp",
        Key::PAGEDOWN => "PageDown",
        Key::INSERT => "Insert",
        Key::SHIFT | Key::CTRL | Key::ALT | Key::META => return None,
        _ => {
            let c = char::from_u32(unicode as u32)?;
            if c == '\0' {
                return None;
            }
            return Some(apply_mods(c.to_string(), false, ctrl, shift, alt));
        }
    };
    Some(apply_mods(raw.to_string(), true, ctrl, shift, alt))
}
