//! The switcher's keys and the chord's release: pure.
//!
//! Which modifier holds the chord: COSMIC binds `Alt+Tab` and `Super+Tab` to the switcher, and
//! Toshy turns a physical Cmd+Tab into `Alt+Tab` with Alt held for as long as Cmd is
//! (`C("RC-Tab"): [bind, C("Alt-Tab")]` in its config), so the chord is held while `ALT` or
//! `META` is. A key-up event's modifiers are the state before it (shell-host applies
//! `wl_keyboard.modifiers`, which the compositor sends after the key), so the release of Alt
//! arrives as a key-up of `Alt` whose modifiers still hold `ALT`: [`chord_after_keyup`] reads the
//! key, not only the modifiers. A release before the surface has the keyboard is never seen as
//! a key; that needs shell-host's modifiers on focus.

use chordkit::Modifier;
use dioxus::prelude::{Key, Modifiers};

use ds::base::command::modifiers_of;
use ds_behaviour::switcher::SwKey;

/// Whether the chord's modifier is still down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chord {
    Held,
    Released,
}

/// The machine's key for a key-down of `key` with `modifiers`: Tab (Shift+Tab is `ShiftTab`),
/// `` ` ``, the arrows, Q, H (either case), Escape; anything else is `None`.
pub fn sw_key(key: &Key, modifiers: Modifiers) -> Option<SwKey> {
    match key {
        Key::Tab if modifiers.contains(Modifiers::SHIFT) => Some(SwKey::ShiftTab),
        Key::Tab => Some(SwKey::Tab),
        Key::ArrowLeft => Some(SwKey::Left),
        Key::ArrowRight => Some(SwKey::Right),
        Key::Escape => Some(SwKey::Escape),
        Key::Character(text) => match text.to_lowercase().as_str() {
            "`" => Some(SwKey::Grave),
            "q" => Some(SwKey::Q),
            "h" => Some(SwKey::H),
            _ => None,
        },
        _ => None,
    }
}

/// Whether the chord is still held after a key-up of `key` (`modifiers` as the event carries
/// them): an `Alt` or `Meta` (`Super`) key-up releases it unless the other of the two is still
/// in `modifiers`; any other key-up leaves it as `modifiers` say.
pub fn chord_after_keyup(key: &Key, modifiers: Modifiers) -> Chord {
    match key {
        Key::Alt => held_while(&OS_KEY, modifiers),
        Key::Meta | Key::Super => held_while(&[Modifier::Alt], modifiers),
        _ => held_while(&[Modifier::Alt, Modifier::Super, Modifier::Meta], modifiers),
    }
}

/// The Command key as a window reports it: Super or Meta.
const OS_KEY: [Modifier; 2] = [Modifier::Super, Modifier::Meta];

/// Held while any of `holding` is in `modifiers`. This reads a modifier's release, which is not a
/// chord, so it is not the keymap's to resolve.
fn held_while(holding: &[Modifier], modifiers: Modifiers) -> Chord {
    let held = modifiers_of(modifiers);
    if holding.iter().any(|modifier| held.contains(*modifier)) {
        Chord::Held
    } else {
        Chord::Released
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::{Key, Modifiers};

    use super::{Chord, chord_after_keyup, sw_key};
    use ds_behaviour::switcher::SwKey;

    #[test]
    fn keys_while_up() {
        let ch = |c: &str| Key::Character(c.to_owned());
        let alt = Modifiers::ALT;
        let cases = [
            (Key::Tab, alt, Some(SwKey::Tab)),
            (Key::Tab, alt | Modifiers::SHIFT, Some(SwKey::ShiftTab)),
            (ch("`"), alt, Some(SwKey::Grave)),
            (Key::ArrowLeft, alt, Some(SwKey::Left)),
            (Key::ArrowRight, alt, Some(SwKey::Right)),
            (ch("q"), alt, Some(SwKey::Q)),
            (ch("Q"), alt, Some(SwKey::Q)),
            (ch("h"), Modifiers::META, Some(SwKey::H)),
            (Key::Escape, alt, Some(SwKey::Escape)),
            (ch("x"), alt, None),
            (Key::Enter, alt, None),
        ];
        for (key, modifiers, want) in cases {
            assert_eq!(sw_key(&key, modifiers), want, "{key:?} {modifiers:?}");
        }
    }

    #[test]
    fn the_chord_goes_up_with_its_modifier() {
        let cases = [
            (Key::Alt, Modifiers::ALT, Chord::Released),
            (Key::Meta, Modifiers::META, Chord::Released),
            (Key::Super, Modifiers::META, Chord::Released),
            (Key::Alt, Modifiers::ALT | Modifiers::META, Chord::Held),
            (Key::Alt, Modifiers::ALT | Modifiers::SUPER, Chord::Held),
            (Key::Tab, Modifiers::SUPER, Chord::Held),
            (Key::Tab, Modifiers::ALT, Chord::Held),
            (Key::Shift, Modifiers::ALT | Modifiers::SHIFT, Chord::Held),
            (Key::Tab, Modifiers::empty(), Chord::Released),
        ];
        for (key, modifiers, want) in cases {
            assert_eq!(
                chord_after_keyup(&key, modifiers),
                want,
                "{key:?} {modifiers:?}"
            );
        }
    }
}
