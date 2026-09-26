//! Whether the chord is held, from shell-host's modifiers on focus (`use_modifiers`, G180 closed
//! by shell-host F66): the compositor sends the seat's modifiers right after
//! `wl_keyboard.enter`, so a release that beat the surface to the keyboard reads `Released`
//! without any key event, and a held chord reads `Held` before its first key. Pure.
//!
//! What it changes: a publish of `Released` while armed or shown is the release (a quick tap
//! switches with no UI; the panel closes and activates); once the chord is known `Held`, a
//! second chord inside the show delay is Tab held down, not a second tap, so it steps.

use shell_host::{Modifier, Modifiers};

use super::keys::Chord;
use super::machine::{Dir, SwIn, SwKey, Switcher};

/// The chord's state in `modifiers`: held while Alt (which Toshy holds for Cmd) or the logo key
/// is.
pub fn chord_of(modifiers: &Modifiers) -> Chord {
    match (modifiers.alt, modifiers.logo) {
        (Modifier::Released, Modifier::Released) => Chord::Released,
        _ => Chord::Held,
    }
}

/// The machine's input for a chord toward `dir` in `state`, `hold` being what the modifiers
/// said since the opening (`None`: nothing yet). Armed with the chord known held, it is a step
/// (the armed machine steps on a key); otherwise the chord itself.
pub fn chord_input(state: Switcher, hold: Option<Chord>, dir: Dir) -> SwIn {
    match (state, hold, dir) {
        (Switcher::Armed { .. }, Some(Chord::Held), Dir::Next) => SwIn::Key(SwKey::Tab),
        (Switcher::Armed { .. }, Some(Chord::Held), Dir::Prev) => SwIn::Key(SwKey::ShiftTab),
        _ => SwIn::Chord(dir),
    }
}

#[cfg(test)]
mod tests {
    use shell_host::{Modifier, Modifiers};
    use sill_services::Stamp;

    use super::{chord_input, chord_of};
    use crate::surfaces::switcher::keys::Chord;
    use crate::surfaces::switcher::machine::{Dir, SwIn, SwKey, Switcher};

    #[test]
    fn alt_or_logo_holds_the_chord() {
        let with = |alt, logo, shift| Modifiers {
            alt,
            logo,
            shift,
            ..Modifiers::default()
        };
        let (held, up) = (Modifier::Held, Modifier::Released);
        let cases = [
            (with(held, up, up), Chord::Held),
            (with(up, held, up), Chord::Held),
            (with(held, held, held), Chord::Held),
            (with(up, up, held), Chord::Released),
            (Modifiers::default(), Chord::Released),
        ];
        for (modifiers, want) in cases {
            assert_eq!(chord_of(&modifiers), want, "{modifiers:?}");
        }
    }

    #[test]
    fn a_chord_known_held_steps_while_armed() {
        let armed = Switcher::Armed {
            since: Stamp(0),
            sel: 1,
        };
        let cases = [
            (armed, Some(Chord::Held), Dir::Next, SwIn::Key(SwKey::Tab)),
            (
                armed,
                Some(Chord::Held),
                Dir::Prev,
                SwIn::Key(SwKey::ShiftTab),
            ),
            (armed, None, Dir::Next, SwIn::Chord(Dir::Next)),
            (
                armed,
                Some(Chord::Released),
                Dir::Next,
                SwIn::Chord(Dir::Next),
            ),
            (
                Switcher::Hidden,
                Some(Chord::Held),
                Dir::Next,
                SwIn::Chord(Dir::Next),
            ),
            (
                Switcher::Shown { sel: 1 },
                Some(Chord::Held),
                Dir::Prev,
                SwIn::Chord(Dir::Prev),
            ),
        ];
        for (state, hold, dir, want) in cases {
            assert_eq!(chord_input(state, hold, dir), want, "{state:?} {hold:?}");
        }
    }
}
