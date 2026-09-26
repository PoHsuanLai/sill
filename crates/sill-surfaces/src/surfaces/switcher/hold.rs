//! Whether the chord is held, from shell-host's modifiers on focus (`use_modifiers`, G180 closed
//! by shell-host F66): the compositor sends the seat's modifiers right after
//! `wl_keyboard.enter`, so a held chord reads `Held` before its first key, and its release reads
//! `Released` with no key event. Pure.
//!
//! What it changes: a publish of `Released` while armed or shown is the release (the panel closes
//! and activates); once the chord is known `Held`, a second chord inside the show delay is Tab
//! held down, not a second tap, so it steps. shell-host publishes only a change, so an opening
//! starts from the value it mounts on (`watch.rs`: its baseline) and a focus that finds the same
//! state publishes nothing. The show delay then decides: an opening that reaches it without the
//! chord read held was a tap, and switches with no UI; one read held shows the panel. (The
//! keyboard focus lands 50 to 90 ms after the chord on the nested cosmic-comp, inside the
//! 150 ms delay; a focus later than the delay would read a held chord as a tap.)

use shell_host::{Modifier, Modifiers};

use super::keys::Chord;
use super::machine::{Dir, SwIn, SwKey, Switcher};

/// Whether an opening follows the modifiers: on a host surface, for the compositor's apps
/// (`watch.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Watch {
    #[default]
    Off,
    On,
}

/// What the modifiers have told one opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hold {
    pub watch: Watch,
    /// The last publish; `None` until the first.
    pub chord: Option<Chord>,
}

/// The machine's input when the show delay's timer fires in `state`: followed and never read
/// held, the opening was a tap (the release); otherwise the tick.
pub fn tick_input(state: Switcher, hold: Hold) -> SwIn {
    match (state, hold.watch, hold.chord) {
        (Switcher::Armed { .. }, Watch::On, None | Some(Chord::Released)) => SwIn::ModifierReleased,
        _ => SwIn::Tick,
    }
}

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

    use super::{Hold, Watch, chord_input, chord_of, tick_input};
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
    fn the_delay_ends_a_tap_that_never_read_held() {
        let armed = Switcher::Armed {
            since: Stamp(0),
            sel: 1,
        };
        let hold = |watch, chord| Hold { watch, chord };
        let cases = [
            (armed, hold(Watch::On, None), SwIn::ModifierReleased),
            (
                armed,
                hold(Watch::On, Some(Chord::Released)),
                SwIn::ModifierReleased,
            ),
            (armed, hold(Watch::On, Some(Chord::Held)), SwIn::Tick),
            (armed, hold(Watch::Off, None), SwIn::Tick),
            (
                Switcher::Shown { sel: 1 },
                hold(Watch::On, None),
                SwIn::Tick,
            ),
        ];
        for (state, hold, want) in cases {
            assert_eq!(tick_input(state, hold), want, "{state:?} {hold:?}");
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
