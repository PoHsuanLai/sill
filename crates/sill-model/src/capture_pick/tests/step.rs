use super::super::*;
use super::{ask, candidates, window};

fn open(id: u64, kinds: u32) -> Asking {
    Asking {
        id: AskId(id),
        ask: ask(kinds, 0, &[1]),
    }
}

fn serving() -> CapturePicker {
    let (state, _) = step(
        CapturePicker::default(),
        CaptureEvent::Link(CaptureLink::Serving),
    );
    let (state, _) = step(state, CaptureEvent::Candidates(candidates()));
    state
}

#[test]
fn a_second_request_is_refused_while_one_is_open() {
    let (state, _) = step(serving(), CaptureEvent::Opened(open(1, 3)));
    assert!(!admits(&state));
    let (again, out) = step(state.clone(), CaptureEvent::Opened(open(2, 1)));
    assert_eq!(again, state);
    assert!(out.is_empty());
}

#[test]
fn allow_answers_with_the_pick_and_frees_the_sheet() {
    let (state, _) = step(serving(), CaptureEvent::Opened(open(1, 2)));
    let plan = plan(&state.asking.as_ref().expect("open").ask, &state.candidates); // test-only
    let picks = toggle(&plan, Plurality::One, Picks::start(&plan), window("w-2"));
    let (state, out) = step(state, CaptureEvent::Answered(AskId(1), Some(picks)));
    assert!(admits(&state));
    let [CaptureOut::Reply(id, reply)] = out.as_slice() else {
        panic!("{out:?}")
    };
    assert_eq!(*id, AskId(1));
    assert_eq!(reply.sources, vec![("window".to_owned(), "w-2".to_owned())]);
}

#[test]
fn cancel_answers_empty_and_a_stale_answer_is_ignored() {
    let (state, _) = step(serving(), CaptureEvent::Opened(open(5, 3)));
    let (same, out) = step(state.clone(), CaptureEvent::Answered(AskId(4), None));
    assert_eq!(same, state);
    assert!(out.is_empty());
    let (state, out) = step(state, CaptureEvent::Answered(AskId(5), None));
    assert_eq!(out, vec![CaptureOut::Reply(AskId(5), Reply::cancelled())]);
    assert!(state.asking.is_none());
}

#[test]
fn the_caller_leaving_closes_the_sheet_without_a_reply() {
    let (state, _) = step(serving(), CaptureEvent::Opened(open(1, 3)));
    let (state, out) = step(state, CaptureEvent::CallerGone);
    assert!(state.asking.is_none());
    assert!(out.is_empty());
}
