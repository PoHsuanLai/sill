//! The preview pane's keys ([`step_preview`]). Pure.
//!
//! Quick Look's Space, in a launcher whose field has the keyboard: Space toggles the pane only
//! while the person is *browsing* (the selection last moved by Up or Down, nothing typed since);
//! while *typing* it is a space in the query. ⌘Y (Quick Look's standard key) toggles it
//! whenever the selected row has a preview. → with the caret at the field's end shows the pane
//! and ⌘→ always does; either, with the pane shown, moves the keyboard into it, where Tab and
//! Shift+Tab walk its controls (quire design/27 §6.4: every control in the Tab order; in the
//! results Tab stays the provider filter, design/06 §20.2). ← or Escape in the pane gives the
//! keyboard back to the results; Escape with the pane shown hides it; only then does Escape
//! close the launcher. A row with no preview (`Preview::None`) never opens the pane.

/// Whether the pane is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Pane {
    #[default]
    Hidden,
    Shown,
}

/// Who has the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PaneFocus {
    /// The field and the results (the palette).
    #[default]
    Results,
    /// The pane's controls, at this index in its Tab order.
    Pane(usize),
}

/// What the person did last in the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FieldEdit {
    /// Typed (or just opened): Space is a character.
    #[default]
    Typing,
    /// Moved the selection with Up or Down since the last character: Space is Quick Look.
    Browsing,
}

/// Where the field's caret is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Caret {
    AtEnd,
    Inside,
}

/// Whether the selected row has a preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Previewable {
    Yes,
    No,
}

/// The pane's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PreviewState {
    pub pane: Pane,
    pub focus: PaneFocus,
    pub edit: FieldEdit,
}

/// A key the pane machine reads (the panel maps real keys to these; `keymap.rs` style).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreviewKey {
    /// A printable character other than Space went into the field.
    Typed,
    /// Up or Down moved the selection.
    Moved,
    Space,
    /// ⌘Y.
    QuickLook,
    Right(Caret),
    /// ⌘→.
    CommandRight,
    Left,
    Tab,
    ShiftTab,
    Escape,
}

/// What the panel must do besides keeping the new state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreviewOut {
    /// Let the key through to the field (a Space typed, a → moving the caret).
    PassToField,
    /// Give the keyboard to the pane's control at this index.
    FocusPane(usize),
    /// Give the keyboard back to the field.
    FocusField,
    /// Nothing is left to close here: the launcher closes (Escape at the outermost level).
    CloseLauncher,
}

/// `state` after `key`, the selected row being `previewable`, the pane having `controls`
/// controls in its Tab order.
pub fn step_preview(
    state: PreviewState,
    key: PreviewKey,
    previewable: Previewable,
    controls: usize,
) -> (PreviewState, Vec<PreviewOut>) {
    match state.focus {
        PaneFocus::Pane(at) => in_pane(state, key, at, controls),
        PaneFocus::Results => in_results(state, key, previewable, controls),
    }
}

/// A key while the results (the field) have the keyboard.
fn in_results(
    state: PreviewState,
    key: PreviewKey,
    previewable: Previewable,
    controls: usize,
) -> (PreviewState, Vec<PreviewOut>) {
    let pass = (state, vec![PreviewOut::PassToField]);
    match (key, state.edit, state.pane) {
        (PreviewKey::Typed, _, _) => (edited(state, FieldEdit::Typing), Vec::new()),
        (PreviewKey::Moved, _, _) => (edited(state, FieldEdit::Browsing), Vec::new()),
        (PreviewKey::Space, FieldEdit::Typing, _) => pass,
        (PreviewKey::Space | PreviewKey::QuickLook, _, Pane::Shown) => {
            (shown(state, Pane::Hidden), Vec::new())
        }
        (PreviewKey::Space | PreviewKey::QuickLook, _, Pane::Hidden) => match previewable {
            Previewable::Yes => (shown(state, Pane::Shown), Vec::new()),
            // Nothing to look at: a browsing Space is a space after all; ⌘Y does nothing.
            Previewable::No if key == PreviewKey::Space => pass,
            Previewable::No => (state, Vec::new()),
        },
        (PreviewKey::Right(Caret::Inside), _, _) => pass,
        (PreviewKey::Right(Caret::AtEnd) | PreviewKey::CommandRight, _, Pane::Shown) => {
            enter(state, controls)
        }
        (PreviewKey::Right(Caret::AtEnd) | PreviewKey::CommandRight, _, Pane::Hidden) => {
            match previewable {
                Previewable::Yes => (shown(state, Pane::Shown), Vec::new()),
                Previewable::No => pass,
            }
        }
        (PreviewKey::Escape, _, Pane::Shown) => (shown(state, Pane::Hidden), Vec::new()),
        (PreviewKey::Escape, _, Pane::Hidden) => (state, vec![PreviewOut::CloseLauncher]),
        (PreviewKey::Left | PreviewKey::Tab | PreviewKey::ShiftTab, _, _) => pass,
    }
}

