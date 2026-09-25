//! What the panel does on its inputs: an opening (reset, first query, `open_latency_ms`), a
//! keystroke in the field (a new generation), a key the palette leaves alone (the keyboard
//! machine), a pick (activation, then close) and a dismissal.

use dioxus::prelude::*;
use ds::{FocusRequest, Rect, use_focus_request};
use shell_host::{OutputId, SurfaceHandle};
use sill_launcher::{Choice, ItemKey};
use sill_services::{Results, SearchCommand, Services};

use crate::hooks::use_services;
use crate::surfaces::launcher::activate::{
    AfterEffect, LauncherEffect, LauncherProviders, effect_of,
};
use crate::surfaces::launcher::entries::RowRef;
use crate::surfaces::launcher::follow::{use_follow_visibility, use_shown};
use crate::surfaces::launcher::groups::{fresh, groups, next_shown, rows};
use crate::surfaces::launcher::keymap::launcher_key;
use crate::surfaces::launcher::keys::{
    ActionsMenu, LauncherKey, LauncherOut, LauncherState, next_filter, previous_filter, step,
};
use crate::surfaces::launcher::panel::shown_groups;
use crate::surfaces::launcher::presence::{LauncherLink, LauncherShown, OpenSerial, Opening};
use crate::surfaces::launcher::run::{EffectFrom, run_effect};
use crate::surfaces::launcher::session::Session;
use crate::surfaces::power_menu::PowerMenuLink;

/// The stderr line for one opening: from the request's arrival (`sill launcher toggle|open`)
/// to the first frame the compositor showed of the panel. The daemon's
/// `launcher.open_latency_ms` (M4 perf-ime) times the IPC dispatch alone.
pub fn first_frame_line(elapsed: std::time::Duration) -> String {
    format!(
        "launcher.open_to_frame_ms={:.3}",
        elapsed.as_secs_f64() * 1000.0
    )
}

/// Whether a pick is under way: the palette calls `onclose` just before `onpick`, and the
/// launcher must stay mapped until the pick has its activation token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Picking {
    No,
    Yes,
}

/// Everything the panel's handlers reach. Clone is cheap.
#[derive(Clone)]
pub(crate) struct PanelCtx {
    output: OutputId,
    link: LauncherLink,
    shown: ReadSignal<LauncherShown>,
    pub session: Signal<Session>,
    /// The palette field's focus: asked for again when the actions menu closes.
    pub field: FocusRequest,
    /// The selected row's rect, as the palette reports it: where the actions menu hangs.
    pub row: Signal<Option<Rect>>,
    results: ReadSignal<Results>,
    services: Services,
    providers: LauncherProviders,
    surface: Option<SurfaceHandle>,
    /// Where a Log Out, Restart or Shut Down row opens the power menu (the daemon gives every
    /// root one; a demo or a test may not).
    power_menu: Option<PowerMenuLink>,
    picking: CopyValue<Picking>,
}

/// The panel's state and handlers on `output`, following the shared launcher state.
pub(crate) fn use_panel(output: OutputId, results: ReadSignal<Results>) -> PanelCtx {
    let link = use_context::<LauncherLink>();
    let services = use_services();
    let providers = use_hook(|| {
        try_consume_context::<LauncherProviders>()
            .unwrap_or_else(|| LauncherProviders::from_services(&services))
    });
    let surface = try_use_context::<SurfaceHandle>();
    let shown = use_shown(&link);
    use_follow_visibility(shown, output);
    let ctx = PanelCtx {
        output,
        link,
        shown,
        session: use_signal(Session::default),
        field: use_focus_request(),
        row: use_signal(|| None),
        results,
        services,
        providers,
        surface,
        power_menu: try_use_context::<PowerMenuLink>(),
        picking: use_hook(|| CopyValue::new(Picking::No)),
    };
    use_openings(ctx.clone());
    ctx
}

/// Start a session on every opening on this output; cancel the search on a close.
fn use_openings(ctx: PanelCtx) {
    let mut last = use_hook(|| CopyValue::new(None::<OpenSerial>));
    use_effect(move || {
        let opening = ctx.opening_tracked();
        let seen = *last.peek();
        match (opening, seen) {
            (Some(opening), seen) if seen != Some(opening.serial) => {
                last.set(Some(opening.serial));
                ctx.opened(&opening);
            }
            (None, Some(_)) => {
                last.set(None);
                let _ = ctx.services.search.send(SearchCommand::Cancel);
            }
            _ => {}
        }
    });
}

impl PanelCtx {
    /// The opening on this output, if the launcher shows here.
    pub fn opening(&self) -> Option<Opening> {
        self.shown
            .peek()
            .opening()
            .filter(|o| o.output == self.output)
            .cloned()
    }

    /// [`PanelCtx::opening`], subscribing the caller to changes.
    pub fn opening_tracked(&self) -> Option<Opening> {
        self.shown
            .read()
            .opening()
            .filter(|o| o.output == self.output)
            .cloned()
    }

    /// A new opening: a fresh session over the results so far, its first query, and the
    /// latency from the request to the first frame the compositor showed.
    fn opened(&self, opening: &Opening) {
        let since = self.results.peek().generation;
        let mut session = self.session;
        session.set(Session::opened(opening, since));
        let mut row = self.row;
        row.set(None);
        let _ = self
            .services
            .search
            .send(SearchCommand::Query(opening.query.clone()));
        if let Some(surface) = self.surface.clone() {
            let requested = opening.requested;
            spawn(async move {
                let drive = surface.drive_frames();
                let _tick = surface.next_frame().await;
                drop(drive);
                eprintln!("{}", first_frame_line(requested.elapsed()));
            });
        }
    }

