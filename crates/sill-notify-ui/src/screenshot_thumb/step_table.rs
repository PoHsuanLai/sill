//! The thumbnail machine's table.

use sill_model::screenshots::{ShotId, ShotLeave};

use super::{
    DragEnd, HoldToken, KeyFocus, Pointer, ThumbExit, ThumbIn, ThumbOut, ThumbParams, ThumbState,
    step,
};

const P: ThumbParams = ThumbParams {
    hold_ms: 5200,
    rest_ms: 1500,
};
const ONE: ShotId = ShotId(1);
const TWO: ShotId = ShotId(2);

fn shown(id: ShotId, pointer: Pointer, token: u32) -> ThumbState {
    held(id, pointer, KeyFocus::Away, token)
}

fn held(id: ShotId, pointer: Pointer, keys: KeyFocus, token: u32) -> ThumbState {
    ThumbState::Shown {
        id,
        pointer,
        keys,
        token: HoldToken(token),
    }
}

fn leaving(id: ShotId, token: u32, exit: ThumbExit) -> ThumbState {
    ThumbState::Leaving {
        id,
        token: HoldToken(token),
        exit,
    }
}

type Case = (ThumbState, ThumbIn, ThumbState, Vec<ThumbOut>);

fn check(cases: Vec<Case>) {
    for (from, input, want, outs) in cases {
        assert_eq!(step(from, input, P), (want, outs), "{from:?} + {input:?}");
    }
}

#[test]
fn the_thumbnail_follows_its_table() {
    use Pointer::{Away, Over};
    use ThumbExit::{Card, Row};
    use ThumbIn::*;
    check(vec![
        (
            ThumbState::Hidden,
            Current(Some(ONE)),
            shown(ONE, Away, 1),
            vec![ThumbOut::Show(ONE), ThumbOut::StartHold(HoldToken(1), 5200)],
        ),
        (
            ThumbState::Hidden,
            Current(None),
            ThumbState::Hidden,
            vec![],
        ),
        (ThumbState::Hidden, Click, ThumbState::Hidden, vec![]),
        (
            shown(ONE, Away, 1),
            Current(Some(ONE)),
            shown(ONE, Away, 1),
            vec![],
        ),
        (
            shown(ONE, Away, 1),
            Current(Some(TWO)),
            shown(TWO, Away, 2),
            vec![ThumbOut::Show(TWO), ThumbOut::StartHold(HoldToken(2), 5200)],
        ),
        (
            shown(ONE, Away, 1),
            PointerEnter,
            shown(ONE, Over, 2),
            vec![],
        ),
        (
            shown(ONE, Over, 2),
            HoldElapsed(HoldToken(1)),
            shown(ONE, Over, 2),
            vec![],
        ),
        (
            shown(ONE, Over, 2),
            PointerLeave,
            shown(ONE, Away, 3),
            vec![ThumbOut::StartHold(HoldToken(3), 1500)],
        ),
        (
            shown(ONE, Away, 3),
            HoldElapsed(HoldToken(3)),
            leaving(ONE, 3, Card),
            vec![ThumbOut::Leave(ONE, ShotLeave::Kept)],
        ),
        (
            shown(ONE, Over, 2),
            Click,
            leaving(ONE, 2, Card),
            vec![ThumbOut::Leave(ONE, ShotLeave::Opened)],
        ),
        (
            shown(ONE, Over, 2),
            SwipedAway,
            leaving(ONE, 2, Row),
            vec![ThumbOut::Leave(ONE, ShotLeave::Kept)],
        ),
        (
            shown(ONE, Over, 2),
            TrashPressed,
            leaving(ONE, 2, Card),
            vec![ThumbOut::Leave(ONE, ShotLeave::Trashed)],
        ),
        (
            shown(ONE, Away, 1),
            Current(None),
            leaving(ONE, 1, Card),
            vec![],
        ),
        (
            leaving(ONE, 2, Row),
            ExitSettled,
            ThumbState::Hidden,
            vec![ThumbOut::Hide],
        ),
        (leaving(ONE, 2, Card), Click, leaving(ONE, 2, Card), vec![]),
        (
            leaving(ONE, 2, Card),
            Current(Some(TWO)),
            shown(TWO, Away, 3),
            vec![ThumbOut::Show(TWO), ThumbOut::StartHold(HoldToken(3), 5200)],
        ),
    ]);
}

