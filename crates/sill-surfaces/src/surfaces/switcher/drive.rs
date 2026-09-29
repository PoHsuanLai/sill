//! The effects end of one switcher surface: follow the [`SwitcherLink`] for this output, step the
//! pure [`step`] machine, and carry out what it says. A chord maps the surface (keyboard
//! `Exclusive`, so it sees the release) and snapshots the apps; the show delay is quire's one timer
//! (`ds::sleep`); `Activate`, `Quit` and `HideApp` go to `Services.cosmic_wl`; a quit tile folds
//! and is dropped at `settle(Anim::Fold)`; the machine back at `Hidden` unmaps at once. Hidden, no
//! task runs and nothing asks for a frame.

use std::time::Duration;

use crate::services::Services;
use dioxus::core::Task;
use dioxus::prelude::*;
use ds::{Anim, AppKey, MotionLevel, StaggerIndex};
use shell_host::{OutputId, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_launcher::AppId;
use sill_services::switcher::{SwitcherApp, switcher_apps};

use super::hold::{Hold, Watch, chord_input, tick_input};
use super::keys::{Chord, chord_after_keyup, sw_key};
use super::machine::{SwIn, SwKey, SwOut, Switcher, SwitcherParams, step};
use super::session::Session;
use super::tiles::index_of;
use crate::FrameClock;
use crate::links::Dir;
use crate::links::{AskSerial, SwitcherAsk, SwitcherLink};

/// One surface's switcher: its machine, its opening, and what it talks to. Copy, so every
/// handler and task holds it.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct SwitcherCtx {
    pub state: Signal<Switcher>,
    pub session: Signal<Option<Session>>,
    params: CopyValue<SwitcherParams>,
    level: CopyValue<MotionLevel>,
    clock: CopyValue<FrameClock>,
    ticking: CopyValue<Option<Task>>,
    surface: CopyValue<Option<SurfaceHandle>>,
    services: CopyValue<Services>,
    /// What the modifiers have told this opening (`hold.rs`).
    hold: CopyValue<Hold>,
}

/// Run `output`'s switcher from `link` with `params` (read at each input) and `level` (for the
/// fold's settle).
pub(crate) fn use_switcher(
    link: &SwitcherLink,
    output: OutputId,
    params: SwitcherParams,
    level: MotionLevel,
) -> SwitcherCtx {
    let latest = use_watch(link.subscribe());
    // A surface respawned after a close does not replay the ask before it.
    let mut seen = use_hook(|| CopyValue::new(link.latest().map(|asked| asked.serial)));
    let services = crate::hooks::use_services();
    let surface = try_use_context::<SurfaceHandle>();
    let ctx = use_hook(|| SwitcherCtx {
        state: Signal::new(Switcher::Hidden),
        session: Signal::new(None),
        params: CopyValue::new(params),
        level: CopyValue::new(level),
        clock: CopyValue::new(FrameClock::started()),
        ticking: CopyValue::new(None),
        surface: CopyValue::new(surface),
        services: CopyValue::new(services),
        hold: CopyValue::new(Hold::default()),
    });
    ctx.keep(params, level);
    use_effect(move || {
        let Some(asked) = latest.read().clone() else {
            return;
        };
        if seen.peek().is_some_and(|serial| asked.serial <= serial) {
            return;
        }
        seen.set(Some(asked.serial));
        if asked.output == output {
            ctx.ask(asked.serial, asked.ask);
        }
    });
    ctx
}

impl SwitcherCtx {
    /// A settings change applies from the next input.
    fn keep(self, params: SwitcherParams, level: MotionLevel) {
        let (mut kept, mut kept_level) = (self.params, self.level);
        if *kept.peek() != params {
            kept.set(params);
        }
        if *kept_level.peek() != level {
            kept_level.set(level);
        }
    }

    /// One ask from the daemon.
    pub fn ask(self, serial: AskSerial, ask: SwitcherAsk) {
        match ask {
            SwitcherAsk::Chord(dir) => {
                if *self.state.peek() == Switcher::Hidden {
                    let apps = self.live();
                    if apps.is_empty() {
                        return;
                    }
                    self.open(Session::live(serial, apps));
                }
                let (state, hold) = (*self.state.peek(), *self.hold.peek());
                self.input(chord_input(state, hold.chord, dir));
            }
            SwitcherAsk::Cancel => self.input(SwIn::Key(SwKey::Escape)),
            SwitcherAsk::Commit => self.input(SwIn::ModifierReleased),
            SwitcherAsk::Fake(ids) => self.fake(Session::made_up(serial, ids)),
        }
    }

    /// Open on made-up apps, shown at once: the chord and the show delay's tick in one go.
    fn fake(self, session: Session) {
        self.close();
        let ids = session.ids();
        self.open(session);
        let (now, params) = (self.clock.peek().now(), *self.params.peek());
        let (armed, first) = step(Switcher::Hidden, SwIn::Chord(Dir::Next), now, &ids, &params);
        let later = now.after(params.show_delay_ms);
        let (shown, then) = step(armed, SwIn::Tick, later, &ids, &params);
        let outs = first.into_iter().chain(then);
        self.apply(
            shown,
            outs.filter(|out| !matches!(out, SwOut::RequestTick(_)))
                .collect(),
        );
    }

    /// Start an opening: its apps, and the surface mapped so it takes the keyboard.
    fn open(self, session: Session) {
        let mut hold = self.hold;
        hold.set(Hold::default());
        let mut stored = self.session;
        stored.set(Some(session));
        self.visibility(Visibility::Shown);
    }