    /// The session as it stands for the current opening.
    fn current(&self) -> Option<Session> {
        self.opening()
            .map(|opening| self.session.peek().for_opening(&opening))
    }

    /// A key the palette's field got, after the palette read it: Up, Down, Enter and Escape
    /// are the palette's own (its selection is the session's, asked for through
    /// [`PanelCtx::select`]); Tab, Shift+Tab, Ctrl+K and Alt+K are the machine's.
    pub fn field_key(&self, key: &KeyboardData) {
        if let Some(key @ (LauncherKey::Tab | LauncherKey::ShiftTab | LauncherKey::Actions)) =
            launcher_key(&key.key(), key.modifiers())
        {
            self.press(key);
        }
    }

    /// A key reached the surface's root: the keys the machine reads from the field must not
    /// take Blitz's default (Tab moves the focus off the field), and the palette's `onkey`
    /// hands on the event's data alone, so the root prevents it as the key bubbles past.
    pub fn root_key(event: &KeyboardEvent) {
        if matches!(
            launcher_key(&event.key(), event.modifiers()),
            Some(LauncherKey::Tab | LauncherKey::ShiftTab | LauncherKey::Actions)
        ) {
            event.prevent_default();
        }
    }

    /// The palette asked for row `index` (Up, Down, the pointer): it becomes the selection.
    pub fn select(&self, index: usize) {
        let Some(session) = self.current() else {
            return;
        };
        if session.keys.actions != ActionsMenu::Closed || session.keys.selected == index {
            return;
        }
        let mut store = self.session;
        store.set(Session {
            keys: LauncherState {
                selected: index,
                ..session.keys
            },
            ..session
        });
    }

    /// `key` through the keyboard machine: Tab skips providers with no rows, an Escape the
    /// machine does not keep for the actions menu closes the launcher.
    pub fn press(&self, key: LauncherKey) {
        let Some(session) = self.current() else {
            return;
        };
        let results = self.results.peek().clone();
        let current = shown_groups(&results, &session);
        let shown = rows(&current);
        let actions = shown
            .get(session.keys.selected)
            .map_or(0, |item| item.actions.len());
        let was = session.keys.filter;
        let (keys, outs) = step(session.keys.clone(), key, shown.len(), actions);
        let items = fresh(&results, session.since);
        let keys = match key {
            LauncherKey::Tab | LauncherKey::ShiftTab if keys.actions == ActionsMenu::Closed => {
                let next = match key {
                    LauncherKey::ShiftTab => previous_filter,
                    _ => next_filter,
                };
                // The palette is not remounted for a filter: it keeps its selection, clamped
                // to the rows the new filter shows.
                let filter = next_shown(items, &session.query, was, next);
                let shown = groups(items, &session.query, filter);
                LauncherState {
                    filter,
                    selected: session
                        .keys
                        .selected
                        .min(rows(&shown).len().saturating_sub(1)),
                    ..keys
                }
            }
            _ => keys,
        };
        let closing =
            session.keys.actions != ActionsMenu::Closed && keys.actions == ActionsMenu::Closed;
        let mut store = self.session;
        store.set(Session { keys, ..session });
        if closing {
            self.field.request();
        }
        if outs.contains(&LauncherOut::Close) {
            self.link.close();
        }
    }

    /// The field's text changed: a new generation of results.
    pub fn typed(&self, text: String) {
        let mut session = self.session;
        if let Some(current) = self.current() {
            session.set(current.typed(text.clone()));
        }
        let _ = self.services.search.send(SearchCommand::Query(text));
    }

    /// A row (or one of its actions) was chosen: count it, carry out what its provider says,
    /// then close.
    pub fn pick(&self, row: RowRef, choice: Choice) {
        let Some(session) = self.current() else {
            return;
        };
        let results = self.results.peek().clone();
        let Some(item) = row.find(fresh(&results, session.since)).cloned() else {
            return;
        };
        // Chosen first: the runner looks the row up among the current rows, which closing
        // clears (FINDINGS F117).
        let _ = self.services.search.send(SearchCommand::Chosen {
            key: ItemKey::of(item.provider, &item.id),
            choice: choice.clone(),
        });
        let effect = effect_of(self.providers.activation(&item, &choice));
        let mut picking = self.picking;
        picking.set(Picking::Yes);
        if let LauncherEffect::Insert(text) = &effect {
            // The field takes the text and the launcher stays; the palette's `onclose` that
            // came with an Enter is ignored for this turn.
            self.typed(text.clone());
            spawn(async move { picking.set(Picking::No) });
            return;
        }
        let from = EffectFrom {
            surface: self.surface.clone(),
            output: self.output,
            power_menu: self.power_menu.clone(),
        };
        let (services, link) = (self.services.clone(), self.link.clone());
        spawn(async move {
            let after = effect.after();
            if let Err(error) = run_effect(effect, &from, &services).await {
                eprintln!("sill: launcher: {error}");
            }
            if after == AfterEffect::Close {
                link.close();
            }
            picking.set(Picking::No);
        });
    }

    /// The palette asked to close (Escape, Enter, a press outside its card). Deferred a turn,
    /// so a pick that follows in the same handler keeps the panel mapped for its token.
    pub fn dismissed(&self) {
        let (picking, link) = (self.picking, self.link.clone());
        spawn(async move {
            if *picking.peek() == Picking::No {
                link.close();
            }
        });
    }

    /// The actions menu closed (Escape, a pick, a press outside it): it unmounts and the field
    /// has the keyboard back, the palette and its selection as they were.
    pub fn actions_closed(&self) {
        if let Some(session) = self.current() {
            let mut store = self.session;
            store.set(Session {
                keys: LauncherState {
                    actions: ActionsMenu::Closed,
                    ..session.keys
                },
                ..session
            });
        }
        self.field.request();
    }
}
