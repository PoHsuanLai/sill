use super::{Gate, Lockout, LockoutIn, LockoutOut, LockoutPolicy, step, wait_after};
use crate::auth::types::{Attempts, AuthFault, Outcome};
use crate::units::Stamp;

const POLICY: LockoutPolicy = LockoutPolicy {
    free_attempts: 2,
    first_wait_ms: 1_000,
    max_wait_ms: 3_000,
};

/// One case: a name, the start, the inputs, the end, and everything said on the way.
type Case = (
    &'static str,
    Lockout,
    Vec<LockoutIn>,
    Lockout,
    Vec<LockoutOut>,
);

fn run(start: Lockout, inputs: &[LockoutIn]) -> (Lockout, Vec<LockoutOut>) {
    inputs
        .iter()
        .fold((start, Vec::new()), |(s, mut all), input| {
            let (s, out) = step(s, input.clone(), &POLICY);
            all.extend(out);
            (s, all)
        })
}

fn wrong(at: u64) -> [LockoutIn; 2] {
    [
        LockoutIn::Submit(Stamp(at)),
        LockoutIn::Verdict(Outcome::Refused, Stamp(at + 10)),
    ]
}

#[test]
fn waits_grow_and_cap() {
    let cases = [
        (0, None),
        (1, None),
        (2, None),
        (3, Some(1_000)),
        (4, Some(2_000)),
        (5, Some(3_000)),
        (9, Some(3_000)),
    ];
    for (failures, want) in cases {
        assert_eq!(wait_after(Attempts(failures), &POLICY), want, "{failures}");
    }
    assert_eq!(
        wait_after(Attempts(6), &LockoutPolicy::DEFAULT),
        Some(60_000)
    );
}

#[test]
fn the_rules_in_the_module_doc() {
    let open = Lockout::default();
    let three_wrong: Vec<LockoutIn> = [wrong(0), wrong(100), wrong(200)].concat();
    let cases: Vec<Case> = vec![
        (
            "a submit checks",
            open,
            vec![LockoutIn::Submit(Stamp(0))],
            Lockout {
                gate: Gate::Checking,
                failures: Attempts(0),
            },
            vec![LockoutOut::Check],
        ),
        (
            "a second submit while checking is busy",
            open,
            vec![LockoutIn::Submit(Stamp(0)), LockoutIn::Submit(Stamp(5))],
            Lockout {
                gate: Gate::Checking,
                failures: Attempts(0),
            },
            vec![LockoutOut::Check, LockoutOut::Busy],
        ),
        (
            "right after two wrong resets the count",
            open,
            [
                wrong(0).to_vec(),
                wrong(100).to_vec(),
                vec![
                    LockoutIn::Submit(Stamp(200)),
                    LockoutIn::Verdict(Outcome::Accepted, Stamp(210)),
                ],
            ]
            .concat(),
            Lockout::default(),
            vec![
                LockoutOut::Check,
                LockoutOut::Wrong,
                LockoutOut::Check,
                LockoutOut::Wrong,
                LockoutOut::Check,
                LockoutOut::Accepted,
            ],
        ),
        (
            "the third wrong waits a second",
            open,
            three_wrong.clone(),
            Lockout {
                gate: Gate::Waiting {
                    until: Stamp(1_210),
                },
                failures: Attempts(3),
            },
            vec![
                LockoutOut::Check,
                LockoutOut::Wrong,
                LockoutOut::Check,
                LockoutOut::Wrong,
                LockoutOut::Check,
                LockoutOut::Wrong,
                LockoutOut::LockedOut {
                    until: Stamp(1_210),
                },
            ],
        ),
        (
            "a submit while waiting is busy; the tick reopens",
            Lockout {
                gate: Gate::Waiting {
                    until: Stamp(1_000),
                },
                failures: Attempts(3),
            },
            vec![
                LockoutIn::Submit(Stamp(500)),
                LockoutIn::Tick(Stamp(999)),
                LockoutIn::Tick(Stamp(1_000)),
            ],
            Lockout {
                gate: Gate::Open,
                failures: Attempts(3),
            },
            vec![LockoutOut::Busy, LockoutOut::Reopened],
        ),
        (
            "unavailable reopens and counts nothing",
            open,
            vec![
                LockoutIn::Submit(Stamp(0)),
                LockoutIn::Verdict(Outcome::Unavailable(AuthFault::TimedOut), Stamp(10)),
            ],
            Lockout::default(),
            vec![LockoutOut::Check, LockoutOut::Unavailable],
        ),
        (
            "reset keeps a wait and the count",
            Lockout {
                gate: Gate::Waiting {
                    until: Stamp(1_000),
                },
                failures: Attempts(3),
            },
            vec![LockoutIn::Reset],
            Lockout {
                gate: Gate::Waiting {
                    until: Stamp(1_000),
                },
                failures: Attempts(3),
            },
            vec![],
        ),
        (
            "a stale verdict changes nothing",
            open,
            vec![LockoutIn::Verdict(Outcome::Accepted, Stamp(0))],
            open,
            vec![],
        ),
    ];
    for (name, start, inputs, want_state, want_out) in cases {
        assert_eq!(run(start, &inputs), (want_state, want_out), "{name}");
    }
}
