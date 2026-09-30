//! Which keys the preview pane's machine claims from the palette's field. Pure.
//!
//! Space while browsing, → (the caret decides what it does), and, with the pane holding the
//! keyboard, ←, Tab and Shift+Tab; Escape while the pane shows or has the keyboard. On a cell of
//! the emoji grid ← and → are the grid's (design/13 §13.3.9: "2D arrow keys"): they pass to the
//! palette, which moves the cell, and the pane is ⌘Y's, ⌘→'s and Space's there, as in the
//! reference's character viewer.

use dioxus::prelude::{Key, Modifiers};

use crate::launcher::preview::keys::{Caret, Pane, PaneFocus, PreviewKey};
use crate::launcher::view::Layout;

/// Where a key lands: the pane, who has the keyboard, and how the selected line is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyPlace {
    pub pane: Pane,
    pub focus: PaneFocus,
    pub layout: Layout,
}

/// The pane machine's key for `pressed` with `modifiers`, the field's caret at `caret`; none
/// when the key is not the pane's (the palette and the field hear it).
pub fn pane_key(
    pressed: &Key,
    modifiers: Modifiers,
    caret: Caret,
    place: KeyPlace,
) -> Option<PreviewKey> {
    let in_pane = matches!(place.focus, PaneFocus::Pane(_));
    let shift = modifiers.contains(Modifiers::SHIFT);
    match (pressed, in_pane, place.layout) {
        (Key::Character(text), ..) if text == " " && modifiers.is_empty() => {
            Some(PreviewKey::Space)
        }
        (Key::ArrowRight | Key::ArrowLeft, false, Layout::Grid) => None,
        (Key::ArrowRight, ..) => Some(PreviewKey::Right(caret)),
        (Key::ArrowLeft, true, _) => Some(PreviewKey::Left),
        (Key::Tab, true, _) if shift => Some(PreviewKey::ShiftTab),
        (Key::Tab, true, _) => Some(PreviewKey::Tab),
        (Key::Escape, ..) if in_pane || place.pane == Pane::Shown => Some(PreviewKey::Escape),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyPlace, pane_key};
    use crate::launcher::preview::keys::{Caret, Pane, PaneFocus, PreviewKey};
    use crate::launcher::view::Layout;
    use dioxus::prelude::{Key, Modifiers};

    #[test]
    fn arrows_on_a_grid_cell_are_the_grids_and_the_rest_the_panes() {
        let place = |pane, focus, layout| KeyPlace {
            pane,
            focus,
            layout,
        };
        let list = place(Pane::Hidden, PaneFocus::Results, Layout::List);
        let grid = place(Pane::Hidden, PaneFocus::Results, Layout::Grid);
        let grid_shown = place(Pane::Shown, PaneFocus::Results, Layout::Grid);
        let grid_in_pane = place(Pane::Shown, PaneFocus::Pane(0), Layout::Grid);
        let list_in_pane = place(Pane::Shown, PaneFocus::Pane(1), Layout::List);
        let space = Key::Character(" ".to_owned());
        let none = Modifiers::empty();
        let shift = Modifiers::SHIFT;
        let (end, inside) = (Caret::AtEnd, Caret::Inside);
        let right = |caret| Some(PreviewKey::Right(caret));
        #[rustfmt::skip]
        let cases: Vec<(Key, Modifiers, Caret, KeyPlace, Option<PreviewKey>)> = vec![
            // A list row: → shows the pane (at the end) or moves the caret; ← is the field's.
            (Key::ArrowRight, none, end, list, right(end)),
            (Key::ArrowRight, none, inside, list, right(inside)),
            (Key::ArrowLeft, none, end, list, None),
            // A grid cell: ← and → move the cell, the pane shown or not.
            (Key::ArrowRight, none, end, grid, None),
            (Key::ArrowLeft, none, end, grid, None),
            (Key::ArrowRight, none, end, grid_shown, None),
            (Key::ArrowLeft, none, end, grid_shown, None),
            // In the pane, the pane's arrows, whatever row it shows.
            (Key::ArrowLeft, none, end, grid_in_pane, Some(PreviewKey::Left)),
            (Key::ArrowRight, none, end, grid_in_pane, right(end)),
            (Key::ArrowLeft, none, end, list_in_pane, Some(PreviewKey::Left)),
            // Space, Tab and Escape as before.
            (space.clone(), none, end, grid, Some(PreviewKey::Space)),
            (space, shift, end, grid, None),
            (Key::Tab, none, end, grid, None),
            (Key::Tab, none, end, list_in_pane, Some(PreviewKey::Tab)),
            (Key::Tab, shift, end, list_in_pane, Some(PreviewKey::ShiftTab)),
            (Key::Escape, none, end, list, None),
            (Key::Escape, none, end, grid_shown, Some(PreviewKey::Escape)),
            (Key::Escape, none, end, grid_in_pane, Some(PreviewKey::Escape)),
            (Key::ArrowDown, none, end, grid, None),
        ];
        for (key, modifiers, caret, at, want) in cases {
            assert_eq!(
                pane_key(&key, modifiers, caret, at),
                want,
                "{key:?} {modifiers:?} {caret:?} {at:?}"
            );
        }
    }
}
