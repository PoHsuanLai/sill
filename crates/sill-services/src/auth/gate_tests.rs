use super::{Checks, Gate, GateIn, GateOut, step};
use crate::auth::types::{Attempts, AuthFault, Outcome};

/// One case: a name, the start, the inputs, the end, and everything said on the way.
type Case = (&'static str, Checks, Vec<GateIn>, Checks, Vec<GateOut>);

fn run(start: Checks, inputs: &[GateIn]) -> (Checks, Vec<GateOut>) {
    inputs
        .iter()
        .fold((start, Vec::new()), |(s, mut all), input| {
            let (s, out) = step(s, input.clone());
            all.extend(out);
            (s, all)
        })
}

fn wrong() -> [GateIn; 2] {
    [GateIn::Submit, GateIn::Verdict(Outcome::Refused)]
}

fn at(gate: Gate, failures: u32) -> Checks {
    Checks {
        gate,
        failures: Attempts(failures),
    }
}

#[test]
fn the_rules_in_the_module_doc() {
    let open = Checks::default();
    let twenty_wrong: Vec<GateIn> = (0..20).flat_map(|_| wrong()).collect();
    let twenty_said: Vec<GateOut> = (0..20)
        .flat_map(|_| [GateOut::Check, GateOut::Wrong])
        .collect();
    let cases: Vec<Case> = vec![
        (
            "a submit checks",
            open,
            vec![GateIn::Submit],
            at(Gate::Checking, 0),
            vec![GateOut::Check],
        ),
        (
            "a second submit while checking is busy",
            open,
            vec![GateIn::Submit, GateIn::Submit],
            at(Gate::Checking, 0),
            vec![GateOut::Check, GateOut::Busy],
        ),
        (
            "right after two wrong resets the count",
            open,
            [
                wrong().to_vec(),
                wrong().to_vec(),
                vec![GateIn::Submit, GateIn::Verdict(Outcome::Accepted)],
            ]
            .concat(),
            Checks::default(),
            vec![
                GateOut::Check,
                GateOut::Wrong,
                GateOut::Check,
                GateOut::Wrong,
                GateOut::Check,
                GateOut::Accepted,
            ],
        ),
        (
            "twenty wrong never wait: the gate reopens after each (no lockout, F862)",
            open,
            twenty_wrong,
            at(Gate::Open, 20),
            twenty_said,
        ),
        (
            "the twenty-first submit is checked at once",
            at(Gate::Open, 20),
            vec![GateIn::Submit],
            at(Gate::Checking, 20),
            vec![GateOut::Check],
        ),
        (
            "unavailable reopens and counts nothing",
            open,
            vec![
                GateIn::Submit,
                GateIn::Verdict(Outcome::Unavailable(AuthFault::TimedOut)),
            ],
            Checks::default(),
            vec![GateOut::Check, GateOut::Unavailable],
        ),
        (
            "reset reopens a check and keeps the count",
            at(Gate::Checking, 3),
            vec![GateIn::Reset],
            at(Gate::Open, 3),
            vec![],
        ),
        (
            "a stale verdict changes nothing",
            open,
            vec![GateIn::Verdict(Outcome::Accepted)],
            open,
            vec![],
        ),
    ];
    for (name, start, inputs, want_state, want_out) in cases {
        assert_eq!(run(start, &inputs), (want_state, want_out), "{name}");
    }
}