/// A key while the pane's control `at` has the keyboard.
fn in_pane(
    state: PreviewState,
    key: PreviewKey,
    at: usize,
    controls: usize,
) -> (PreviewState, Vec<PreviewOut>) {
    let back = PreviewState {
        focus: PaneFocus::Results,
        ..state
    };
    let focus = |to: usize| {
        (
            PreviewState {
                focus: PaneFocus::Pane(to),
                ..state
            },
            vec![PreviewOut::FocusPane(to)],
        )
    };
    let count = controls.max(1);
    match key {
        PreviewKey::Tab => focus((at + 1) % count),
        PreviewKey::ShiftTab => focus((at + count - 1) % count),
        PreviewKey::Left | PreviewKey::Escape => (back, vec![PreviewOut::FocusField]),
        // ⌘Y hides the pane from inside it too, and the field takes the keyboard back.
        PreviewKey::QuickLook => (
            PreviewState {
                pane: Pane::Hidden,
                ..back
            },
            vec![PreviewOut::FocusField],
        ),
        // Space presses the focused control (quire's button hears it); the rest do nothing here.
        PreviewKey::Typed
        | PreviewKey::Moved
        | PreviewKey::Space
        | PreviewKey::Right(_)
        | PreviewKey::CommandRight => (state, Vec::new()),
    }
}

fn edited(state: PreviewState, edit: FieldEdit) -> PreviewState {
    PreviewState { edit, ..state }
}

fn shown(state: PreviewState, pane: Pane) -> PreviewState {
    PreviewState { pane, ..state }
}

/// Move the keyboard into a shown pane, onto its first control; a pane with no controls keeps
/// it in the field.
fn enter(state: PreviewState, controls: usize) -> (PreviewState, Vec<PreviewOut>) {
    match controls {
        0 => (state, Vec::new()),
        _ => (
            PreviewState {
                focus: PaneFocus::Pane(0),
                ..state
            },
            vec![PreviewOut::FocusPane(0)],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Caret, FieldEdit, Pane, PaneFocus, PreviewKey, PreviewOut, PreviewState, Previewable,
        step_preview,
    };

    fn run(keys: &[PreviewKey], previewable: Previewable) -> (PreviewState, Vec<PreviewOut>) {
        keys.iter()
            .fold((PreviewState::default(), Vec::new()), |(state, _), key| {
                step_preview(state, *key, previewable, 3)
            })
    }

    #[test]
    fn space_is_quick_look_only_while_browsing() {
        use PreviewKey::{Moved, Space, Typed};
        let (state, out) = run(&[Typed, Space], Previewable::Yes);
        assert_eq!(state.pane, Pane::Hidden);
        assert_eq!(out, [PreviewOut::PassToField], "typing: Space is a space");
        let (state, out) = run(&[Typed, Moved, Space], Previewable::Yes);
        assert_eq!(state.pane, Pane::Shown);
        assert!(out.is_empty());
        let (state, _) = run(&[Moved, Space, Space], Previewable::Yes);
        assert_eq!(state.pane, Pane::Hidden, "Space again closes it");
        let (state, _) = run(&[Moved, Space], Previewable::No);
        assert_eq!(state.pane, Pane::Hidden, "nothing to preview");
        let (state, _) = run(&[Moved, Typed], Previewable::Yes);
        assert_eq!(state.edit, FieldEdit::Typing);
    }

    #[test]
    fn command_y_and_the_right_arrow_open_and_enter_the_pane() {
        use PreviewKey::{CommandRight, QuickLook, Right, Typed};
        let (state, _) = run(&[Typed, QuickLook], Previewable::Yes);
        assert_eq!(state.pane, Pane::Shown, "⌘Y whatever the field is doing");
        let (state, out) = run(&[Right(Caret::Inside)], Previewable::Yes);
        assert_eq!(
            (state.pane, out),
            (Pane::Hidden, vec![PreviewOut::PassToField])
        );
        let (state, _) = run(&[Right(Caret::AtEnd)], Previewable::Yes);
        assert_eq!(state.pane, Pane::Shown);
        let (state, out) = run(
            &[Right(Caret::AtEnd), Right(Caret::AtEnd)],
            Previewable::Yes,
        );
        assert_eq!(state.focus, PaneFocus::Pane(0));
        assert_eq!(out, [PreviewOut::FocusPane(0)]);
        let (state, _) = run(&[CommandRight, CommandRight], Previewable::Yes);
        assert_eq!(state.focus, PaneFocus::Pane(0));
    }

    #[test]
    fn tab_walks_the_pane_and_escape_closes_one_level_at_a_time() {
        use PreviewKey::{CommandRight, Escape, Left, ShiftTab, Tab};
        let (state, out) = run(
            &[CommandRight, CommandRight, Tab, Tab, Tab],
            Previewable::Yes,
        );
        assert_eq!(
            state.focus,
            PaneFocus::Pane(0),
            "wraps within the pane's 3 controls"
        );
        assert_eq!(out, [PreviewOut::FocusPane(0)]);
        let (state, _) = run(&[CommandRight, CommandRight, ShiftTab], Previewable::Yes);
        assert_eq!(state.focus, PaneFocus::Pane(2));
        let (state, out) = run(&[CommandRight, CommandRight, Left], Previewable::Yes);
        assert_eq!(
            (state.focus, out),
            (PaneFocus::Results, vec![PreviewOut::FocusField])
        );
        let steps = [CommandRight, CommandRight, Escape];
        let (state, out) = run(&steps, Previewable::Yes);
        assert_eq!((state.pane, state.focus), (Pane::Shown, PaneFocus::Results));
        assert_eq!(out, [PreviewOut::FocusField]);
        let (state, out) = run(&[CommandRight, Escape], Previewable::Yes);
        assert_eq!((state.pane, out), (Pane::Hidden, vec![]));
        let (_, out) = run(&[Escape], Previewable::Yes);
        assert_eq!(out, [PreviewOut::CloseLauncher]);
    }
}