    /// One input through the machine.
    pub fn input(self, input: SwIn) {
        let ids = self
            .session
            .peek()
            .as_ref()
            .map(Session::ids)
            .unwrap_or_default();
        let now = self.clock.peek().now();
        let (next, outs) = step(*self.state.peek(), input, now, &ids, &self.params.peek());
        self.apply(next, outs);
    }

    fn apply(self, next: Switcher, outs: Vec<SwOut>) {
        let session = self.session.peek().clone();
        if let Some(session) = &session {
            let live = self.live();
            let wl = self.services.peek().cosmic_wl.control.clone();
            for command in outs.iter().flat_map(|out| session.commands(out, &live)) {
                let _ = wl.send(command);
            }
        }
        let mut state = self.state;
        state.set(next);
        for out in outs {
            match out {
                SwOut::RequestTick(at) => self.tick_at(at.since(self.clock.peek().now())),
                SwOut::Quit(app) => self.fold(app),
                SwOut::Show | SwOut::Hide | SwOut::Select(_) => {}
                SwOut::Activate(_) | SwOut::HideApp(_) => {}
            }
        }
        if next == Switcher::Hidden {
            self.close();
        }
    }

    /// Back to nothing: no timer, no opening, the surface unmapped.
    fn close(self) {
        let mut ticking = self.ticking;
        if let Some(task) = ticking.take() {
            task.cancel();
        }
        let (mut state, mut session) = (self.state, self.session);
        if session.peek().is_some() {
            session.set(None);
            self.visibility(Visibility::Hidden);
        }
        if *state.peek() != Switcher::Hidden {
            state.set(Switcher::Hidden);
        }
    }

    /// The machine's `Tick` in `ms`.
    fn tick_at(self, ms: u64) {
        let mut ticking = self.ticking;
        if let Some(task) = ticking.take() {
            task.cancel();
        }
        ticking.set(Some(spawn(async move {
            ds::sleep(Duration::from_millis(ms)).await;
            let mut ticking = self.ticking;
            ticking.set(None);
            let (state, hold) = (*self.state.peek(), *self.hold.peek());
            self.input(tick_input(state, hold));
        })));
    }

    /// Q: `app`'s tile folds, then goes; the panel stays (closing when none is left).
    fn fold(self, app: AppId) {
        let mut session = self.session;
        let Some(serial) = session.peek().as_ref().map(|s| s.serial) else {
            return;
        };
        session.with_mut(|s| *s = s.take().map(|s| s.leave(&app)));
        let settle = ds::settle(Anim::Fold, *self.level.peek(), StaggerIndex::default());
        spawn(async move {
            ds::sleep(settle).await;
            let mut session = self.session;
            if session.peek().as_ref().map(|s| s.serial) != Some(serial) {
                return;
            }
            session.with_mut(|s| *s = s.take().map(|s| s.gone(&app)));
            self.input(SwIn::Tick);
        });
    }

    fn visibility(self, visibility: Visibility) {
        if let Some(surface) = self.surface.peek().as_ref() {
            surface.apply(SurfaceOp::SetVisibility(visibility));
        }
    }

    /// The compositor's apps now.
    fn live(self) -> Vec<SwitcherApp> {
        let services = self.services.peek();
        let toplevels = services.cosmic_wl.state.borrow().toplevels.clone();
        let index = services.apps.state.borrow().clone();
        switcher_apps(&toplevels, &index)
    }

    /// A key-down on the surface: the switcher's keys go to the machine and never to Blitz
    /// (Tab would move the focus).
    pub fn key_down(self, event: &KeyboardEvent) {
        if let Some(key) = sw_key(&event.key(), event.modifiers()) {
            event.prevent_default();
            self.input(SwIn::Key(key));
        }
    }

    /// The modifiers published on the surface (on focus, and at each change): a release is the
    /// release, whether or not a key event carried it.
    pub fn modifiers(self, chord: Chord) {
        let mut hold = self.hold;
        hold.with_mut(|hold| hold.chord = Some(chord));
        if chord == Chord::Released {
            self.input(SwIn::ModifierReleased);
        }
    }

    /// This opening follows the modifiers (`watch.rs`, as it mounts), from `baseline`.
    pub fn watched(self, baseline: Chord) {
        let mut hold = self.hold;
        hold.set(Hold {
            watch: Watch::On,
            chord: Some(baseline),
        });
    }

    /// A key-up on the surface: the chord's modifier going up is the release. Only a modifier's
    /// own key-up counts: another key's carries whatever modifiers the event was given (a virtual
    /// keyboard's own, say), and a release with no key-up at all is the modifiers' publish.
    pub fn key_up(self, event: &KeyboardEvent) {
        let key = event.key();
        if matches!(key, Key::Alt | Key::Meta | Key::Super)
            && chord_after_keyup(&key, event.modifiers()) == Chord::Released
        {
            self.input(SwIn::ModifierReleased);
        }
    }

    /// The pointer on a tile selects it; a click activates it.
    pub fn hover(self, key: &AppKey) {
        if let Some(index) = self.index(key) {
            self.input(SwIn::Hover(index));
        }
    }

    pub fn click(self, key: &AppKey) {
        if let Some(index) = self.index(key) {
            self.input(SwIn::Click(index));
        }
    }

    fn index(self, key: &AppKey) -> Option<usize> {
        let session = self.session.peek();
        index_of(key, &session.as_ref()?.apps)
    }
}
