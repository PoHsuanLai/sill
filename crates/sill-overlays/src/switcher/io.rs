//! The effects end of one switcher surface: follow the [`SwitcherLink`] for this output, run the
//! [`Opening`] machine (quire's switcher, the one the compositor runs too) on quire's
//! `use_machine`, and carry out what it says. A chord maps the surface (keyboard `Exclusive`, so
//! it sees the release) and snapshots the apps; the machine reads the apps and the modifiers'
//! state as its context at every step, and the show delay is its own deadline, so there is no
//! timer here; `Activate`, `Quit` and `HideApp` go to `Services.windows`; a quit tile folds and
//! is dropped at `settle(Anim::Fold)`; the machine back at `Hidden` unmaps at once. Hidden, no
//! task runs and nothing asks for a frame.

use ds::base::time::clock::sleep;

use super::apps::{SwitcherApp, switcher_apps};
use super::opening::{Facts, Opening};
use super::pointer::{At, Pointer};
use dioxus::prelude::*;
use ds::machine::{MachineRef, MachineState, use_machine_in, use_machine_state};
use ds::prelude::*;
use shell_host::{OutputId, SurfaceHandle, Visibility, use_watch};
use sill_launcher::AppId;
use sill_shell_kit::switcher::app_switcher::AppKey;
use sill_ui_kit::services::Services;

use super::hold::{Hold, Watch, chord_input};
use super::keys::{Chord, chord_after_keyup, sw_key};
use super::model::{AppOut, heading};
use super::session::Session;
use super::tiles::index_of;
use ds_behaviour::switcher::{Phase, SwIn, SwKey, SwOut, SwitcherParams};
use sill_ui_kit::links::Dir;
use sill_ui_kit::links::{AskSerial, SwitcherAsk, SwitcherLink};

/// One surface's switcher: its machine, its opening, and what it talks to. Copy, so every
/// handler and task holds it.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct SwitcherCtx {
    pub state: Signal<Opening>,
    pub session: Signal<Option<Session>>,
    machine: MachineRef<Opening>,
    surface: CopyValue<Option<SurfaceHandle>>,
    services: CopyValue<Services>,
    /// What the modifiers have told this opening (`hold.rs`).
    hold: CopyValue<Hold>,
    /// Whether the pointer has moved since it opened (`pointer.rs`).
    pointer: CopyValue<Pointer>,
}

/// What carrying out the machine's outputs reads: the opening, the services and the motion level.
#[derive(Clone, Copy)]
struct Effects {
    session: Signal<Option<Session>>,
    services: CopyValue<Services>,
    level: CopyValue<MotionLevel>,
}

/// Run `output`'s switcher from `link` with `params` (read at each input) and `level` (for the
/// fold's settle).
pub(super) fn use_switcher(
    link: &SwitcherLink,
    output: OutputId,
    params: SwitcherParams,
    level: MotionLevel,
) -> SwitcherCtx {
    let asks = use_watch(link.subscribe());
    // A surface respawned after a close does not replay the ask before it.
    let mut seen = use_hook(|| CopyValue::new(link.latest().map(|asked| asked.serial)));
    let services = sill_ui_kit::hooks::use_services();
    let surface = try_use_context::<SurfaceHandle>();
    let session = use_signal(|| None::<Session>);
    let hold = use_hook(|| CopyValue::new(Hold::default()));
    let services = use_hook(|| CopyValue::new(services));
    let mut kept = use_hook(|| CopyValue::new(level));
    if *kept.peek() != level {
        kept.set(level);
    }
    let held: MachineState<Opening> = use_machine_state(|_| Opening::default());
    let effects = Effects {
        session,
        services,
        level: kept,
    };
    let machine = use_machine_in(
        held,
        params,
        move || Facts {
            apps: session
                .peek()
                .as_ref()
                .map(Session::ids)
                .unwrap_or_default(),
            hold: *hold.peek(),
        },
        move |out, machine| effects.carry(out, machine),
    );
    let ctx = SwitcherCtx {
        state: held.state,
        session,
        machine,
        surface: use_hook(|| CopyValue::new(surface)),
        services,
        hold,
        pointer: use_hook(|| CopyValue::new(Pointer::default())),
    };
    // Every ask since the last read, in order: two Tabs before a read are two steps.
    use_effect(move || {
        let fresh = asks.read().since(*seen.peek());
        for asked in fresh {
            seen.set(Some(asked.serial));
            if asked.output == output {
                ctx.ask(asked.serial, asked.ask);
            }
        }
    });
    // The show delay's wake can end the opening with no input from here (a tap that was never
    // read held): back at `Hidden`, close it.
    use_effect(move || {
        if ctx.state.read().phase() == Phase::Hidden {
            ctx.close();
        }
    });
    ctx
}

