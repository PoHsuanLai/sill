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

/// What a key asks of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CenterOut {
    Close,
    Activate(usize),
    Dismiss(usize),
}

/// `focus` (the drawn row with the focus ring, if any) after `key` over `rows` drawn rows, and
/// what the panel must do. Down from none focuses the first row, Up from none the last; a focus
/// past the rows (they shrank) is clamped first; with no rows only Escape does anything.
/// Dismissing keeps the ring on the same index, clamped to the rows that remain.
pub fn step_keys(
    focus: Option<usize>,
    key: CenterKey,
    rows: usize,
) -> (Option<usize>, Vec<CenterOut>) {
    let last = rows.checked_sub(1);
    let focus = focus.zip(last).map(|(at, last)| at.min(last));
    match (key, focus, last) {
        (CenterKey::Escape, _, _) => (focus, vec![CenterOut::Close]),
        (_, _, None) => (None, Vec::new()),
        (CenterKey::Down, None, Some(_)) => (Some(0), Vec::new()),
        (CenterKey::Up, None, Some(last)) => (Some(last), Vec::new()),
        (CenterKey::Down, Some(at), Some(last)) => (Some((at + 1).min(last)), Vec::new()),
        (CenterKey::Up, Some(at), Some(_)) => (Some(at.saturating_sub(1)), Vec::new()),
        (CenterKey::Return, Some(at), Some(_)) => (focus, vec![CenterOut::Activate(at)]),
        (CenterKey::Delete, Some(at), Some(last)) => {
            let after = last.checked_sub(1).map(|left| at.min(left));
            (after, vec![CenterOut::Dismiss(at)])
        }
        (CenterKey::Return | CenterKey::Delete, None, Some(_)) => (None, Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::{CenterKey, CenterOut, step_keys};

    #[test]
    fn keys_move_the_ring_act_and_close() {
        use CenterKey::{Delete, Down, Escape, Return, Up};
        let cases = [
            (None, Escape, 3, None, vec![CenterOut::Close]),
            (Some(1), Escape, 3, Some(1), vec![CenterOut::Close]),
            (None, Down, 3, Some(0), vec![]),
            (None, Up, 3, Some(2), vec![]),
            (Some(0), Up, 3, Some(0), vec![]),
            (Some(2), Down, 3, Some(2), vec![]),
            (Some(1), Down, 3, Some(2), vec![]),
            (Some(1), Return, 3, Some(1), vec![CenterOut::Activate(1)]),
            (Some(2), Delete, 3, Some(1), vec![CenterOut::Dismiss(2)]),
            (Some(0), Delete, 1, None, vec![CenterOut::Dismiss(0)]),
            (Some(5), Down, 2, Some(1), vec![]),
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
