//! The gate as a table: pure, on the machine's own stamps.

use ds::base::machine::Machine;
use ds::base::time::stamp::{Ms, Stamp};
use sill_model::intents::sheet::SheetChoice;

use super::gate::{AnswerGate, GateIn, GateOut, GateParams};

const PARAMS: GateParams = GateParams {
    arm_ms: Ms(500),
    hold_ms: Ms(1000),
};

const ALLOW: &[GateOut] = &[GateOut::Answer(SheetChoice::AllowOnce)];
const DENY: &[GateOut] = &[GateOut::Answer(SheetChoice::Deny)];
const NONE: &[GateOut] = &[];

/// A gate, an input at a time: the state after, what it asked for, when it wants waking.
type Case = (
    &'static str,
    AnswerGate,
    GateIn,
    u64,
    AnswerGate,
    &'static [GateOut],
    Option<Stamp>,
);

#[test]
fn the_gate_arms_then_answers_by_press_or_hold_and_escape_always_denies() {
    use AnswerGate::{Answered, Armed, Arming, Holding};
    use GateIn::{Activity, Elapsed, Escape, HoldBegan, HoldEnded, Press};
    let arming = Arming { until: Stamp(600) };
    let holding = Holding {
        since: Stamp(1_000),
        until: Stamp(2_000),
    };
    #[rustfmt::skip]
    let cases: &[Case] = &[
        ("a press while arming is lost",   arming,  Press(SheetChoice::AllowOnce),      300,   arming, NONE,      Some(Stamp(600))),
        ("a hold while arming is lost",    arming,  HoldBegan,                          300,   arming, NONE,      Some(Stamp(600))),
        ("an early wake changes nothing",  arming,  Elapsed,                            599,   arming, NONE,      Some(Stamp(600))),
        ("the wake arms it",               arming,  Elapsed,                            600,   Armed,  NONE,      None),
        ("a key while arming starts the wait again", arming, Activity,                 550,   Arming { until: Stamp(1_050) }, NONE, Some(Stamp(1_050))),
        ("the old wake no longer arms it", Arming { until: Stamp(1_050) }, Elapsed,    600,   Arming { until: Stamp(1_050) }, NONE, Some(Stamp(1_050))),
        ("activity once armed changes nothing", Armed, Activity,                       900,   Armed,  NONE,      None),
        ("activity while holding changes nothing", holding, Activity,                1_500,   holding, NONE,    Some(Stamp(2_000))),
        ("activity after an answer changes nothing", Answered, Activity,             3_000,   Answered, NONE,   None),
        ("escape denies while arming",     arming,  Escape,                             300,   Answered, DENY, None),
        ("escape denies when armed",       Armed,   Escape,                             900,   Answered, DENY, None),
        ("a press answers with its choice", Armed,  Press(SheetChoice::AllowOnce),      900,   Answered, ALLOW, None),
        ("deny is a press too",            Armed,   Press(SheetChoice::Deny),           900,   Answered, DENY, None),
        ("a hold begins",                  Armed,   HoldBegan,                          1_000, holding, NONE,     Some(Stamp(2_000))),
        ("letting go early abandons it",   holding, HoldEnded,                          1_500, Armed,  NONE,      None),
        ("an early wake keeps holding",    holding, Elapsed,                            1_999, holding, NONE,     Some(Stamp(2_000))),
        ("the hold's wake answers",        holding, Elapsed,                            2_000, Answered, ALLOW, None),
        ("a press while holding is the release's click", holding, Press(SheetChoice::AllowOnce), 1_500, holding, NONE, Some(Stamp(2_000))),
        ("escape stops a hold with a no",  holding, Escape,                             1_500, Answered, DENY, None),
        ("an answered gate is done",       Answered, Press(SheetChoice::AllowOnce),     3_000, Answered, NONE,    None),
        ("an answered gate stays done",    Answered, Escape,                            3_000, Answered, NONE,    None),
    ];
    for (name, from, input, at, state, outs, wake) in cases {
        let (next, out) = from.step(*input, Stamp(*at), &PARAMS, &());
        assert_eq!(next, *state, "{name}: state");
        assert_eq!(out.as_slice(), *outs, "{name}: outputs");
        assert_eq!(next.wake(), *wake, "{name}: wake");
    }
}

#[test]
fn a_sheet_that_appears_arms_for_the_arming_delay() {
    let gate = AnswerGate::arming(Stamp(100), &PARAMS);
    assert_eq!(gate, AnswerGate::Arming { until: Stamp(600) });
    assert_eq!(gate.wake(), Some(Stamp(600)));
}

#[test]
fn only_an_armed_or_holding_gate_takes_input() {
    for (gate, takes) in [
        (AnswerGate::Arming { until: Stamp(1) }, false),
        (AnswerGate::Armed, true),
        (
            AnswerGate::Holding {
                since: Stamp(1),
                until: Stamp(2),
            },
            true,
        ),
        (AnswerGate::Answered, false),
    ] {
        assert_eq!(gate.takes_input(), takes, "{gate:?}");
    }
}

/// A person typing in another window while the sheet appears: each key keeps pushing the arming
/// out, so a Return that lands in the typing never answers, and only after a pause as long as the
/// arming delay does the sheet take an answer.
#[test]
fn typing_keeps_the_sheet_from_arming_until_the_person_pauses() {
    let mut gate = AnswerGate::arming(Stamp(0), &PARAMS);
    for at in [100, 300, 700, 1_100] {
        let (next, out) = gate.step(GateIn::Activity, Stamp(at), &PARAMS, &());
        assert!(out.is_empty());
        let (next, out) = next.step(
            GateIn::Press(SheetChoice::AllowOnce),
            Stamp(at),
            &PARAMS,
            &(),
        );
        assert!(out.is_empty(), "a Return at {at} ms answered");
        gate = next;
    }
    let wake = gate
        .wake()
        .unwrap_or_else(|| panic!("an arming gate wakes"));
    assert_eq!(wake, Stamp(1_600), "500 ms after the last key");
    let (armed, _) = gate.step(GateIn::Elapsed, wake, &PARAMS, &());
    let (answered, out) = armed.step(GateIn::Press(SheetChoice::AllowOnce), wake, &PARAMS, &());
    assert_eq!(answered, AnswerGate::Answered);
    assert_eq!(out.as_slice(), ALLOW);
}
