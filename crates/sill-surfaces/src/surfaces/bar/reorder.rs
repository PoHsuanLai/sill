//! Workspace drag reorder (design/20 §1.1 "workspace indicator (drag reorder)"): a press on
//! one pill and a release on another, far enough away, moves the pressed workspace before the
//! other when dragged left and after it when dragged right. Pure.

use sill_services::{Relative, WlCommand, WorkspaceKey};

/// A press on a workspace pill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pressed {
    pub key: WorkspaceKey,
    pub index: usize,
    /// The pointer's x in the bar, logical pixels.
    pub x: i32,
}

/// A pill a press was released over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Dropped {
    pub key: WorkspaceKey,
    pub index: usize,
    pub x: i32,
}

/// The reorder `pressed` then `dropped` asks for: none for a release on the same pill or one
/// closer than `threshold` px (the drag threshold, `dock.hold_to_menu_move_px`, FINDINGS F9).
pub(crate) fn reorder(pressed: Pressed, dropped: Dropped, threshold: u16) -> Option<WlCommand> {
    let moved = pressed.x.abs_diff(dropped.x);
    if dropped.index == pressed.index || moved < u32::from(threshold) {
        return None;
    }
    let relative = if dropped.index < pressed.index {
        Relative::Before
    } else {
        Relative::After
    };
    Some(WlCommand::MoveWorkspace {
        workspace: pressed.key,
        relative,
        other: dropped.key,
    })
}

#[cfg(test)]
mod tests {
    use super::{Dropped, Pressed, reorder};
    use sill_services::{Relative, WlCommand, WorkspaceKey};

    #[test]
    fn a_drop_moves_the_pressed_workspace_beside_the_other() {
        let pressed = Pressed {
            key: WorkspaceKey(11),
            index: 1,
            x: 100,
        };
        let drop = |index: usize, x| Dropped {
            key: WorkspaceKey(10 + index as u64),
            index,
            x,
        };
        let moved = |relative, other| {
            Some(WlCommand::MoveWorkspace {
                workspace: WorkspaceKey(11),
                relative,
                other: WorkspaceKey(other),
            })
        };
        let cases = [
            (
                "left onto the first",
                drop(0, 70),
                moved(Relative::Before, 10),
            ),
            (
                "right onto the third",
                drop(2, 130),
                moved(Relative::After, 12),
            ),
            ("back on itself", drop(1, 140), None),
            ("under the threshold is a click", drop(2, 105), None),
        ];
        for (name, dropped, want) in cases {
            assert_eq!(reorder(pressed, dropped, 8), want, "{name}");
        }
    }
}
