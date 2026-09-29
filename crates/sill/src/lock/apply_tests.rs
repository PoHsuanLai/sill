//! `apply` and `on_host_event` against a fake host: nothing here locks a session.

use shell_host::{
    HostEvent, KeyboardFocus, LockId, LockState, OutputId, OutputInfo, Rect, Scale, SurfaceId,
};
use sill_ui_kit::links::{LockLink, PromptOutput};

use super::host::{LockFactory, LockRefusal, SessionLockHost};
use super::{LockWish, ShellLock, apply, on_host_event};
use crate::daemon::msg::ShellMsg;
use crate::outputs::{OutputSurfaces, Outputs};

/// What the fake host was asked, and what it answers.
#[derive(Debug, Default)]
struct FakeHost {
    answer: Option<LockRefusal>,
    locks: Vec<LockId>,
    unlocks: Vec<LockId>,
}

impl SessionLockHost for FakeHost {
    fn lock_session(&mut self, _factory: LockFactory) -> Result<LockId, LockRefusal> {
        let id = LockId(7);
        self.locks.push(id);
        self.answer.map_or(Ok(id), Err)
    }

    fn unlock_session(&mut self, id: LockId) {
        self.unlocks.push(id);
    }

    fn lock_surface_output(&self, id: LockId, surface: SurfaceId) -> Option<OutputId> {
        (id == LockId(7) && surface == SurfaceId(22)).then_some(OutputId(2))
    }
}

fn output(id: u32, name: &str) -> OutputInfo {
    OutputInfo {
        id: OutputId(id),
        name: Some(name.to_owned()),
        description: None,
        logical: Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
        scale: Scale::ONE,
        refresh_mhz: 60_000,
    }
}

fn outputs() -> Outputs {
    Outputs {
        outputs: vec![
            (output(1, "DP-1"), OutputSurfaces::default()),
            (output(2, "DP-2"), OutputSurfaces::default()),
        ],
    }
}

fn never_built() -> LockFactory {
    LockFactory(Box::new(|_| panic!("the fake host builds no surface")))
}

/// A folder of this test's own, removed when dropped.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("sill-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        Scratch(dir)
    }

    /// Nothing reaches the real logind, UPower, session bus, compositor or files.
    fn env(&self) -> sill_services::env::Env {
        sill_services::env::Env::isolated(&self.0)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_wish_takes_and_drops_the_lock_once() {
    let rt = runtime();
    let scratch = Scratch::new("lock-wish");
    let services = crate::daemon::services::start(rt.handle(), &scratch.env());
    let (mut host, mut held, link) = (FakeHost::default(), ShellLock::default(), LockLink::new());
    let outputs = outputs();
    let wish = |host: &mut FakeHost, held: &mut ShellLock, wish| {
        apply(host, held, wish, never_built, &link, &outputs, &services);
    };
    wish(&mut host, &mut held, LockWish::Held);
    wish(&mut host, &mut held, LockWish::Held);
    assert_eq!(host.locks, vec![LockId(7)], "held once");
    assert_eq!(held.id, Some(LockId(7)));
    assert_eq!(
        link.now(),
        PromptOutput::On(OutputId(1)),
        "the first output"
    );
    wish(&mut host, &mut held, LockWish::Released);
    assert_eq!(host.unlocks, vec![LockId(7)]);

    // The host's word ends it; the keyboard moves the prompt only while it holds.
    let event = |state| HostEvent::<ShellMsg>::Lock {
        id: LockId(7),
        state,
    };
    let focus = HostEvent::<ShellMsg>::KeyboardFocus {
        id: SurfaceId(22),
        focus: KeyboardFocus::Gained,
    };
    on_host_event(
        &host,
        &mut held,
        &event(LockState::Locked),
        &link,
        &services,
    );
    on_host_event(&host, &mut held, &focus, &link, &services);
    assert_eq!(
        link.now(),
        PromptOutput::On(OutputId(2)),
        "follows the keyboard"
    );
    on_host_event(
        &host,
        &mut held,
        &event(LockState::Unlocked),
        &link,
        &services,
    );
    assert_eq!(held.id, None);
    link.set(PromptOutput::On(OutputId(1)));
    on_host_event(&host, &mut held, &focus, &link, &services);
    assert_eq!(
        link.now(),
        PromptOutput::On(OutputId(1)),
        "no lock, no move"
    );
    wish(&mut host, &mut held, LockWish::Released);
    assert_eq!(
        host.unlocks,
        vec![LockId(7)],
        "nothing held, nothing unlocked"
    );
}

#[test]
fn a_refusal_is_not_held_and_a_lock_in_progress_is_adopted() {
    let rt = runtime();
    let scratch = Scratch::new("lock-refusal");
    let services = crate::daemon::services::start(rt.handle(), &scratch.env());
    let link = LockLink::new();
    let outputs = outputs();
    let mut refused = FakeHost {
        answer: Some(LockRefusal::Unsupported),
        ..FakeHost::default()
    };
    let mut held = ShellLock::default();
    apply(
        &mut refused,
        &mut held,
        LockWish::Held,
        never_built,
        &link,
        &outputs,
        &services,
    );
    assert_eq!(held.id, None);
    let mut busy = FakeHost {
        answer: Some(LockRefusal::InProgress(LockId(3))),
        ..FakeHost::default()
    };
    apply(
        &mut busy,
        &mut held,
        LockWish::Held,
        never_built,
        &link,
        &outputs,
        &services,
    );
    assert_eq!(held.id, Some(LockId(3)));
}
