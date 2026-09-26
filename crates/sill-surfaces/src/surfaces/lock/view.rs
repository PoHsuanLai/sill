//! What the lock screen shows, from the auth service's state, the keyboard and the clock: pure.

use dioxus::prelude::Modifiers;
use sill_services::AuthPhase;
use sill_services::notifications::UnixMs;
use sill_settings::LockClock;

/// Whether Caps Lock is on, from the modifiers of the last key event the surface saw (the lock
/// surface has the keyboard, so it sees them; shell-host's `modifiers_from` sets `CAPS_LOCK`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CapsLock {
    #[default]
    Off,
    On,
}

/// Caps Lock from `modifiers`.
pub fn caps_lock(modifiers: Modifiers) -> CapsLock {
    todo!("M11 fill: a-lock ({modifiers:?})")
}

/// quire's `LockPrompt` states (sill's mirror until quire lands).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PromptState {
    #[default]
    Idle,
    Checking,
    /// Shake once and clear the field.
    Wrong,
    /// "Try again in N seconds", the field disabled; `seconds` rounded up.
    LockedOut {
        seconds: u64,
    },
}

/// The prompt's state for `phase` at `now`: `Accepted` shows `Checking` (the unlock is on its
/// way), `Unavailable` shows `Idle` (the service keeps its notes for the line under the field),
/// a `LockedOut` whose `until` has passed shows `Idle`.
pub fn prompt_state(phase: &AuthPhase, now: UnixMs) -> PromptState {
    todo!("M11 fill: a-lock ({phase:?} {now:?})")
}

/// The clock's two lines.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ClockLines {
    /// `14:05` (`H24`) or `2:05` (`H12`, no AM/PM, macOS's lock screen).
    pub time: String,
    /// `Saturday, 26 September` (macOS: the weekday and the date above the time).
    pub date: String,
}

/// `now` as the lock screen's clock.
pub fn clock_lines(now: &jiff::Zoned, format: LockClock) -> ClockLines {
    todo!("M11 fill: a-lock ({now} {format:?})")
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::Modifiers;
    use sill_services::AuthPhase;
    use sill_services::auth::AuthFault;
    use sill_services::notifications::UnixMs;
    use sill_settings::LockClock;

    use super::{CapsLock, ClockLines, PromptState, caps_lock, clock_lines, prompt_state};

    #[test]
    #[ignore = "M11 fill: a-lock"]
    fn caps_lock_from_the_modifiers() {
        let cases = [
            (Modifiers::empty(), CapsLock::Off),
            (Modifiers::SHIFT, CapsLock::Off),
            (Modifiers::CAPS_LOCK, CapsLock::On),
            (Modifiers::CAPS_LOCK | Modifiers::SHIFT, CapsLock::On),
        ];
        for (modifiers, want) in cases {
            assert_eq!(caps_lock(modifiers), want, "{modifiers:?}");
        }
    }

    #[test]
    #[ignore = "M11 fill: a-lock"]
    fn prompt_states() {
        let now = UnixMs(1_000_000);
        let cases = [
            (AuthPhase::Idle, PromptState::Idle),
            (AuthPhase::Checking, PromptState::Checking),
            (AuthPhase::Wrong, PromptState::Wrong),
            (AuthPhase::Accepted, PromptState::Checking),
            (
                AuthPhase::Unavailable(AuthFault::TimedOut),
                PromptState::Idle,
            ),
            (
                AuthPhase::LockedOut {
                    until: UnixMs(1_059_001),
                },
                PromptState::LockedOut { seconds: 60 },
            ),
            (
                AuthPhase::LockedOut {
                    until: UnixMs(1_000_000),
                },
                PromptState::Idle,
            ),
        ];
        for (phase, want) in cases {
            assert_eq!(prompt_state(&phase, now), want, "{phase:?}");
        }
    }

    #[test]
    #[ignore = "M11 fill: a-lock"]
    fn clock_lines_in_both_formats() {
        let at = |text: &str| {
            text.parse::<jiff::Zoned>()
                .unwrap_or_else(|e| panic!("{text}: {e}"))
        };
        let lines = |time: &str, date: &str| ClockLines {
            time: time.to_owned(),
            date: date.to_owned(),
        };
        let cases = [
            (
                "2026-09-26T14:05:00+08:00[Asia/Taipei]",
                LockClock::H24,
                lines("14:05", "Saturday, 26 September"),
            ),
            (
                "2026-09-26T14:05:00+08:00[Asia/Taipei]",
                LockClock::H12,
                lines("2:05", "Saturday, 26 September"),
            ),
            (
                "2026-01-01T00:07:00+00:00[UTC]",
                LockClock::H12,
                lines("12:07", "Thursday, 1 January"),
            ),
            (
                "2026-01-01T09:07:00+00:00[UTC]",
                LockClock::H24,
                lines("09:07", "Thursday, 1 January"),
            ),
        ];
        for (text, format, want) in cases {
            assert_eq!(clock_lines(&at(text), format), want, "{text} {format:?}");
        }
    }
}
