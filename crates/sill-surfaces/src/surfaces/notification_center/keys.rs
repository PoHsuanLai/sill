//! The center's keys (design/20 §1.6 keyboard `OnDemand`; design/06 Escape): Escape closes,
//! Up and Down move a focus ring over the drawn rows (not wrapping), Return activates the
//! focused row (its `default` action, or opens a collapsed group's "N more"), Delete dismisses
//! it.

/// A key the center handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CenterKey {
    Escape,
    Up,
    Down,
    Return,
    Delete,
}

/// Which drawn row has the focus ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RowFocus {
    #[default]
    None,
    Row(usize),
}

/// What a key asks of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CenterOut {
    Close,
    Activate(usize),
    Dismiss(usize),
}

/// `focus` after `key` over `rows` drawn rows, and what the panel must do. Down from none
/// focuses the first row, Up from none the last; a focus past the rows (they shrank) is
/// clamped first; with no rows only Escape does anything. Dismissing keeps the ring on the same
/// index, clamped to the rows that remain.
pub fn step_keys(focus: RowFocus, key: CenterKey, rows: usize) -> (RowFocus, Vec<CenterOut>) {
    let last = rows.checked_sub(1);
    let focus = clamp(focus, last);
    match (key, focus, last) {
        (CenterKey::Escape, _, _) => (focus, vec![CenterOut::Close]),
        (_, _, None) => (RowFocus::None, Vec::new()),
        (CenterKey::Down, RowFocus::None, Some(_)) => (RowFocus::Row(0), Vec::new()),
        (CenterKey::Up, RowFocus::None, Some(last)) => (RowFocus::Row(last), Vec::new()),
        (CenterKey::Down, RowFocus::Row(at), Some(last)) => {
            (RowFocus::Row((at + 1).min(last)), Vec::new())
        }
        (CenterKey::Up, RowFocus::Row(at), Some(_)) => {
            (RowFocus::Row(at.saturating_sub(1)), Vec::new())
        }
        (CenterKey::Return, RowFocus::Row(at), Some(_)) => (focus, vec![CenterOut::Activate(at)]),
        (CenterKey::Delete, RowFocus::Row(at), Some(last)) => {
            let after = last.checked_sub(1).map(|left| RowFocus::Row(at.min(left)));
            (
                after.unwrap_or(RowFocus::None),
                vec![CenterOut::Dismiss(at)],
            )
        }
        (CenterKey::Return | CenterKey::Delete, RowFocus::None, Some(_)) => {
            (RowFocus::None, Vec::new())
        }
    }
}

/// `focus` kept inside rows `0..=last` (none when there are no rows).
fn clamp(focus: RowFocus, last: Option<usize>) -> RowFocus {
    match (focus, last) {
        (RowFocus::None, _) | (_, None) => RowFocus::None,
        (RowFocus::Row(at), Some(last)) => RowFocus::Row(at.min(last)),
    }
}

#[cfg(test)]
mod tests {
    use super::{CenterKey, CenterOut, RowFocus, step_keys};

    #[test]
    fn keys_move_the_ring_act_and_close() {
        use CenterKey::{Delete, Down, Escape, Return, Up};
        use RowFocus::{None, Row};
        let cases = [
            (None, Escape, 3, None, vec![CenterOut::Close]),
            (Row(1), Escape, 3, Row(1), vec![CenterOut::Close]),
            (None, Down, 3, Row(0), vec![]),
            (None, Up, 3, Row(2), vec![]),
            (Row(0), Up, 3, Row(0), vec![]),
            (Row(2), Down, 3, Row(2), vec![]),
            (Row(1), Down, 3, Row(2), vec![]),
            (Row(1), Return, 3, Row(1), vec![CenterOut::Activate(1)]),
            (Row(2), Delete, 3, Row(1), vec![CenterOut::Dismiss(2)]),
            (Row(0), Delete, 1, None, vec![CenterOut::Dismiss(0)]),
            (Row(5), Down, 2, Row(1), vec![]),
            (None, Return, 3, None, vec![]),
            (None, Down, 0, None, vec![]),
        ];
        for (focus, key, rows, want_focus, want_out) in cases {
            assert_eq!(
                step_keys(focus, key, rows),
                (want_focus, want_out),
                "{focus:?} {key:?} over {rows}"
            );
        }
    }
}
