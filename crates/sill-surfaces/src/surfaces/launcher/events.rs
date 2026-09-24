//! What the panel does on its inputs: an opening (reset, first query, `open_latency_ms`), a
//! keystroke in the field (a new generation), a key the palette leaves alone (the keyboard
//! machine), a pick (activation, then close) and a dismissal.

use dioxus::prelude::*;
use shell_host::{OutputId, SurfaceHandle};
use sill_launcher::{Choice, ItemKey};
use sill_services::{Results, SearchCommand, Services};

use crate::hooks::use_services;
use crate::surfaces::launcher::activate::{LauncherProviders, effect_of};
use crate::surfaces::launcher::entries::RowRef;
use crate::surfaces::launcher::follow::{use_follow_visibility, use_shown};
use crate::surfaces::launcher::groups::{fresh, next_shown, rows};
use crate::surfaces::launcher::keymap::launcher_key;
use crate::surfaces::launcher::keys::{
    ActionsMenu, LauncherKey, LauncherOut, LauncherState, next_filter, previous_filter, step,
};
use crate::surfaces::launcher::panel::shown_groups;
use crate::surfaces::launcher::presence::{LauncherLink, LauncherShown, OpenSerial, Opening};
use crate::surfaces::launcher::run::run_effect;
use crate::surfaces::launcher::session::Session;

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
    results: ReadSignal<Results>,
    services: Services,
    providers: LauncherProviders,
    surface: Option<SurfaceHandle>,
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
        results,
        services,
        providers,
        surface,
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

    /// A key reached the surface's root. The palette has already handled Up, Down, Enter and
    /// Escape for itself; the machine mirrors them so Ctrl+K opens the right row's actions,
    /// and it alone handles Tab, Shift+Tab, Ctrl+K and Alt+K.
    pub fn key(&self, event: &KeyboardEvent) {
        let Some(key) = launcher_key(&event.key(), event.modifiers()) else {
            return;
        };
        if matches!(
            key,
            LauncherKey::Tab | LauncherKey::ShiftTab | LauncherKey::Actions
        ) {
            event.prevent_default();
            event.stop_propagation();
        }
        self.press(key);
    }

    /// `key` through the keyboard machine: Tab skips providers with no rows, an Escape the
    /// machine does not keep for the actions menu closes the launcher.
    pub fn press(&self, key: LauncherKey) {
        let Some(session) = self.current() else {
            return;
        };
        let results = self.results.peek().clone();
        let groups = shown_groups(&results, &session);
        let shown = rows(&groups);
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
                LauncherState {
                    filter: next_shown(items, &session.query, was, next),
                    ..keys
                }
            }
            _ => keys,
        };
        let mut store = self.session;
        store.set(Session { keys, ..session });
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
        let activation = self.providers.activation(&item, &choice);
        let _ = self.services.search.send(SearchCommand::Chosen {
            key: ItemKey::of(item.provider, &item.id),
            choice,
        });
        let mut picking = self.picking;
        picking.set(Picking::Yes);
        let (surface, services, link) =
            (self.surface.clone(), self.services.clone(), self.link.clone());
        spawn(async move {
            if let Err(error) = run_effect(effect_of(activation), surface, &services).await {
                eprintln!("sill: launcher: {error}");
            }
            link.close();
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

    /// The actions menu closed (Escape, a pick, a press outside it): the palette takes the
    /// keyboard back by remounting.
    pub fn actions_closed(&self) {
        let Some(session) = self.current() else {
            return;
        };
        let mut store = self.session;
        store.set(
            Session {
                keys: LauncherState {
                    actions: ActionsMenu::Closed,
                    ..session.keys
                },
                ..session
            }
            .remounted(),
        );
    }
}
