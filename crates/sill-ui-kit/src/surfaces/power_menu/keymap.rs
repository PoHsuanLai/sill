//! A DOM key as the menu's [`PowerMenuKey`] (design/20 §1.8: arrows move, Enter confirms, Esc
//! closes; Tab and Shift+Tab move as the arrows do). Pure.

use dioxus::prelude::{Key, Modifiers};

use crate::surfaces::power_menu::keys::PowerMenuKey;

/// `key` with `modifiers` held, when the menu handles it.
pub fn power_menu_key(key: &Key, modifiers: Modifiers) -> Option<PowerMenuKey> {
    match key {
        Key::ArrowLeft => Some(PowerMenuKey::Left),
        Key::ArrowRight => Some(PowerMenuKey::Right),
        Key::ArrowUp => Some(PowerMenuKey::Up),
        Key::ArrowDown => Some(PowerMenuKey::Down),
        Key::Tab if modifiers.contains(Modifiers::SHIFT) => Some(PowerMenuKey::BackTab),
        Key::Tab => Some(PowerMenuKey::Tab),
        Key::Enter => Some(PowerMenuKey::Enter),
        Key::Escape => Some(PowerMenuKey::Escape),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::power_menu_key;
    use crate::surfaces::power_menu::keys::PowerMenuKey;
    use dioxus::prelude::{Key, Modifiers};

    #[test]
    fn keys_map_per_design_20_section_1_8() {
        let none = Modifiers::empty();
        let cases: Vec<(Key, Modifiers, Option<PowerMenuKey>)> = vec![
            (Key::ArrowLeft, none, Some(PowerMenuKey::Left)),
            (Key::ArrowRight, none, Some(PowerMenuKey::Right)),
            (Key::ArrowUp, none, Some(PowerMenuKey::Up)),
            (Key::ArrowDown, none, Some(PowerMenuKey::Down)),
            (Key::Tab, none, Some(PowerMenuKey::Tab)),
            (Key::Tab, Modifiers::SHIFT, Some(PowerMenuKey::BackTab)),
            (Key::Enter, none, Some(PowerMenuKey::Enter)),
            (Key::Escape, none, Some(PowerMenuKey::Escape)),
            (Key::Character("a".to_owned()), none, None),
            (Key::Backspace, none, None),
        ];
        for (key, modifiers, want) in cases {
            assert_eq!(power_menu_key(&key, modifiers), want, "{key:?}");
        }
    }
}