impl Effects {
    /// Carry out one output of the machine.
    fn carry(self, out: AppOut, machine: MachineRef<Opening>) {
        let session = self.session.peek().clone();
        if let Some(session) = &session {
            let live = live(self.services);
            let wl = self.services.peek().windows.control.clone();
            for command in session.commands(&out, &live) {
                let _ = wl.send(command);
            }
        }
        match out {
            SwOut::Quit(app) => self.fold(app, machine),
            // `sw_key` maps no Up or Down, so the machine never asks for App Exposé.
            SwOut::Show
            | SwOut::Hide
            | SwOut::Select(_)
            | SwOut::Activate(_)
            | SwOut::HideApp(_)
            | SwOut::Expose(_) => {}
        }
    }

    /// Q: `app`'s tile fades out, then goes; the panel stays (closing when none is left).
    fn fold(self, app: AppId, machine: MachineRef<Opening>) {
        let mut session = self.session;
        let Some(serial) = session.peek().as_ref().map(|s| s.serial) else {
            return;
        };
        session.with_mut(|s| *s = s.take().map(|s| s.leave(&app)));
        let settle = settle(Anim::RowOut, *self.level.peek());
        spawn(async move {
            sleep(settle).await;
            let mut session = self.session;
            if session.peek().as_ref().map(|s| s.serial) != Some(serial) {
                return;
            }
            session.with_mut(|s| *s = s.take().map(|s| s.gone(&app)));
            machine.send(SwIn::Elapsed);
        });
    }
}

/// The compositor's apps now.
fn live(services: CopyValue<Services>) -> Vec<SwitcherApp> {
    let services = services.peek();
    let toplevels = services.windows.state.borrow().toplevels.clone();
    let index = services.apps.state.borrow().clone();
    switcher_apps(&toplevels, &index)
}

impl SwitcherCtx {
    /// One ask from the daemon.
    pub fn ask(self, serial: AskSerial, ask: SwitcherAsk) {
        match ask {
            SwitcherAsk::Chord(dir) => {
                if self.state.peek().phase() == Phase::Hidden {
                    let apps = self.live();
                    if apps.is_empty() {
                        return;
                    }
                    self.open(Session::live(serial, apps));
                }
                let (phase, hold) = (self.state.peek().phase(), *self.hold.peek());
                self.input(chord_input(phase, hold.chord, dir));
            }
            SwitcherAsk::Cancel => self.input(SwIn::Key(SwKey::Escape)),
            SwitcherAsk::Commit => self.input(SwIn::ModifierReleased),
            SwitcherAsk::Fake(ids) => self.fake(Session::made_up(serial, ids)),
        }
    }

    /// Open on made-up apps, shown at once: the chord, and the panel up without the show delay.
    fn fake(self, session: Session) {
        self.close();
        self.open(session);
        self.machine.send(SwIn::Chord(heading(Dir::Next)));
        let mut state = self.state;
        let armed = state.peek().phase();
        if let Phase::Armed { sel, .. } = armed {
            state.set(Opening::in_phase(Phase::Shown { sel }));
        }
    }

    /// Start an opening: its apps, and the surface mapped so it takes the keyboard.
    fn open(self, session: Session) {
        let (mut hold, mut pointer) = (self.hold, self.pointer);
        hold.set(Hold::default());
        pointer.set(Pointer::default());
        let mut stored = self.session;
        stored.set(Some(session));
        self.visibility(Visibility::Shown);
    }

    /// One input through the machine; back at `Hidden`, the opening closes at once.
    pub fn input(self, input: SwIn) {
        self.machine.send(input);
        if self.state.peek().phase() == Phase::Hidden {
            self.close();
        }
    }

    /// Back to nothing: no opening, the surface unmapped. The machine's timer follows its state.
    fn close(self) {
        let (mut state, mut session) = (self.state, self.session);
        if session.peek().is_some() {
            session.set(None);
            self.visibility(Visibility::Hidden);
        }
        if state.peek().phase() != Phase::Hidden {
            state.set(Opening::default());
        }
    }

    fn visibility(self, visibility: Visibility) {
        if let Some(surface) = self.surface.peek().as_ref() {
            surface.set_visibility(visibility);
        }
    }

    /// The compositor's apps now.
    fn live(self) -> Vec<SwitcherApp> {
        live(self.services)
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

    /// The pointer on a tile selects it once the pointer has moved (`pointer.rs`); a click
    /// activates it.
    pub fn hover(self, key: &AppKey) {
        let mut pointer = self.pointer;
        let (after, chosen) = pointer.peek().clone().entered(key.clone());
        pointer.set(after);
        self.choose(chosen);
    }

    /// The pointer seen at `at` on the surface.
    pub fn pointer_moved(self, at: At) {
        let mut pointer = self.pointer;
        let (after, chosen) = pointer.peek().clone().moved(at);
        pointer.set(after);
        self.choose(chosen);
    }

    fn choose(self, chosen: Option<AppKey>) {
        if let Some(index) = chosen.and_then(|key| self.index(&key)) {
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
