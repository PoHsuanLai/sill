//! The M9 keys (F642, F654): the preview pane's machine (`preview::keys::step_preview`) fed
//! from the palette's field and from `sill debug launcher-key`, row chords run from the results
//! (`action_for_chord`), and Enter on the pane's focused control.
//!
//! The field keeps the keyboard: the pane's focus is the machine's `PaneFocus::Pane(n)`, drawn by
//! quire's `PreviewPane` as `focused`. Keys reach the machine through quire's key claim (Q299):
//! [`PanelCtx::claim`] hears every key before the palette with the caret's place, and takes Space
//! while browsing, → at the field's end, and the pane's own keys, so the field never types that
//! Space nor moves its caret. The keys the palette leaves (Tab's filter, Ctrl+K, Up and Down
//! having moved the selection) come after it, to [`PanelCtx::field_key`].

use dioxus::prelude::*;
use ds::{Claim, FieldKey};
use sill_launcher::{Choice, Item, Preview, action_for_chord, chords, preview_for};

use crate::surfaces::launcher::chord_keys::chord_of;
use crate::surfaces::launcher::debug_link::LinkKey;
use crate::surfaces::launcher::entries::RowRef;
use crate::surfaces::launcher::events::PanelCtx;
use crate::surfaces::launcher::keymap::launcher_key;
use crate::surfaces::launcher::keys::{ActionsMenu, LauncherKey};
use crate::surfaces::launcher::pane_session::PaneSession;
use crate::surfaces::launcher::preview::keys::{
    Caret, Pane, PaneFocus, PreviewKey, PreviewOut, PreviewState, Previewable, step_preview,
};
use crate::surfaces::launcher::session::Session;

impl PanelCtx {
    /// quire's key claim: a key the field got, before the palette, the field or `onkey`.
    pub fn claim(&self, key: &FieldKey) -> Claim {
        let (pressed, modifiers) = (key.event.key(), key.event.modifiers());
        if launcher_key(&pressed, modifiers) == Some(LauncherKey::Actions) {
            return Claim::Pass;
        }
        if let Some(chord) = chord_of(&pressed, modifiers) {
            return self.chord(&chord);
        }
        let Some(pane) = self.current_pane() else {
            return Claim::Pass;
        };
        let in_pane = matches!(pane.preview.focus, PaneFocus::Pane(_));
        let preview = match (&pressed, in_pane) {
            (Key::Character(text), _) if text == " " && modifiers.is_empty() => PreviewKey::Space,
            (Key::ArrowRight, _) => PreviewKey::Right(caret_of(key.caret)),
            (Key::ArrowLeft, true) => PreviewKey::Left,
            (Key::Tab, true) if modifiers.contains(Modifiers::SHIFT) => PreviewKey::ShiftTab,
            (Key::Tab, true) => PreviewKey::Tab,
            (Key::Escape, _) if in_pane || pane.preview.pane == Pane::Shown => PreviewKey::Escape,
            _ => return Claim::Pass,
        };
        let outs = self.preview_key(preview);
        match outs.contains(&PreviewOut::PassToField) || outs.contains(&PreviewOut::CloseLauncher) {
            true => Claim::Pass,
            false => Claim::Take,
        }
    }

    /// A key the palette's field got and nobody claimed, after the palette read it (it has moved
    /// the selection on Up and Down): Tab's filter, Ctrl+K, and the moves that make Space Quick
    /// Look.
    pub fn field_key(&self, event: &KeyboardEvent) {
        match launcher_key(&event.key(), event.modifiers()) {
            Some(key @ (LauncherKey::Tab | LauncherKey::ShiftTab | LauncherKey::Actions)) => {
                event.prevent_default();
                self.press(key);
            }
            Some(LauncherKey::Up | LauncherKey::Down) => {
                self.preview_key(PreviewKey::Moved);
            }
            _ => {}
        }
    }

    /// `sill debug launcher-key`: `key` as if pressed in the open panel. It cannot see the
    /// field's caret, so its → acts as → at the field's end.
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
                self.preview_key(PreviewKey::Right(Caret::AtEnd));
            }
            LinkKey::Left => {
                self.preview_key(PreviewKey::Left);
            }
            LinkKey::Reveal => {
                self.chord(&chords::reveal());
            }
        }
    }

    /// A press on the pane's control `index`: the selected row's action there.
    pub(crate) fn run_pane_action(&self, index: usize) {
        let Some(item) = self.selected_row() else {
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

    /// The selected row, if the selection is on a row.
    fn selected_row(&self) -> Option<Item> {
        let (session, pane) = (self.current()?, self.current_pane()?);
        self.view_of(&session, &pane.expanded)
            .row(session.keys.selected)
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

    /// A chord from the results: ⌘Y and ⌘→ are the pane's; any other runs the selected row's
    /// action that carries it (H5), and nothing else hears it.
    fn chord(&self, chord: &sill_launcher::Chord) -> Claim {
        if *chord == chords::quick_look() {
            self.preview_key(PreviewKey::QuickLook);
            return Claim::Take;
        }
        if *chord == sill_launcher::Chord::command(sill_launcher::ChordKey::Right) {
            self.preview_key(PreviewKey::CommandRight);
            return Claim::Take;
        }
        let Some(item) = self.selected_row() else {
            return Claim::Pass;
        };
        match action_for_chord(&item, chord) {
            Some(action) => {
                self.pick(RowRef::of(&item), Choice::Action(action.clone()));
                Claim::Take
            }
            None => Claim::Pass,
        }
    }
}

/// quire's caret as the pane's machine reads it: a caret nobody could read is not at the end
/// (→ moves it; ⌘Y and ⌘→ still reach the pane).
fn caret_of(caret: ds::Caret) -> Caret {
    match caret {
        ds::Caret::AtEnd => Caret::AtEnd,
        ds::Caret::Inside | ds::Caret::Unknown => Caret::Inside,
    }
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
    use super::caret_of;
    use crate::surfaces::launcher::preview::keys::Caret;

    #[test]
    fn only_a_caret_read_at_the_end_is_at_the_end() {
        assert_eq!(caret_of(ds::Caret::AtEnd), Caret::AtEnd);
        assert_eq!(caret_of(ds::Caret::Inside), Caret::Inside);
        assert_eq!(caret_of(ds::Caret::Unknown), Caret::Inside);
    }
}
