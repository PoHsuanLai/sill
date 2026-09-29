//! The thumbnail machine's table.

use sill_model::screenshots::{ShotId, ShotLeave};

use super::{
    DragEnd, HoldToken, Pointer, ThumbExit, ThumbIn, ThumbOut, ThumbParams, ThumbState, step,
};

const P: ThumbParams = ThumbParams {
    hold_ms: 5200,
    rest_ms: 1500,
};
const ONE: ShotId = ShotId(1);
const TWO: ShotId = ShotId(2);

fn shown(id: ShotId, pointer: Pointer, token: u32) -> ThumbState {
    ThumbState::Shown {
        id,
        pointer,
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
