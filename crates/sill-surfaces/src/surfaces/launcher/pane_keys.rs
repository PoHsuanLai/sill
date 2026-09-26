//! The M9 keys (F642, F654): the preview pane's machine (`preview::keys::step_preview`) fed
//! from the palette's field and from `sill debug launcher-key`, row chords run from the results
//! (`action_for_chord`), and Enter on the pane's focused control.
//!
//! The pane's keyboard focus is the machine's `PaneFocus::Pane(n)`, drawn by the pane on its
//! `n`th control; the field keeps the DOM focus, so its keys still arrive here (quire's
//! `PreviewPane`, Q292, takes the same `focused` index). Two gaps wait on quire (Q299): the
//! field inserts a Space before `onkey` hears it, so a Space while browsing is recognised by the
//! text it made (the query then carries a trailing space, which asks nothing new); and the
//! caret is not reported, so → reads the estimate in `pane_session.rs`.

use dioxus::prelude::*;
use sill_launcher::{Choice, Item, Preview, action_for_chord, chords, preview_for};

use crate::surfaces::launcher::chord_keys::chord_of;
use crate::surfaces::launcher::debug_link::LinkKey;
use crate::surfaces::launcher::entries::RowRef;
use crate::surfaces::launcher::events::{Held, PanelCtx};
use crate::surfaces::launcher::keymap::launcher_key;
use crate::surfaces::launcher::keys::{ActionsMenu, LauncherKey};
use crate::surfaces::launcher::pane_session::{CaretAt, CaretMove, PaneSession, moved_caret};
use crate::surfaces::launcher::preview::keys::{
    FieldEdit, PaneFocus, PreviewKey, PreviewOut, PreviewState, Previewable, step_preview,
};
use crate::surfaces::launcher::session::Session;

/// Whether a key was the panel's to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Taken {
    Yes,
    No,
}

impl PanelCtx {
    /// A key the palette's field got, after the palette read it (Up, Down, Enter and Escape
    /// are the palette's own; it has moved the selection or picked already).
    pub fn field_key(&self, event: &KeyboardEvent) {
        if self.field_key_taken(&event.key(), event.modifiers()) == Taken::Yes {
            event.prevent_default();
        }
    }

    fn field_key_taken(&self, key: &Key, modifiers: Modifiers) -> Taken {
        let launcher = launcher_key(key, modifiers);
        if let Some(LauncherKey::Actions) = launcher {
            self.press(LauncherKey::Actions);
            return Taken::Yes;
        }
        if let Some(chord) = chord_of(key, modifiers) {
            return self.chord(&chord);
        }
        match (launcher, key) {
            (Some(tab @ (LauncherKey::Tab | LauncherKey::ShiftTab)), _) => {
                self.tab(tab);
                Taken::Yes
            }
            (Some(LauncherKey::Up | LauncherKey::Down), _) => {
                self.preview_key(PreviewKey::Moved);
                Taken::No
            }
            (Some(LauncherKey::Escape), _) => {
                self.field_escape();
                Taken::No
            }
            (_, Key::ArrowRight) => self.caret_key(PreviewKey::Right, CaretMove::Right),
            (_, Key::ArrowLeft) => self.caret_key(|_| PreviewKey::Left, CaretMove::Left),
            (_, Key::Home) => self.caret_moved(CaretMove::Home),
            (_, Key::End) => self.caret_moved(CaretMove::End),
            _ => Taken::No,
        }
    }

