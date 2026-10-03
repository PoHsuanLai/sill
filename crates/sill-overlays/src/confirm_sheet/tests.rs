//! The gate and the stage as tables: pure, on the machine's own stamps.

use ds::base::machine::Machine;
use ds::base::time::stamp::Stamp;
use sill_model::intents::ConfirmOffer;
use sill_model::intents::sheet::{SheetChoice, SheetView};

use super::gate::{AnswerGate, GateIn, GateOut, GateParams};
use super::stage::{Mapped, Stage, StageIn, advance};

const PARAMS: GateParams = GateParams {
    arm_ms: 500,
    hold_ms: 1000,
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
    use AnswerGate::{Answered, Armed, Arming, Holding, Unshown};
    use GateIn::{Elapsed, Escape, HoldBegan, HoldEnded, Press, Shown};
    let arming = Arming { until: Stamp(600) };
    let holding = Holding {
        since: Stamp(1_000),
        until: Stamp(2_000),
    };
    #[rustfmt::skip]
    let cases: &[Case] = &[
        ("shown starts the arming",        Unshown, Shown,                              100,   arming, NONE,      Some(Stamp(600))),
        ("a press while arming is lost",   arming,  Press(SheetChoice::AllowOnce),      300,   arming, NONE,      Some(Stamp(600))),
        ("a hold while arming is lost",    arming,  HoldBegan,                          300,   arming, NONE,      Some(Stamp(600))),
        ("an early wake changes nothing",  arming,  Elapsed,                            599,   arming, NONE,      Some(Stamp(600))),
        ("the wake arms it",               arming,  Elapsed,                            600,   Armed,  NONE,      None),
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
        ("shown twice does not re-arm",    Armed,   Shown,                              900,   Armed,  NONE,      None),
    ];
    for (name, from, input, at, state, outs, wake) in cases {
        let (next, out) = from.step(*input, Stamp(*at), &PARAMS);
        assert_eq!(next, *state, "{name}: state");
        assert_eq!(out.as_slice(), *outs, "{name}: outputs");
        assert_eq!(next.wake(), *wake, "{name}: wake");
    }
}

#[test]
fn only_an_armed_or_holding_gate_takes_input() {
    for (gate, takes) in [
        (AnswerGate::Unshown, false),
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

fn view(id: &str, waiting: usize) -> SheetView {
    SheetView {
        id: sill_model::intents::ConfirmId::parse(id).unwrap_or_else(|e| panic!("{e}")),
        title: "Archive a thread?".to_owned(),
        byline: String::new(),
        facts: Vec::new(),
        notes: Vec::new(),
        offer: ConfirmOffer::OnceOnly,
        gesture: sill_model::intents::Gesture::Press,
        effect: sill_model::intents::Effect::UndoableWrite,
        waiting,
    }
}

#[test]
fn the_stage_follows_the_question_and_keeps_it_while_its_sheet_leaves() {
    let up = |id: &str, waiting| Stage::Up(view(id, waiting));
    let leaving = |id: &str| Stage::Leaving(view(id, 0));
    #[rustfmt::skip]
    let cases = [
        ("a question appears",          Stage::Empty,   StageIn::Want(Some(view("a", 0))), up("a", 0),   Mapped::Yes),
        ("none while empty",            Stage::Empty,   StageIn::Want(None),               Stage::Empty, Mapped::No),
        ("the question goes",           up("a", 0),     StageIn::Want(None),               leaving("a"), Mapped::Yes),
        ("the exit settles",            leaving("a"),   StageIn::Settled,                  Stage::Empty, Mapped::No),
        ("the same question, one more waiting", up("a", 0), StageIn::Want(Some(view("a", 1))), up("a", 1), Mapped::Yes),
        ("the next question replaces",  up("a", 1),     StageIn::Want(Some(view("b", 0))), up("b", 0),   Mapped::Yes),
        ("one that arrives while leaving is up at once", leaving("a"), StageIn::Want(Some(view("b", 0))), up("b", 0), Mapped::Yes),
        ("a settle with nothing leaving is nothing", up("a", 0), StageIn::Settled,         up("a", 0),   Mapped::Yes),
    ];
    for (name, from, input, want, mapped) in cases {
        let next = advance(from, input);
        assert_eq!(next, want, "{name}");
        assert_eq!(next.mapped(), mapped, "{name}");
    }
    let (view_of, shown) = up("a", 0)
        .drawn()
        .map(|(v, s)| (v.id.clone(), s))
        .unwrap_or_else(|| panic!("drawn"));
    assert_eq!(view_of.as_str(), "a");
    assert_eq!(shown, ds::prelude::Shown::Visible);
    assert_eq!(
        leaving("a").drawn().map(|(_, s)| s),
        Some(ds::prelude::Shown::Hidden)
    );
    assert_eq!(Stage::Empty.drawn(), None);
}