#[test]
fn a_drag_out_pauses_the_hold_until_its_outcome() {
    use Pointer::{Away, Dragging, Over};
    use ThumbExit::Card;
    use ThumbIn::*;
    check(vec![
        (
            shown(ONE, Over, 2),
            DragStarted(ONE),
            shown(ONE, Dragging, 3),
            vec![],
        ),
        (
            shown(ONE, Over, 2),
            DragStarted(TWO),
            shown(ONE, Over, 2),
            vec![],
        ),
        (
            shown(ONE, Dragging, 3),
            PointerLeave,
            shown(ONE, Dragging, 3),
            vec![],
        ),
        (
            shown(ONE, Dragging, 3),
            HoldElapsed(HoldToken(3)),
            shown(ONE, Dragging, 3),
            vec![],
        ),
        (
            shown(ONE, Dragging, 3),
            Click,
            shown(ONE, Dragging, 3),
            vec![],
        ),
        (
            shown(ONE, Dragging, 3),
            DragEnded(ONE, DragEnd::Copied),
            leaving(ONE, 3, Card),
            vec![ThumbOut::Leave(ONE, ShotLeave::Dragged)],
        ),
        (
            shown(ONE, Dragging, 3),
            DragEnded(ONE, DragEnd::Moved),
            leaving(ONE, 3, Card),
            vec![ThumbOut::Leave(ONE, ShotLeave::Moved)],
        ),
        (
            shown(ONE, Dragging, 3),
            DragEnded(ONE, DragEnd::Cancelled),
            shown(ONE, Away, 4),
            vec![ThumbOut::StartHold(HoldToken(4), 1500)],
        ),
        (
            shown(ONE, Dragging, 3),
            DragEnded(ONE, DragEnd::Refused),
            shown(ONE, Away, 4),
            vec![ThumbOut::StartHold(HoldToken(4), 1500)],
        ),
        (
            shown(TWO, Dragging, 4),
            DragEnded(ONE, DragEnd::Copied),
            shown(TWO, Away, 5),
            vec![ThumbOut::StartHold(HoldToken(5), 1500)],
        ),
        (
            shown(ONE, Away, 1),
            DragEnded(ONE, DragEnd::Copied),
            shown(ONE, Away, 1),
            vec![],
        ),
    ]);
}

#[test]
fn the_keyboard_is_asked_for_held_and_given_back_on_every_way_out() {
    use KeyFocus::{Away as Free, Held};
    use Pointer::{Away, Over};
    use ThumbExit::Card;
    use ThumbIn::*;
    use ThumbOut::{ReleaseKeyboard, TakeKeyboard};
    let kept = || ThumbOut::Leave(ONE, ShotLeave::Kept);
    check(vec![
        (
            shown(ONE, Away, 1),
            FocusAsked,
            held(ONE, Away, Held, 2),
            vec![TakeKeyboard],
        ),
        (
            held(ONE, Over, Free, 2),
            FocusAsked,
            held(ONE, Over, Held, 3),
            vec![TakeKeyboard],
        ),
        // The hold is paused: the timer that was running is stale.
        (
            held(ONE, Away, Held, 2),
            HoldElapsed(HoldToken(1)),
            held(ONE, Away, Held, 2),
            vec![],
        ),
        (
            held(ONE, Away, Held, 2),
            HoldElapsed(HoldToken(2)),
            leaving(ONE, 2, Card),
            vec![ReleaseKeyboard, kept()],
        ),
        // The pointer leaving does not restart the hold while the keyboard is held.
        (
            held(ONE, Over, Held, 3),
            PointerLeave,
            held(ONE, Away, Held, 4),
            vec![],
        ),
        (
            held(ONE, Away, Held, 2),
            FocusLost,
            shown(ONE, Away, 3),
            vec![ReleaseKeyboard, ThumbOut::StartHold(HoldToken(3), 1500)],
        ),
        (
            held(ONE, Over, Held, 2),
            FocusLost,
            shown(ONE, Over, 3),
            vec![ReleaseKeyboard],
        ),
        (shown(ONE, Away, 1), FocusLost, shown(ONE, Away, 1), vec![]),
        (ThumbState::Hidden, FocusAsked, ThumbState::Hidden, vec![]),
        (
            leaving(ONE, 2, Card),
            FocusAsked,
            leaving(ONE, 2, Card),
            vec![],
        ),
        (
            held(ONE, Away, Held, 2),
            Click,
            leaving(ONE, 2, Card),
            vec![ReleaseKeyboard, ThumbOut::Leave(ONE, ShotLeave::Opened)],
        ),
        (
            held(ONE, Away, Held, 2),
            TrashPressed,
            leaving(ONE, 2, Card),
            vec![ReleaseKeyboard, ThumbOut::Leave(ONE, ShotLeave::Trashed)],
        ),
        (
            held(ONE, Away, Held, 2),
            Dismissed,
            leaving(ONE, 2, Card),
            vec![ReleaseKeyboard, kept()],
        ),
        (
            shown(ONE, Over, 2),
            Dismissed,
            leaving(ONE, 2, Card),
            vec![kept()],
        ),
        (
            held(ONE, Away, Held, 2),
            Current(None),
            leaving(ONE, 2, Card),
            vec![ReleaseKeyboard],
        ),
        // A newer shot takes the card and the keyboard goes back: it never follows a shot.
        (
            held(ONE, Away, Held, 2),
            Current(Some(TWO)),
            shown(TWO, Away, 3),
            vec![
                ReleaseKeyboard,
                ThumbOut::Show(TWO),
                ThumbOut::StartHold(HoldToken(3), 5200),
            ],
        ),
    ]);
}