    /// `sill debug launcher-key`: `key` as if pressed in the open panel.
    pub fn debug_key(&self, key: LinkKey) {
        let Some(session) = self.current() else {
            return;
        };
        match key {
            LinkKey::Up | LinkKey::Down => {
                self.press(match key {
                    LinkKey::Up => LauncherKey::Up,
                    _ => LauncherKey::Down,
                });
                if session.keys.actions == ActionsMenu::Closed {
                    self.preview_key(PreviewKey::Moved);
                }
            }
            LinkKey::Enter => self.press(LauncherKey::Enter),
            LinkKey::Escape if session.keys.actions != ActionsMenu::Closed => {
                self.press(LauncherKey::Escape);
            }
            LinkKey::Escape => {
                if self
                    .preview_key(PreviewKey::Escape)
                    .contains(&PreviewOut::CloseLauncher)
                {
                    self.link.close();
                }
            }
            LinkKey::Tab => self.tab(LauncherKey::Tab),
            LinkKey::ShiftTab => self.tab(LauncherKey::ShiftTab),
            LinkKey::Actions => self.press(LauncherKey::Actions),
            LinkKey::Space => {
                if self
                    .preview_key(PreviewKey::Space)
                    .contains(&PreviewOut::PassToField)
                {
                    self.typed(format!("{} ", session.query));
                }
            }
            LinkKey::QuickLook => {
                self.preview_key(PreviewKey::QuickLook);
            }
            LinkKey::Right => {
                self.caret_key(PreviewKey::Right, CaretMove::Right);
            }
            LinkKey::Left => {
                self.caret_key(|_| PreviewKey::Left, CaretMove::Left);
            }
            LinkKey::Reveal => {
                self.chord(&chords::reveal());
            }
        }
    }

    /// A press on the pane's control `index`: the selected row's action there.
    pub(crate) fn run_pane_action(&self, index: usize) {
        let (Some(session), Some(pane)) = (self.current(), self.current_pane()) else {
            return;
        };
        let Some(item) = self
            .view_of(&session, &pane.expanded)
            .row(session.keys.selected)
        else {
            return;
        };
        if let Some(action) = item.actions.get(index) {
            self.pick(RowRef::of(&item), Choice::Action(action.id.clone()));
        }
    }

    /// The choice a pick of `item` makes: with the pane's control `n` holding the keyboard,
    /// that control's action; else `choice`.
    pub(crate) fn pane_choice(&self, item: &Item, choice: Choice) -> Choice {
        let focus = self.current_pane().map(|pane| pane.preview.focus);
        match (focus, choice) {
            (Some(PaneFocus::Pane(n)), Choice::Primary) => item
                .actions
                .get(n)
                .map_or(Choice::Primary, |action| Choice::Action(action.id.clone())),
            (_, choice) => choice,
        }
    }

    /// `text` is the query with one Space added while browsing a row with a preview: Quick Look
    /// took it (Q299: the field inserted it first). The query keeps the space, so the field and
    /// the query agree, but nothing is asked again and the selection stays.
    pub(crate) fn browsing_space(&self, text: &str) -> bool {
        let (Some(session), Some(pane)) = (self.current(), self.current_pane()) else {
            return false;
        };
        if !one_space_more(&session.query, text)
            || pane.preview.edit != FieldEdit::Browsing
            || pane.preview.focus != PaneFocus::Results
        {
            return false;
        }
        let (preview, outs) = self.step_pane(&pane, &session, PreviewKey::Space);
        if outs.contains(&PreviewOut::PassToField) {
            return false;
        }
        self.store_pane(PaneSession {
            preview,
            caret: CaretAt(pane.caret.0 + 1),
            ..pane
        });
        let mut store = self.session;
        store.set(Session {
            query: text.to_owned(),
            ..session
        });
        true
    }

    /// `key` through the pane's machine; returns what came out.
    pub(crate) fn preview_key(&self, key: PreviewKey) -> Vec<PreviewOut> {
        let (Some(session), Some(pane)) = (self.current(), self.current_pane()) else {
            return Vec::new();
        };
        let (preview, outs) = self.step_pane(&pane, &session, key);
        self.store_pane(PaneSession { preview, ..pane });
        if outs.contains(&PreviewOut::FocusField) {
            self.field.request();
        }
        outs
    }

    /// The machine's step for `key` over the selected row.
    pub(crate) fn step_pane(
        &self,
        pane: &PaneSession,
        session: &Session,
        key: PreviewKey,
    ) -> (PreviewState, Vec<PreviewOut>) {
        let selected = self
            .view_of(session, &pane.expanded)
            .row(session.keys.selected);
        let previewable = match selected.as_ref().map(preview_for) {
            Some(Preview::None) | None => Previewable::No,
            Some(_) => Previewable::Yes,
        };
        let controls = selected.map_or(0, |item| item.actions.len());
        step_preview(pane.preview, key, previewable, controls)
    }

