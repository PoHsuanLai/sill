//! What the panel does on its inputs: an opening (reset, first query, `open_latency_ms`), a
//! keystroke in the field (a new generation, filtered while Tab's filter holds), a key the
//! palette leaves alone (the keyboard machine), a pick (activation, then close; a "Show More"
//! line opens its section) and a dismissal. The other keys (the preview pane's, row chords) are `pane_keys.rs`.

use dioxus::prelude::*;
use ds::components::menus::palette::palette_motion::PaletteHandle;
use ds::components::menus::palette::palette_motion::use_palette_handle;
use ds::focus::request::{FocusRequest, use_focus_request};
use ds::prelude::*;
use shell_host::{OutputId, SurfaceHandle};
use sill_launcher::{Choice, CompanionAct, ItemKey, LauncherMode};
use sill_model::results::{Results, SearchCommand};
use sill_ui_kit::services::Services;

use crate::launcher::activate::{
    AfterEffect, LauncherEffect, LauncherProviders, effect_of, file_results, with_results,
};
use crate::launcher::companion::{Way, ask_input, companion_filter};
use crate::launcher::entries::RowRef;
use crate::launcher::follow::{use_follow_visibility, use_shown};
use crate::launcher::groups::fresh;
use crate::launcher::keys::{
    ActionsMenu, Filter, LauncherKey, LauncherOut, LauncherState, next_filter, previous_filter,
    step,
};
use crate::launcher::pane_session::{PaneSession, next_in_ring, ring_for};
use crate::launcher::preview::keys::{PreviewKey, PreviewState};
use crate::launcher::run::{EffectFrom, run_effect};
use crate::launcher::session::Session;
use crate::launcher::view::{Expanded, View, toggled, view};
use sill_ui_kit::hooks::{use_service, use_services};
use sill_ui_kit::links::PowerMenuLink;
use sill_ui_kit::links::{LauncherLink, OpenSerial, Opening, Reveal};

/// The stderr line for one opening: from the request's arrival (`sill launcher toggle|open`)
/// to the first frame the compositor showed of the panel. The daemon's
/// `launcher.open_latency_ms` times the IPC dispatch alone.
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
    pub(crate) shown: ReadSignal<Reveal<Opening>>,
    pub session: Signal<Session>,
    /// The pane state: the preview pane, "Show More", Tab's ring.
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
    /// quire's palette handle: a Show More / Show Less run here, not through the
    /// palette's own Enter or click, still plays the group's rise or heal.
    pub(crate) palette: PaletteHandle,
    /// The answer the selection is to move to once the rows have it (after an ask).
    pub(crate) wanted: Signal<Option<prov::TaskId>>,
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
        palette: use_palette_handle(),
        wanted: use_signal(|| None),
    };
    use_openings(ctx.clone());
    // The companion's rows come from its own state, which the search does not watch: ask again
    // whenever an answer, a run or the journal moves, in companion mode.
    let companion = use_service::<sill_services::CompanionService>();
    use_effect({
        let ctx = ctx.clone();
        move || {
            let _ = companion.read();
            ctx.companion_refresh();
        }
    });
    // A new answer's row arrives with the search that follows the ask: select it then.
    use_effect({
        let ctx = ctx.clone();
        move || {
            let _ = results.read();
            ctx.apply_wanted();
        }
    });
    ctx
}

/// Start a session on every opening on this output; cancel the search on a close.
fn use_openings(ctx: PanelCtx) {
    let mut last = use_hook(|| CopyValue::new(None::<(OpenSerial, LauncherMode)>));
    use_effect(move || {
        let opening = ctx.opening_tracked();
        let seen = *last.peek();
        match (opening, seen) {
            (Some(opening), seen) if seen.map(|(serial, _)| serial) != Some(opening.serial) => {
                last.set(Some((opening.serial, opening.mode)));
                ctx.opened(&opening);
            }
            // The same opening in another mode: the double tap switched it in place.
            (Some(opening), Some((_, mode))) if mode != opening.mode => {
                last.set(Some((opening.serial, opening.mode)));
                ctx.mode_changed(&opening);
            }
            (None, Some(_)) => {
                last.set(None);
                ctx.link.show_rows(None);
                let _ = ctx.services.results.control.send(SearchCommand::Cancel);
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
        let filter = crate::launcher::session::filter_for(opening.mode);
        self.search(&opening.query, filter);
        if let Some(surface) = self.surface.clone() {
            let requested = opening.requested;
            spawn(async move {
                let drive = surface.drive_frames();
                let _tick = surface.next_frame().await;
                drop(drive);
                eprintln!(
                    "{}",
                    first_frame_line(ds::base::time::clock::since(requested))
                );
            });
        }
    }

    /// The session as it stands for the current opening.
    pub(crate) fn current(&self) -> Option<Session> {
        self.opening()
            .map(|opening| self.session.peek().for_opening(&opening))
    }

    /// The pane state as it stands for the current opening.
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
            crate::launcher::keymap::launcher_key(&event.key(), event.modifiers()),
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
                let filter = if self.in_companion_mode() {
                    // There is no "every provider" in a prompt: Tab goes round the companion's
                    // own rows, the runs and the memory.
                    let way = match key {
                        LauncherKey::ShiftTab => Way::Backward,
                        _ => Way::Forward,
                    };
                    companion_filter(was, way)
                } else {
                    next_in_ring(was, &ring, next)
                };
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
                    crate::launcher::view::Line::More { section, .. } => Some(*section),
                    crate::launcher::view::Line::Row(_) => None,
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
        let _ = self.services.results.control.send(command);
    }

    /// The field's text changed: a new generation of results.
    pub fn typed(&self, text: String) {
        let Some(current) = self.current() else {
            let _ = self
                .services
                .results
                .control
                .send(SearchCommand::Query(text));
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

    /// Keep `pane` as this opening's pane state.
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
    /// follows it: left on the old index it would land on another row.
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
        // clears.
        let _ = self.services.results.control.send(SearchCommand::Chosen {
            key: ItemKey::of(item.provider, &item.id),
            choice: choice.clone(),
        });
        let effect = effect_of(self.providers.activation(&item, &choice));
        let effect = match effect {
            // The prompt goes with what this opening kept: the window it was summoned over.
            LauncherEffect::Companion(CompanionAct::Ask { prompt }) => {
                let companion = self.services.companion.state.borrow();
                let toplevels = self.services.cosmic_wl.state.borrow().toplevels.clone();
                // sill has no Space store yet, so no workspace has a Space of its own and every
                // ask is made on the desktop (FINDINGS: the Space store).
                let space = prov::SpaceId::desktop();
                LauncherEffect::Ask(ask_input(prompt, &companion.summon, &toplevels, space))
            }
            other => other,
        };
        let effect = match self.current_pane() {
            Some(pane) => {
                let shown = self.view_of(&session, &pane.expanded);
                with_results(effect, results.generation, file_results(&shown.rows()))
            }
            None => effect,
        };
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
        let asked = matches!(effect, LauncherEffect::Ask(_));
        let front = self.services.companion.state.borrow().front.clone();
        spawn(async move {
            let after = effect.after();
            if let Err(error) = run_effect(effect, &from, &services).await {
                eprintln!("sill: launcher: {error}");
            }
            if asked {
                // The prompt is sent: the field empties and the selection follows the answer.
                ctx.typed(String::new());
                ctx.after_ask(front).await;
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
        // Escape ends what voice was hearing too: the words are discarded, never sent.
        let _ = self
            .services
            .voice
            .control
            .send(sill_model::voice::VoiceCommand::Cancel(
                sill_model::voice::CancelCause::Escape,
            ));
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
