//! The pane keys: the preview pane's machine (`preview::keys::step_preview`) fed
//! from the palette's field, row chords run from the results
//! (`action_for_key`), and Enter on the pane's focused control.
//!
//! The field keeps the keyboard: the pane's focus is the machine's `focus: Some(n)`, drawn by
//! quire's `PreviewPane` as `focused`. Keys reach the machine through quire's key claim:
//! [`PanelCtx::claim`] hears every key before the palette with the caret's place, and takes Space
//! while browsing, → at the field's end (not on an emoji cell, where ← and → are the grid's:
//! `claim_keys.rs`), and the pane's own keys, so the field never types that Space nor moves its
//! caret. The keys the palette leaves (Tab's filter, ⌘K, Up and Down
//! having moved the selection) come after it, to [`PanelCtx::field_key`].

use dioxus::prelude::*;
use ds::components::menus::palette::palette_claim::{Claim, FieldKey};
use sill_launcher::{Choice, Item, OwnAction, Preview, action_for_key, preview_for};

use crate::launcher::claim_keys::{KeyPlace, pane_key};
use crate::launcher::entries::RowRef;
use crate::launcher::events::PanelCtx;
use crate::launcher::keys::LauncherKey;
use crate::launcher::pane_session::PaneSession;
use crate::launcher::preview::keys::{Caret, PreviewKey, PreviewOut, PreviewState, step_preview};
use crate::launcher::session::Session;
use crate::launcher::view::Layout;

impl PanelCtx {
    /// quire's key claim: a key the field got, before the palette, the field or `onkey`.
    pub fn claim(&self, key: &FieldKey) -> Claim {
        let (pressed, modifiers) = (key.event.key(), key.event.modifiers());
        let action = self.resolved(&pressed, modifiers);
        if self.launcher_key_of(&key.event) == Some(LauncherKey::Actions) {
            return Claim::Pass;
        }
        if let Some(claim) = self.companion_claim(&pressed, modifiers, action.as_ref()) {
            return claim;
        }
        if let Some(action) = action {
            return self.run_action(&action);
        }
        let Some(pane) = self.current_pane() else {
            return Claim::Pass;
        };
        let place = KeyPlace {
            pane: pane.preview.pane,
            focus: pane.preview.focus,
            layout: self.selected_layout(),
        };
        let Some(preview) = pane_key(&pressed, modifiers, caret_of(key.caret), place) else {
            return Claim::Pass;
        };
        let outs = self.preview_key(preview);
        match outs.contains(&PreviewOut::PassToField) || outs.contains(&PreviewOut::CloseLauncher) {
            true => Claim::Pass,
            false => Claim::Take,
        }
    }

    /// A key the palette's field got and nobody claimed, after the palette read it (it has moved
    /// the selection on Up and Down): Tab's filter, ⌘K, and the moves that make Space Quick
    /// Look.
    pub fn field_key(&self, event: &KeyboardEvent) {
        match self.launcher_key_of(event) {
            Some(key @ (LauncherKey::Tab | LauncherKey::ShiftTab | LauncherKey::Actions)) => {
                event.prevent_default();
                self.press(key);
            }
            Some(LauncherKey::Up | LauncherKey::Down) => {
                self.preview_key(PreviewKey::Moved);
            }
            // ← and → the claim passed on an emoji cell: the palette moved the cell.
            None if matches!(event.key(), Key::ArrowLeft | Key::ArrowRight)
                && self.selected_layout() == Layout::Grid =>
            {
                self.preview_key(PreviewKey::Moved);
            }
            _ => {}
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
        let focus = self.current_pane().and_then(|pane| pane.preview.focus);
        match (focus, choice) {
            (Some(n), Choice::Primary) => item
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
        let previewable = !matches!(
            selected.as_ref().map(preview_for),
            Some(Preview::None) | None
        );
        let controls = selected.map_or(0, |item| item.actions.len());
        step_preview(pane.preview, key, previewable, controls)
    }

    /// How the selected line is laid out: a cell of the emoji grid, or a list line.
    fn selected_layout(&self) -> Layout {
        match (self.current(), self.current_pane()) {
            (Some(session), Some(pane)) => self
                .view_of(&session, &pane.expanded)
                .layout(session.keys.selected),
            _ => Layout::List,
        }
    }

    /// The selected row, if the selection is on a row.
    fn selected_row(&self) -> Option<Item> {
        let (session, pane) = (self.current()?, self.current_pane()?);
        self.view_of(&session, &pane.expanded)
            .row(session.keys.selected)
    }

    /// An action the keymap made of a key: Quick Look and `sill.focus-pane` are the pane's; any
    /// other runs the selected row's action bound to it (H5), and nothing else hears it.
    fn run_action(&self, action: &chordkit::Action) -> Claim {
        if *action == chordkit::Action::Standard(chordkit::StandardAction::QuickLook) {
            self.preview_key(PreviewKey::QuickLook);
            return Claim::Take;
        }
        if OwnAction::of(action) == Some(OwnAction::FocusPane) {
            self.preview_key(PreviewKey::CommandRight);
            return Claim::Take;
        }
        let Some(item) = self.selected_row() else {
            return Claim::Pass;
        };
        match action_for_key(&item, action) {
            Some(id) => {
                self.pick(RowRef::of(&item), Choice::Action(id.clone()));
                Claim::Take
            }
            None => Claim::Pass,
        }
    }
}

/// quire's caret as the pane's machine reads it: a caret nobody could read is not at the end
/// (→ moves it; ⌘Y and ⌘→ still reach the pane).
fn caret_of(caret: ds::host::caret::Caret) -> Caret {
    match caret {
        ds::host::caret::Caret::AtEnd => Caret::AtEnd,
        ds::host::caret::Caret::Inside | ds::host::caret::Caret::Unknown => Caret::Inside,
    }
}

#[cfg(test)]
mod tests {
    use super::caret_of;
    use crate::launcher::preview::keys::Caret;

    #[test]
    fn only_a_caret_read_at_the_end_is_at_the_end() {
        assert_eq!(caret_of(ds::host::caret::Caret::AtEnd), Caret::AtEnd);
        assert_eq!(caret_of(ds::host::caret::Caret::Inside), Caret::Inside);
        assert_eq!(caret_of(ds::host::caret::Caret::Unknown), Caret::Inside);
    }
}
