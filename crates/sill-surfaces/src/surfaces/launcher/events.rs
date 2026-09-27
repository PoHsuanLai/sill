//! What the panel does on its inputs: an opening (reset, first query, `open_latency_ms`), a
//! keystroke in the field (a new generation, filtered while Tab's filter holds), a key the
//! palette leaves alone (the keyboard machine), a pick (activation, then close; a "Show More"
//! line opens its section) and a dismissal. The M9 keys (the preview pane's, row chords, the
//! debug link's) are `pane_keys.rs`.

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
use crate::surfaces::launcher::groups::fresh;
use crate::surfaces::launcher::keys::{
    ActionsMenu, Filter, LauncherKey, LauncherOut, LauncherState, next_filter, previous_filter,
    step,
};
use crate::surfaces::launcher::pane_session::{PaneSession, next_in_ring, ring_for};
use crate::surfaces::launcher::presence::{LauncherLink, LauncherShown, OpenSerial, Opening};
use crate::surfaces::launcher::preview::keys::{PreviewKey, PreviewState};
use crate::surfaces::launcher::run::{EffectFrom, run_effect};
use crate::surfaces::launcher::session::Session;
use crate::surfaces::launcher::view::{Expanded, View, toggled, view};
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
pub(crate) enum Picking {
    No,
    Yes,
}

