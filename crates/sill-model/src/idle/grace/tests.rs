use GraceVerdict::{Ask, NoPassword};
use LockCause::{Action, IdleScreenOff, Logind, Restored, Sleep};

use super::{GraceCase, GraceVerdict, WakeDelay, grace_verdict, ticket};
use crate::idle::{IdleSecs, LockGrace};
use crate::locker::LockCause;

/// One row: name, cause, grace (s), wake after (ms), a sleep since, the verdict.
type Row = (
    &'static str,
    Option<LockCause>,
    u32,
    u64,
    bool,
    GraceVerdict,
);

/// The table: every lock cause, the grace, the wake delay and a sleep in between.
const NO_SLEEP: bool = false;
const SLEPT: bool = true;

const CASES: &[Row] = &[
    (
        "idle, woke at 4.9 s",
        Some(IdleScreenOff),
        5,
        4_900,
        NO_SLEEP,
        NoPassword,
    ),
    (
        "idle, woke at once",
        Some(IdleScreenOff),
        5,
        0,
        NO_SLEEP,
        NoPassword,
    ),
    (
        "idle, woke at exactly 5 s",
        Some(IdleScreenOff),
        5,
        5_000,
        NO_SLEEP,
        Ask,
    ),
    (
        "idle, woke at 30 s",
        Some(IdleScreenOff),
        5,
        30_000,
        NO_SLEEP,
        Ask,
    ),
    (
        "idle, grace 0 always asks",
        Some(IdleScreenOff),
        0,
        100,
        NO_SLEEP,
        Ask,
    ),
    (
        "idle, grace 60, woke at 59 s",
        Some(IdleScreenOff),
        60,
        59_000,
        NO_SLEEP,
        NoPassword,
    ),
    (
        "idle, then a sleep and resume",
        Some(IdleScreenOff),
        5,
        1_000,
        SLEPT,
        Ask,
    ),
    (
        "sill's Lock (⌃⌘Q, launcher, power menu)",
        Some(Action),
        5,
        100,
        NO_SLEEP,
        Ask,
    ),
    (
        "logind Lock (Super+Escape, loginctl, lid=lock)",
        Some(Logind),
        5,
        100,
        NO_SLEEP,
        Ask,
    ),
    ("lid close or suspend", Some(Sleep), 5, 100, NO_SLEEP, Ask),
    (
        "re-lock after a crash",
        Some(Restored),
        5,
        100,
        NO_SLEEP,
        Ask,
    ),
    ("cause unknown", None, 5, 100, NO_SLEEP, Ask),
];

#[test]
fn the_grace_opens_only_a_quick_wake_from_an_idle_screen_off_lock() {
    for &(name, cause, grace, woke, slept, want) in CASES {
        let case = GraceCase {
            cause,
            grace: LockGrace(IdleSecs(grace)),
            woke: WakeDelay(woke),
            slept,
        };
        assert_eq!(grace_verdict(&case), want, "{name}");
        assert_eq!(
            ticket(&case).is_some(),
            want == NoPassword,
            "{name}: ticket"
        );
    }
}
