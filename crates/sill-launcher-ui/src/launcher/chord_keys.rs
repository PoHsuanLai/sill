//! Row chords between sill's Mac-worded [`Chord`] and the keyboard: how a chord is
//! drawn (quire's `Shortcut`, ⌃⌥⇧⌘ in order, Command as `⌘`) and which chord a DOM key is.
//! Command is whatever the platform layer resolves it to: Toshy maps it to Ctrl, so a Ctrl (or a
//! Super that reached us unmapped) with a letter is Command (quire design/27 §6.2 (e)). Until
//! quire's `StandardAction` table lands the drawing is this one mapping. Pure.

use dioxus::prelude::{Key, Modifiers};
use ds::prelude::*;
use sill_launcher::{ActionKeys, Chord, ChordKey, Modifier};

/// `chord` as quire draws a shortcut.
pub fn shortcut(chord: &Chord) -> Shortcut {
    let mods = chord.mods.iter().map(|m| match m {
        Modifier::Control => ShortcutKey::Ctrl,
        Modifier::Option => ShortcutKey::Alt,
        Modifier::Shift => ShortcutKey::Shift,
        Modifier::Command => ShortcutKey::Super,
    });
    let key = match chord.key {
        ChordKey::Enter => ShortcutKey::Enter,
        ChordKey::Space => ShortcutKey::Space,
        ChordKey::Left => ShortcutKey::Left,
        ChordKey::Right => ShortcutKey::Right,
        ChordKey::Backspace => ShortcutKey::Backspace,
        ChordKey::Char(c) => ShortcutKey::Char(c),
    };
    Shortcut(mods.chain(std::iter::once(key)).collect())
}

/// An action's chord as a shortcut, when it has one.
pub fn action_shortcut(keys: &ActionKeys) -> Option<Shortcut> {
    match keys {
        ActionKeys::Chord(chord) => Some(shortcut(chord)),
        ActionKeys::None => None,
    }
}

/// The chord `key` with `modifiers` held is, when Command is among them: Ctrl or Super is
/// Command; Alt is Option; Shift is Shift.
pub fn chord_of(key: &Key, modifiers: Modifiers) -> Option<Chord> {
    if !modifiers.intersects(Modifiers::CONTROL | Modifiers::META | Modifiers::SUPER) {
        return None;
    }
    let key = match key {
        Key::Enter => ChordKey::Enter,
        Key::ArrowLeft => ChordKey::Left,
        Key::ArrowRight => ChordKey::Right,
        Key::Backspace => ChordKey::Backspace,
        Key::Character(text) if text == " " => ChordKey::Space,
        Key::Character(text) => {
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => ChordKey::Char(c.to_ascii_lowercase()),
                _ => return None,
            }
        }
        _ => return None,
    };
    let held = [
        (Modifiers::ALT, Modifier::Option),
        (Modifiers::SHIFT, Modifier::Shift),
    ];
    let mods = held
        .iter()
        .filter(|(bit, _)| modifiers.contains(*bit))
        .map(|&(_, m)| m)
        .chain(std::iter::once(Modifier::Command))
        .collect();
    Some(Chord::new(mods, key))
}

#[cfg(test)]
mod tests {
    use super::{chord_of, shortcut};
    use dioxus::prelude::{Key, Modifiers};
    use sill_launcher::{Chord, ChordKey, Modifier, chords};

    #[test]
    fn a_chord_draws_in_the_macs_order() {
        let cases: Vec<(Chord, &str)> = vec![
            (chords::reveal(), "⌘R"),
            (chords::forget(), "⌘⌫"),
            (
                Chord::new(
                    vec![Modifier::Command, Modifier::Shift],
                    ChordKey::Char('c'),
                ),
                "⇧⌘C",
            ),
        ];
        for (chord, want) in cases {
            assert_eq!(shortcut(&chord).glyphs(), want, "{chord:?}");
        }
    }

    #[test]
    fn ctrl_or_super_with_a_key_is_a_command_chord() {
        let k = |s: &str| Key::Character(s.to_owned());
        let cases: Vec<(Key, Modifiers, Option<Chord>)> = vec![
            (k("r"), Modifiers::CONTROL, Some(chords::reveal())),
            (k("R"), Modifiers::SUPER, Some(chords::reveal())),
            (k("y"), Modifiers::CONTROL, Some(chords::quick_look())),
            (Key::Backspace, Modifiers::CONTROL, Some(chords::forget())),
            (
                k("c"),
                Modifiers::CONTROL | Modifiers::SHIFT,
                Some(Chord::new(
                    vec![Modifier::Shift, Modifier::Command],
                    ChordKey::Char('c'),
                )),
            ),
            (k("r"), Modifiers::empty(), None),
            (k("r"), Modifiers::ALT, None),
            (Key::Escape, Modifiers::CONTROL, None),
        ];
        for (key, mods, want) in cases {
            assert_eq!(chord_of(&key, mods), want, "{key:?} {mods:?}");
        }
    }
}
