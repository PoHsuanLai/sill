//! A DOM key as the center's [`CenterKey`] (design/20 §1.6: arrows move, Return opens or
//! expands, Delete clears, Escape closes; Backspace is the Mac keyboard's Delete). Pure.

use dioxus::prelude::Key;

use super::keys::CenterKey;

/// `key`, when the center handles it.
pub fn center_key(key: &Key) -> Option<CenterKey> {
    match key {
        Key::Escape => Some(CenterKey::Escape),
        Key::ArrowUp => Some(CenterKey::Up),
        Key::ArrowDown => Some(CenterKey::Down),
        Key::Enter => Some(CenterKey::Return),
        Key::Delete | Key::Backspace => Some(CenterKey::Delete),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::center_key;
    use crate::notification_center::CenterKey;
    use dioxus::prelude::Key;

    #[test]
    fn keys_map_per_design_20_section_1_6() {
        let cases = [
            (Key::Escape, Some(CenterKey::Escape)),
            (Key::ArrowUp, Some(CenterKey::Up)),
            (Key::ArrowDown, Some(CenterKey::Down)),
            (Key::Enter, Some(CenterKey::Return)),
            (Key::Delete, Some(CenterKey::Delete)),
            (Key::Backspace, Some(CenterKey::Delete)),
            (Key::ArrowLeft, None),
            (Key::Tab, None),
            (Key::Character("x".to_owned()), None),
        ];
        for (key, want) in cases {
            assert_eq!(center_key(&key), want, "{key:?}");
        }
    }
}
