use super::super::*;
use super::ask;

fn asking(id: u64) -> ColorAsking {
    ColorAsking {
        id: AskId(id),
        ask: ColorAsk {
            app_label: "Screenshot".to_owned(),
        },
    }
}

fn with_eyedropper() -> CapturePicker {
    let (state, _) = step(CapturePicker::default(), CaptureEvent::Eyedropper(true));
    state
}

const PICKED: Rgb8 = Rgb8 {
    red: 255,
    green: 0,
    blue: 51,
};

#[test]
fn the_reply_is_the_portals_array_of_one_colour_or_empty() {
    // `(case, the reply, the portal's array of (r, g, b))`.
    type Case = (&'static str, Option<Rgb8>, Vec<(f64, f64, f64)>);
    let cases: [Case; 2] = [
        ("cancelled", None, vec![]),
        ("a colour in 0..1", Some(PICKED), vec![(1.0, 0.0, 0.2)]),
    ];
    for (name, reply, want) in cases {
        assert_eq!(color_wire(reply), want, "{name}");
    }
}

#[test]
fn without_an_eyedropper_the_request_is_cancelled_at_once_and_nothing_opens() {
    let before = CapturePicker::default();
    let (state, out) = step(before.clone(), CaptureEvent::ColorOpened(asking(1)));
    assert_eq!(state, before);
    assert_eq!(out, vec![CaptureOut::ColorReply(AskId(1), None)]);
}

#[test]
fn one_question_at_a_time_of_either_kind() {
    let (state, out) = step(with_eyedropper(), CaptureEvent::ColorOpened(asking(1)));
    assert!(out.is_empty());
    assert!(!admits(&state));
    let (again, out) = step(state.clone(), CaptureEvent::ColorOpened(asking(2)));
    assert_eq!((again, out), (state.clone(), vec![]));
    let sources = Asking {
        id: AskId(3),
        ask: ask(3, 0, &[1]),
    };
    let (again, out) = step(state.clone(), CaptureEvent::Opened(sources.clone()));
    assert_eq!((again, out), (state, vec![]), "a colour is open");

    let (sharing, _) = step(with_eyedropper(), CaptureEvent::Opened(sources));
    let (still, out) = step(sharing.clone(), CaptureEvent::ColorOpened(asking(4)));
    assert_eq!(
        (still, out),
        (sharing, vec![]),
        "a sharing question is open"
    );
}

#[test]
fn a_click_answers_with_the_colour_and_frees_the_eyedropper() {
    let (state, _) = step(with_eyedropper(), CaptureEvent::ColorOpened(asking(7)));
    let (same, out) = step(state.clone(), CaptureEvent::ColorAnswered(AskId(6), None));
    assert_eq!((same, out), (state.clone(), vec![]), "a stale answer");
    let (state, out) = step(state, CaptureEvent::ColorAnswered(AskId(7), Some(PICKED)));
    assert!(admits(&state));
    assert_eq!(out, vec![CaptureOut::ColorReply(AskId(7), Some(PICKED))]);
}

#[test]
fn the_caller_leaving_closes_the_eyedropper_without_a_reply() {
    let (state, _) = step(with_eyedropper(), CaptureEvent::ColorOpened(asking(1)));
    let (state, out) = step(state, CaptureEvent::CallerGone);
    assert!(state.color.is_none());
    assert!(out.is_empty());
}