    /// Tab or Shift+Tab: the pane's controls while it has the keyboard, else the filter.
    fn tab(&self, key: LauncherKey) {
        let in_pane = self
            .current_pane()
            .is_some_and(|pane| matches!(pane.preview.focus, PaneFocus::Pane(_)));
        match (in_pane, key) {
            (true, LauncherKey::ShiftTab) => {
                self.preview_key(PreviewKey::ShiftTab);
            }
            (true, _) => {
                self.preview_key(PreviewKey::Tab);
            }
            (false, key) => self.press(key),
        }
    }

    /// Escape in the field: the palette has already asked to close; the pane may keep it open.
    fn field_escape(&self) {
        if !self
            .preview_key(PreviewKey::Escape)
            .contains(&PreviewOut::CloseLauncher)
        {
            let mut held = self.held;
            held.set(Held::Yes);
        }
    }

    /// ← or → (`key` given the caret): the pane's when it takes it, else the field's caret moves.
    fn caret_key(
        &self,
        key: impl Fn(crate::surfaces::launcher::preview::keys::Caret) -> PreviewKey,
        step: CaretMove,
    ) -> Taken {
        let (Some(session), Some(pane)) = (self.current(), self.current_pane()) else {
            return Taken::No;
        };
        let outs = self.preview_key(key(pane.caret_in(&session.query)));
        if outs.contains(&PreviewOut::PassToField) {
            self.caret_moved(step)
        } else {
            Taken::Yes
        }
    }

    /// The field moved its caret by `step`.
    fn caret_moved(&self, step: CaretMove) -> Taken {
        if let (Some(session), Some(pane)) = (self.current(), self.current_pane()) {
            self.store_pane(PaneSession {
                caret: moved_caret(pane.caret, step, &session.query),
                ..pane
            });
        }
        Taken::No
    }

    /// A chord from the results: ⌘Y and ⌘→ are the pane's; any other runs the selected row's
    /// action that carries it (H5).
    fn chord(&self, chord: &sill_launcher::Chord) -> Taken {
        if *chord == chords::quick_look() {
            self.preview_key(PreviewKey::QuickLook);
            return Taken::Yes;
        }
        if *chord == sill_launcher::Chord::command(sill_launcher::ChordKey::Right) {
            self.preview_key(PreviewKey::CommandRight);
            return Taken::Yes;
        }
        let (Some(session), Some(pane)) = (self.current(), self.current_pane()) else {
            return Taken::No;
        };
        let Some(item) = self
            .view_of(&session, &pane.expanded)
            .row(session.keys.selected)
        else {
            return Taken::No;
        };
        match action_for_chord(&item, chord) {
            Some(action) => {
                self.pick(RowRef::of(&item), Choice::Action(action.clone()));
                Taken::Yes
            }
            None => Taken::No,
        }
    }
}

/// Whether `new` is `old` with one Space inserted somewhere (where the caret was).
pub(crate) fn one_space_more(old: &str, new: &str) -> bool {
    new.len() == old.len() + 1
        && new.char_indices().any(|(at, c)| {
            c == ' ' && old.get(..at) == Some(&new[..at]) && old.get(at..) == Some(&new[at + 1..])
        })
}

/// Step every `sill debug launcher-key` press on the panel (the daemon provides the link to
/// every root; only the panel the launcher is open on acts).
pub(crate) fn use_link_keys(panel: PanelCtx) {
    use_hook(move || {
        let mut rx = panel.link.subscribe_keys();
        rx.borrow_and_update();
        spawn(async move {
            while rx.changed().await.is_ok() {
                let key = rx.borrow_and_update().key;
                if let (Some(key), Some(_)) = (key, panel.opening()) {
                    panel.debug_key(key);
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::one_space_more;

    #[test]
    fn a_space_inserted_anywhere_is_one_space_more() {
        let cases = [
            ("m9", "m9 ", true),
            ("m9", " m9", true),
            ("m9", "m 9", true),
            ("m9", "m9x", false),
            ("m9", "m9  ", false),
            ("", " ", true),
            ("naïf", "na ïf", true),
        ];
        for (old, new, want) in cases {
            assert_eq!(one_space_more(old, new), want, "{old:?} -> {new:?}");
        }
    }
}
