//! The panel's companion mode, as its handlers: the keys it reads before the palette, the
//! switch of mode in place, the search that follows the companion's own state, and the selection
//! that follows a new answer.

use std::time::Duration;

use dioxus::prelude::*;
use ds::components::menus::palette::palette_claim::Claim;
use prov::TaskId;
use sill_launcher::{LauncherMode, ProviderKind, Subject};
use sill_model::companion::{CompanionCommand, Summon};
use sill_ui_kit::links::LauncherOpening;

use super::{CompanionOut, companion_step, follows_state, pane_of, selected_of};
use crate::launcher::events::PanelCtx;
use crate::launcher::keymap::companion_key;
use crate::launcher::keys::{Filter, LauncherState};
use crate::launcher::session::{Session, filter_for};

/// How long the selection waits for an ask's answer to become the front task. The companion
/// answers in seconds when it answers at all.
const ANSWER_DEADLINE: Duration = Duration::from_secs(10);

impl PanelCtx {
    /// Whether the opening on this output is in companion mode.
    pub(crate) fn in_companion_mode(&self) -> bool {
        self.opening()
            .is_some_and(|opening| opening.mode == LauncherMode::Companion)
    }

    /// Tell the companion service something.
    fn companion_send(&self, command: CompanionCommand) {
        if let Err(error) = self.services.companion.control.send(command) {
            eprintln!("sill: launcher: {error}");
        }
    }

    /// The panel's mode changed in place (the same opening): becoming a prompt keeps the search
    /// text as a chip and empties the field; going back puts it back. The rows follow the mode.
    pub(crate) fn mode_changed(&self, opening: &LauncherOpening) {
        let Some(session) = self.current() else {
            return;
        };
        let filter = filter_for(opening.mode);
        let next = match opening.mode {
            LauncherMode::Companion => Session {
                kept: Some(session.query.clone()),
                query: String::new(),
                keys: LauncherState {
                    filter,
                    selected: 0,
                    ..session.keys
                },
                ..session
            },
            LauncherMode::Search => Session {
                query: session.kept.clone().unwrap_or_default(),
                kept: None,
                keys: LauncherState {
                    filter,
                    selected: 0,
                    ..session.keys
                },
                ..session
            },
        };
        let (query, filter) = (next.query.clone(), next.keys.filter);
        let mut store = self.session;
        store.set(next);
        self.search(&query, filter);
        self.field.request();
    }

    /// A key before the palette reads it, in companion mode: what [`companion_step`] makes of it,
    /// or `None` when it is not one of its keys or the mode is not the companion's.
    pub(crate) fn companion_claim(
        &self,
        key: &Key,
        modifiers: Modifiers,
        action: Option<&chordkit::Action>,
    ) -> Option<Claim> {
        if !self.in_companion_mode() {
            return None;
        }
        let key = companion_key(key, modifiers, action)?;
        let (session, pane) = (self.current()?, self.current_pane()?);
        let shown = self.view_of(&session, &pane.expanded);
        let selected = selected_of(shown.row(session.keys.selected).as_ref());
        let current = pane_of(&selected, &self.services.companion.state.borrow());
        let outs = companion_step(key, &current, &selected);
        if outs.is_empty() {
            return None;
        }
        for out in outs {
            self.companion_out(out);
        }
        Some(Claim::Take)
    }

    fn companion_out(&self, out: CompanionOut) {
        match out {
            CompanionOut::Promote(task) => {
                self.companion_send(CompanionCommand::Promote(task));
                self.link.close();
            }
            CompanionOut::Cancel(task) => self.companion_send(CompanionCommand::Cancel(task)),
            CompanionOut::StopRun(run) => self.companion_send(CompanionCommand::Run {
                run,
                control: sill_launcher::RunControl::Stop,
            }),
            CompanionOut::Back => self.select(0),
            CompanionOut::Dismiss => {
                // The summon decides whether Escape closes the panel or restores the search it
                // came from; a panel nobody summoned has no summon to ask.
                let summoned = matches!(
                    self.services.companion.state.borrow().summon,
                    Summon::InLauncher { .. }
                );
                if summoned {
                    self.companion_send(CompanionCommand::Dismiss);
                } else {
                    self.link.close();
                }
            }
        }
    }

    /// The companion's own state changed (an answer arrived, a run moved, the journal grew):
    /// ask again for the rows it makes, in companion mode.
    pub(crate) fn companion_refresh(&self) {
        if !self.in_companion_mode() {
            return;
        }
        if let Some(session) = self.current().filter(|s| follows_state(s.keys.filter)) {
            self.search(&session.query, session.keys.filter);
        }
    }

    /// An answer to a waiting memory came back (kept, dropped or failed): ask for the Memory
    /// filter's rows again, once, so the row leaves at once. The timeline's own answers do not
    /// come here (`Companion::memory_answers` moves only on a settle), so this cannot feed itself.
    pub(crate) fn memory_refresh(&self) {
        if let Some(session) = self
            .current()
            .filter(|s| s.keys.filter == Filter::Only(ProviderKind::Memory))
        {
            self.search(&session.query, session.keys.filter);
        }
    }

    /// Put the selection on `task`'s row, once the rows have it.
    pub(crate) fn select_answer(&self, task: &TaskId) {
        let (Some(session), Some(pane)) = (self.current(), self.current_pane()) else {
            return;
        };
        let at = self
            .view_of(&session, &pane.expanded)
            .line_of(|item| matches!(&item.subject, Subject::Answer(wanted) if wanted == task));
        if let Some(at) = at {
            self.select(at);
        }
    }

    /// After an ask was sent: wait for the companion to make its answer the front task, then
    /// want the selection on it once the rows have it.
    pub(crate) async fn after_ask(&self, before: Option<TaskId>) {
        let mut state = self.services.companion.state.clone();
        // A companion that never answers (no connection, the ask refused) must not keep this
        // waiting: past the deadline the selection simply stays where it is.
        let waited = tokio::time::timeout(
            ANSWER_DEADLINE,
            state.wait_for(|companion| companion.front.is_some() && companion.front != before),
        )
        .await;
        let front = match waited {
            Ok(Ok(companion)) => companion.front.clone(),
            Ok(Err(_)) | Err(_) => None,
        };
        if let Some(task) = front {
            let mut wanted = self.wanted;
            wanted.set(Some(task));
            self.apply_wanted();
        }
    }

    /// Select the answer the selection is wanted on, if the rows have it now.
    pub(crate) fn apply_wanted(&self) {
        let mut wanted = self.wanted;
        let Some(task) = wanted.peek().clone() else {
            return;
        };
        let (Some(session), Some(pane)) = (self.current(), self.current_pane()) else {
            return;
        };
        let present = self
            .view_of(&session, &pane.expanded)
            .line_of(|item| matches!(&item.subject, Subject::Answer(at) if *at == task));
        if present.is_some() {
            wanted.set(None);
            self.select_answer(&task);
        }
    }
}
