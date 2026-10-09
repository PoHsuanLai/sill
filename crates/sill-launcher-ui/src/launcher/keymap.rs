//! A DOM key as the panel's [`LauncherKey`] (design/06 §20.2): arrows, Enter, Tab and
//! Shift+Tab, Escape, and Ctrl+K or Alt+K (Toshy maps Cmd to Ctrl; an app-id-less layer
//! surface may not get Toshy's remap, so Alt+K is accepted too, plan "shell risks"). Pure.

use dioxus::prelude::{Key, Modifiers};
use ds::prelude::is_command;

use crate::launcher::companion::CompanionKey;
use crate::launcher::keys::LauncherKey;

/// `key` with `modifiers` held, when the panel handles it.
pub fn launcher_key(key: &Key, modifiers: Modifiers) -> Option<LauncherKey> {
    let chord = is_command(modifiers) || modifiers.contains(Modifiers::ALT);
    match key {
        Key::ArrowUp => Some(LauncherKey::Up),
        Key::ArrowDown => Some(LauncherKey::Down),
        Key::Enter => Some(LauncherKey::Enter),
        Key::Escape => Some(LauncherKey::Escape),
        Key::Tab if modifiers.contains(Modifiers::SHIFT) => Some(LauncherKey::ShiftTab),
        Key::Tab => Some(LauncherKey::Tab),
        Key::Character(text) if chord && text.eq_ignore_ascii_case("k") => {
            Some(LauncherKey::Actions)
        }
        _ => None,
    }
}

/// `key` with `modifiers` held, when it is one companion mode reads before the palette does:
/// Escape, command-Enter and command-period (Toshy maps Cmd to Ctrl, so Ctrl and Meta both
/// count).
pub fn companion_key(key: &Key, modifiers: Modifiers) -> Option<CompanionKey> {
    let command = is_command(modifiers);
    match key {
        Key::Escape if modifiers.is_empty() => Some(CompanionKey::Escape),
        Key::Enter if command => Some(CompanionKey::CommandEnter),
        Key::Character(text) if command && text == "." => Some(CompanionKey::CommandPeriod),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{companion_key, launcher_key};
    use crate::launcher::keys::LauncherKey;
    use dioxus::prelude::{Key, Modifiers};

    #[test]
    fn keys_map_per_design_06_section_20_2() {
        let k = |s: &str| Key::Character(s.to_owned());
        let none = Modifiers::empty();
        let cases: Vec<(Key, Modifiers, Option<LauncherKey>)> = vec![
            (Key::ArrowUp, none, Some(LauncherKey::Up)),
            (Key::ArrowDown, none, Some(LauncherKey::Down)),
            (Key::Enter, none, Some(LauncherKey::Enter)),
            (Key::Escape, none, Some(LauncherKey::Escape)),
            (Key::Tab, none, Some(LauncherKey::Tab)),
            (Key::Tab, Modifiers::SHIFT, Some(LauncherKey::ShiftTab)),
            (k("k"), Modifiers::CONTROL, Some(LauncherKey::Actions)),
            (
                k("K"),
                Modifiers::CONTROL | Modifiers::SHIFT,
                Some(LauncherKey::Actions),
            ),
            (k("k"), Modifiers::ALT, Some(LauncherKey::Actions)),
            // Plain typing is the field's.
            (k("k"), none, None),
            (k("k"), Modifiers::SHIFT, None),
            (k("j"), Modifiers::CONTROL, None),
            (Key::Backspace, none, None),
        ];
        for (key, modifiers, want) in cases {
            assert_eq!(launcher_key(&key, modifiers), want, "{key:?} {modifiers:?}");
        }
    }

    #[test]
    fn companion_keys_are_escape_and_the_command_pair() {
        use crate::launcher::companion::CompanionKey as K;
        let k = |s: &str| Key::Character(s.to_owned());
        let none = Modifiers::empty();
        let cases: Vec<(Key, Modifiers, Option<K>)> = vec![
            (Key::Escape, none, Some(K::Escape)),
            (Key::Escape, Modifiers::SHIFT, None),
            (Key::Enter, none, None),
            (Key::Enter, Modifiers::CONTROL, Some(K::CommandEnter)),
            (Key::Enter, Modifiers::META, Some(K::CommandEnter)),
            (Key::Enter, Modifiers::ALT, None),
            (k("."), Modifiers::CONTROL, Some(K::CommandPeriod)),
            (k("."), Modifiers::META, Some(K::CommandPeriod)),
            (k("."), Modifiers::SUPER, Some(K::CommandPeriod)),
            (Key::Enter, Modifiers::SUPER, Some(K::CommandEnter)),
            (k("."), none, None),
            (k(","), Modifiers::CONTROL, None),
        ];
        for (key, modifiers, want) in cases {
            assert_eq!(
                companion_key(&key, modifiers),
                want,
                "{key:?} {modifiers:?}"
            );
        }
    }
}