/// Everything the panel's handlers reach. Clone is cheap.
#[derive(Clone)]
pub(crate) struct PanelCtx {
    pub(crate) output: OutputId,
    pub(crate) link: LauncherLink,
    pub(crate) shown: ReadSignal<LauncherShown>,
    pub session: Signal<Session>,
    /// The M9 state: the preview pane, "Show More", Tab's ring.
    pub pane: Signal<PaneSession>,
    /// The palette field's focus: asked for again when the actions menu closes.
    pub field: FocusRequest,
    /// The selected row's rect, as the palette reports it: where the actions menu hangs.
    pub row: Signal<Option<Rect>>,
    pub(crate) results: ReadSignal<Results>,
    pub(crate) services: Services,
    pub(crate) providers: LauncherProviders,
    pub(crate) surface: Option<SurfaceHandle>,
    /// Where a Log Out, Restart or Shut Down row opens the power menu (the daemon gives every
    /// root one; a demo or a test may not).
    pub(crate) power_menu: Option<PowerMenuLink>,
    pub(crate) picking: CopyValue<Picking>,
    /// How many times the preview pane has finished leaving (quire's `on_hidden`).
    pub(crate) left: Signal<u64>,
    /// quire's palette handle (Q400): a Show More / Show Less run here, not through the
    /// palette's own Enter or click, still plays the group's rise or heal.
    pub(crate) palette: ds::PaletteHandle,
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
        pane: use_signal(PaneSession::default),
        field: use_focus_request(),
        row: use_signal(|| None),
        results,
        services,
        providers,
        surface,
        power_menu: try_use_context::<PowerMenuLink>(),
        picking: use_hook(|| CopyValue::new(Picking::No)),
        left: use_signal(|| 0),
        palette: ds::use_palette_handle(),
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
                ctx.link.show_rows(None);
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
        let mut pane = self.pane;
        pane.set(PaneSession {
            serial: opening.serial,
            ..PaneSession::default()
        });
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
                eprintln!("{}", first_frame_line(ds::time::since(requested)));
            });
        }
    }

    /// The session as it stands for the current opening.
    pub(crate) fn current(&self) -> Option<Session> {
        self.opening()
            .map(|opening| self.session.peek().for_opening(&opening))
    }

    /// The M9 state as it stands for the current opening.
    pub(crate) fn current_pane(&self) -> Option<PaneSession> {
        self.opening()
            .map(|opening| self.pane.peek().for_serial(opening.serial))
    }

    /// What `session` shows over the latest results, with `pane`'s sections opened.
    pub(crate) fn view_of(&self, session: &Session, expanded: &Expanded) -> View {
        view(
            &self.results.peek(),
            session.since,
            &session.query,
            session.keys.filter,
            expanded,
        )
    }

    /// A key reached the surface's root: the keys the machine reads from the field must not
    /// take Blitz's default (Tab moves the focus off the field), as they bubble past.
    pub fn root_key(event: &KeyboardEvent) {
        if matches!(
            crate::surfaces::launcher::keymap::launcher_key(&event.key(), event.modifiers()),
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

    /// `key` through the keyboard machine: Tab visits the providers of Tab's ring and asks the
    /// new filter's provider alone (`SearchCommand::Filtered`), an Escape the machine does not
    /// keep for the actions menu closes the launcher, an Enter runs the selected line.
    pub fn press(&self, key: LauncherKey) {
        let (Some(session), Some(pane)) = (self.current(), self.current_pane()) else {
            return;
        };
        let shown = self.view_of(&session, &pane.expanded);
        let selected = shown.row(session.keys.selected);
        let actions = selected.as_ref().map_or(0, |item| item.actions.len());
        let was = session.keys.filter;
        let (keys, outs) = step(session.keys.clone(), key, shown.len(), actions);
        let keys = match key {
            LauncherKey::Tab | LauncherKey::ShiftTab if keys.actions == ActionsMenu::Closed => {
                let ring = match was {
                    Filter::All => ring_for(&session.query, shown.providers()),
                    Filter::Only(_) => pane.ring.clone(),
                };
                let next = match key {
                    LauncherKey::ShiftTab => previous_filter,
                    _ => next_filter,
                };
                let filter = next_in_ring(was, &ring, next);
                self.store_pane(PaneSession {
                    ring,
                    preview: PreviewState::default(),
                    ..pane.clone()
                });
                self.search(&session.query, filter);
                LauncherState {
                    filter,
                    selected: 0,
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
        for out in outs {
            match out {
                LauncherOut::Close => self.link.close(),
                LauncherOut::Activate { row, choice } => self.run_line(&shown, row, choice),
            }
        }
    }

    /// Run line `index` of `shown` (a debug Enter; the palette's own Enter comes as a pick).
    pub(crate) fn run_line(&self, shown: &View, index: usize, choice: Choice) {
        match shown.row(index) {
            Some(item) => self.pick(RowRef::of(&item), choice),
            None => {
                if let Some(section) = shown.lines().get(index).and_then(|line| match line {
                    crate::surfaces::launcher::view::Line::More { section, .. } => Some(*section),
                    crate::surfaces::launcher::view::Line::Row(_) => None,
                }) {
                    self.toggle_section(section);
                }
            }
        }
    }

    /// Ask for `text` under `filter`: every provider, or the filtered one alone.
    pub(crate) fn search(&self, text: &str, filter: Filter) {
        let command = match filter {
            Filter::All => SearchCommand::Query(text.to_owned()),
            Filter::Only(provider) => SearchCommand::Filtered {
                text: text.to_owned(),
                provider,
            },
        };
        let _ = self.services.search.send(command);
    }

    /// The field's text changed: a new generation of results.
    pub fn typed(&self, text: String) {
        let Some(current) = self.current() else {
            let _ = self.services.search.send(SearchCommand::Query(text));
            return;
        };
        if let Some(pane) = self.current_pane() {
            let (preview, _) = self.step_pane(&pane, &current, PreviewKey::Typed);
            self.store_pane(PaneSession {
                preview,
                expanded: Expanded::default(),
                ..pane
            });
        }
        let filter = current.keys.filter;
        let mut session = self.session;
        session.set(current.typed(text.clone()));
        self.search(&text, filter);
    }

    /// Keep `pane` as this opening's M9 state.
    pub(crate) fn store_pane(&self, pane: PaneSession) {
        let mut store = self.pane;
        store.set(pane);
    }

    /// The preview pane has played out its leaving: the palette drops its `aside`.
    pub(crate) fn pane_gone(&self) {
        let mut left = self.left;
        left += 1;
    }

    /// Open or close `section`'s hidden rows; the selection stays on its line.
    /// Its line moves (below the added rows, or up past the removed ones), so the selection
    /// follows it: left on the old index it would land on another row, whose "↵" would change
    /// a second group and make quire's palette read the change as a new result set (Q373's
    /// expand and collapse play only when the one group grew or shrank).
    pub(crate) fn toggle_section(&self, section: sill_launcher::Section) {
        let (Some(session), Some(pane)) = (self.current(), self.current_pane()) else {
            return;
        };
        let expanded = toggled(&pane.expanded, section);
        self.palette.mark_group_action(section.title());
        let line = self.view_of(&session, &expanded).more_line(section);
        self.store_pane(PaneSession { expanded, ..pane });
        match line {
            Some(selected) if selected != session.keys.selected => {
                let mut store = self.session;
                store.set(Session {
                    keys: LauncherState {
                        selected,
                        ..session.keys
                    },
                    ..session
                });
            }
            Some(_) | None => {}
        }
    }

    /// A row (or one of its actions) was chosen: count it, carry out what its provider says,
    /// then close. With the preview pane's control `n` holding the keyboard, Enter runs that
    /// control's action.
    pub fn pick(&self, row: RowRef, choice: Choice) {
        let Some(session) = self.current() else {
            return;
        };
        let mut picking = self.picking;
        let results = self.results.peek().clone();
        let Some(item) = row.find(fresh(&results, session.since)).cloned() else {
            return;
        };
        let choice = self.pane_choice(&item, choice);
        // Chosen first: the runner looks the row up among the current rows, which closing
        // clears (FINDINGS F117).
        let _ = self.services.search.send(SearchCommand::Chosen {
            key: ItemKey::of(item.provider, &item.id),
            choice: choice.clone(),
        });
        let effect = effect_of(self.providers.activation(&item, &choice));
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
        let again = match &effect {
            // A forgotten entry leaves the rows: ask again so it goes (the launcher stays).
            LauncherEffect::ForgetClipboard(_) => {
                Some((session.query.clone(), session.keys.filter))
            }
            _ => None,
        };
        let ctx = self.clone();
        spawn(async move {
            let after = effect.after();
            if let Err(error) = run_effect(effect, &from, &services).await {
                eprintln!("sill: launcher: {error}");
            }
            if after == AfterEffect::Close {
                link.close();
            }
            if let Some((query, filter)) = again {
                ctx.search(&query, filter);
            }
            picking.set(Picking::No);
        });
    }

    /// The palette asked to close (Escape, Enter, a press outside its card). Deferred a turn,
    /// so a pick that follows in the same handler keeps the panel mapped for its token. An
    /// Escape the preview pane takes never reaches the palette (`claim`).
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
