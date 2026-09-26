//! The real `PolkitPrompt` surface in a `ds_native::Harness` (a Blitz document, no window) on
//! the fake services, with the agent's watch channel and command queue held by the test and the
//! banners' output set as the daemon sets it. Nothing here reaches polkit.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../support/fake.rs"]
pub mod fake;

use std::cell::RefCell;
use std::time::{Duration, Instant};

use dioxus::prelude::*;
use ds_native::{Harness, Viewport};
use shell_host::OutputId;
use sill_services::auth::UserName;
use sill_services::polkit_agent::{
    ActionId, Cookie, Echo, LastTry, Polkit, PolkitCommand, PromptPhase, PromptView, Registration,
};
use sill_services::{Control, Services};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::PolkitPrompt;
use sill_surfaces::surfaces::banners::{BannerLink, BannerOutput};
use tokio::sync::{mpsc, watch};

/// What the harness's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    link: BannerLink,
}

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(|| world.settings.clone());
    use_context_provider(|| world.link.clone());
    rsx! {
        PolkitPrompt { output: fake::output() }
    }
}

pub const WIDTH: f32 = 1280.0;
pub const HEIGHT: f32 = 800.0;

pub const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

/// A running prompt and what it talks to.
pub struct Running {
    pub harness: Harness,
    pub feeds: fake::Feeds,
    pub polkit: watch::Sender<Polkit>,
    pub commands: mpsc::UnboundedReceiver<PolkitCommand>,
    pub link: BannerLink,
}

impl Running {
    /// Show `view` as the agent's current request.
    pub fn show(&mut self, view: PromptView) {
        self.polkit.send_replace(Polkit {
            registration: Registration::Registered,
            current: Some(view),
            waiting: 0,
        });
    }

    /// The agent has no request.
    pub fn clear(&mut self) {
        self.polkit.send_replace(Polkit::default());
    }

    /// The commands the prompt has sent so far.
    pub fn sent(&mut self) -> Vec<PolkitCommand> {
        std::iter::from_fn(|| self.commands.try_recv().ok()).collect()
    }
}

/// The prompt, hidden, with the banners on `on` (the test output is `OutputId(1)`).
pub fn start(on: OutputId) -> Running {
    let (mut services, feeds) = fake::services();
    let (polkit, rx) = watch::channel(Polkit::default());
    let (control, commands) = Control::channel();
    services.polkit = rx;
    services.polkit_control = control;
    let link = BannerLink::new();
    link.set(BannerOutput::On(on));
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(ShellFile::default()),
            link: link.clone(),
        });
    });
    let harness = Harness::new(app, VIEWPORT);
    Running {
        harness,
        feeds,
        polkit,
        commands,
        link,
    }
}

/// A request asking for ada's password.
pub fn request(cookie: &str, phase: PromptPhase, last: LastTry) -> PromptView {
    PromptView {
        cookie: Cookie(cookie.to_owned()),
        action_id: ActionId("org.freedesktop.systemd1.manage-units".to_owned()),
        message: "Authentication is required to start 'sshd.service'.".to_owned(),
        icon: String::new(),
        details: vec![("unit".to_owned(), "sshd.service".to_owned())],
        user: UserName("ada".to_owned()),
        phase,
        last,
        notes: Vec::new(),
    }
}

/// The helper's password prompt.
pub fn asking() -> PromptPhase {
    PromptPhase::Asking {
        prompt: "Password: ".to_owned(),
        echo: Echo::Hidden,
    }
}

/// The prompt showing `view`, once its sheet has settled.
pub fn shown(view: PromptView) -> Running {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(40));
    running.show(view);
    settle(&mut running.harness, |h| h.count(".ds-polkit") == 1);
    running.harness.advance(Duration::from_millis(600));
    running
}

/// Advance until `done` holds, at most two seconds; then one more frame's worth.
pub fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    let until = Instant::now() + Duration::from_secs(2);
    while !done(harness) && Instant::now() < until {
        harness.advance(Duration::from_millis(20));
    }
    harness.advance(Duration::from_millis(20));
}
