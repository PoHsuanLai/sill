//! What the panel does on its inputs: an opening (reset, first query, `open_latency_ms`), a
//! keystroke in the field (a new generation), a key the palette leaves alone (the keyboard
//! machine), a pick (activation, then close) and a dismissal.

use dioxus::prelude::*;
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
use crate::surfaces::launcher::mounting::{PaletteMount, Remount};
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

/// How long after an opening its new palette mounts.
pub const MOUNT_WAIT: std::time::Duration = std::time::Duration::from_millis(60);

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
    /// The mounted palette (`mounting.rs`).
    pub palette: Signal<PaletteMount>,
    /// When the actions menu last mounted: it too is kept past its entrance (`mounting.rs`).
    menu_at: CopyValue<Option<std::time::Instant>>,
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
        palette: use_signal(PaletteMount::default),
        menu_at: use_hook(|| CopyValue::new(None)),
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
        // The new palette mounts a moment after the opening, once its first results have
        // usually landed: mounting it while a results update also re-renders the panel is what
        // most often lets quire's focus-on-mount task race the document (FINDINGS Q43). Until
        // then the mounted one shows this opening's state.
        let mut palette = self.palette;
        spawn(async move {
            ds::sleep(MOUNT_WAIT).await;
            let next = palette.peek().for_opening(std::time::Instant::now());
            palette.set(next);
        });
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
        if session.keys.actions != ActionsMenu::Closed && keys.actions == ActionsMenu::Closed {
            // The menu closes through the same guarded path as its own close.
            self.actions_closed();
            return;
        }
        if session.keys.actions == ActionsMenu::Closed && keys.actions != ActionsMenu::Closed {
            let mut menu_at = self.menu_at;
            menu_at.set(Some(std::time::Instant::now()));
        }
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
        let (surface, services, link) = (
            self.surface.clone(),
            self.services.clone(),
            self.link.clone(),
        );
        spawn(async move {
            let after = effect.after();
            if let Err(error) = run_effect(effect, surface, &services).await {
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

    /// The actions menu closed (Escape, a pick, a press outside it): once the menu's entrance
    /// has settled it unmounts, and the palette takes the keyboard back by remounting once its
    /// own has (`mounting.rs`).
    pub fn actions_closed(&self) {
        let now = std::time::Instant::now();
        let menu = PaletteMount {
            key: 0,
            at: *self.menu_at.peek(),
        };
        let (ctx, serial) = (self.clone(), self.current().map(|s| s.serial));
        match menu.remount_at(now) {
            Remount::Now => ctx.menu_gone(serial),
            Remount::After(wait) => {
                spawn(async move {
                    ds::sleep(wait).await;
                    ctx.menu_gone(serial);
                });
            }
        }
    }

    /// The actions menu unmounts, if the same opening is still showing it, and the palette is
    /// remounted for the keyboard.
    fn menu_gone(&self, serial: Option<crate::surfaces::launcher::presence::OpenSerial>) {
        let Some(session) = self.current().filter(|s| Some(s.serial) == serial) else {
            return;
        };
        let mut store = self.session;
        store.set(Session {
            keys: LauncherState {
                actions: ActionsMenu::Closed,
                selected: 0,
                ..session.keys
            },
            ..session
        });
        let mut palette = self.palette;
        let mounted = *palette.peek();
        match mounted.remount_at(std::time::Instant::now()) {
            Remount::Now => palette.set(mounted.next(std::time::Instant::now())),
            Remount::After(wait) => {
                spawn(async move {
                    ds::sleep(wait).await;
                    if palette.peek().key == mounted.key {
                        palette.set(mounted.next(std::time::Instant::now()));
                    }
                });
            }
        }
    }
}
