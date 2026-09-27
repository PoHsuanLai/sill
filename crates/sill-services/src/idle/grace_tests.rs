use super::{GraceCase, GraceVerdict, SleepSince, WakeDelay, grace_verdict, ticket};
use crate::idle::policy::{IdleSecs, LockGrace};
use crate::locker::{LockCause, LockDrawer};

use GraceVerdict::{Ask, NoPassword};
use LockCause::{Action, Adopted, IdleScreenOff, Logind, Restored, Sleep};
use LockDrawer::{Program, Shell};
use SleepSince::{NoSleep, Slept};

/// One row: name, cause, drawer, grace (s), wake after (ms), a sleep since, the verdict.
type Row = (
    &'static str,
    Option<LockCause>,
    LockDrawer,
    u32,
    u64,
    SleepSince,
    GraceVerdict,
);

/// F908's table: every lock cause, the grace, the wake delay and a sleep in between.
const CASES: &[Row] = &[
    (
        "idle, woke at 4.9 s",
        Some(IdleScreenOff),
        Shell,
        5,
        4_900,
        NoSleep,
        NoPassword,
    ),
    (
        "idle, woke at once",
        Some(IdleScreenOff),
        Shell,
        5,
        0,
        NoSleep,
        NoPassword,
    ),
    (
        "idle, woke at exactly 5 s",
        Some(IdleScreenOff),
        Shell,
        5,
        5_000,
        NoSleep,
        Ask,
    ),
    (
        "idle, woke at 30 s",
        Some(IdleScreenOff),
        Shell,
        5,
        30_000,
        NoSleep,
        Ask,
    ),
    (
        "idle, grace 0 always asks",
        Some(IdleScreenOff),
        Shell,
        0,
        100,
        NoSleep,
        Ask,
    ),
    (
        "idle, grace 60, woke at 59 s",
        Some(IdleScreenOff),
        Shell,
        60,
        59_000,
        NoSleep,
        NoPassword,
    ),
    (
        "idle, then a sleep and resume",
        Some(IdleScreenOff),
        Shell,
        5,
        1_000,
        Slept,
        Ask,
    ),
    (
        "idle, a borrowed locker",
        Some(IdleScreenOff),
        Program,
        5,
        100,
        NoSleep,
        Ask,
    ),
    (
        "sill's Lock (⌃⌘Q, launcher, power menu)",
        Some(Action),
        Shell,
        5,
        100,
        NoSleep,
        Ask,
    ),
    (
        "logind Lock (Super+Escape, loginctl, lid=lock)",
        Some(Logind),
        Shell,
        5,
        100,
        NoSleep,
        Ask,
    ),
    (
        "lid close or suspend",
        Some(Sleep),
        Shell,
        5,
        100,
        NoSleep,
        Ask,
    ),
    (
        "re-lock after a crash",
        Some(Restored),
        Shell,
        5,
        100,
        NoSleep,
        Ask,
    ),
    (
        "an adopted locker",
        Some(Adopted),
        Program,
        5,
        100,
        NoSleep,
        Ask,
    ),
    ("cause unknown", None, Shell, 5, 100, NoSleep, Ask),
];

#[test]
#[ignore = "idle fill"]
fn the_grace_opens_only_a_quick_wake_from_an_idle_screen_off_lock() {
    for &(name, cause, drawer, grace, woke, sleep, want) in CASES {
        let case = GraceCase {
            cause,
            drawer,
            grace: LockGrace(IdleSecs(grace)),
            woke: WakeDelay(woke),
            sleep,
        };
        assert_eq!(grace_verdict(&case), want, "{name}");
        assert_eq!(
            ticket(&case).is_some(),
            want == NoPassword,
            "{name}: ticket"
        );
    }
}
