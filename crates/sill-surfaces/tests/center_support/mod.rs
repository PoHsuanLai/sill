//! The real `NotificationCenter` in a `ds_native::Harness` (a Blitz document, no window) on the
//! fake services, following a `CenterLink` the test writes, as the daemon's surfaces do. The
//! history is fed through the fake's `watch` channel and the commands the center sends are read
//! back from its queue: nothing here reaches a bus. On the virtual clock: every
//! timer and every frame's CSS move only with the test's `advance`.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../support/fake.rs"]
pub mod fake;

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds_native::harness::settle_until;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_services::{
    AppGroup, AppKey, AppRef, Body, Expiry, Hints, History, IconRef, Notification, NotificationId,
    NotificationsCommand, Services, UnixMs,
};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::NotificationCenter;
use sill_surfaces::hooks::PinnedAppearance;
use sill_surfaces::surfaces::notification_center::CenterLink;

/// What the harness's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    link: CenterLink,
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
    // Not the desktop's portal and appearance file: they answer off-thread.
    use_context_provider(PinnedAppearance::default);
    use_context_provider(|| world.settings.clone());
    use_context_provider(|| world.link.clone());
    rsx! {
        NotificationCenter { output: fake::output() }
    }
}

/// The viewport's size, as the rects report it.
pub const WIDTH: f32 = 1280.0;
pub const HEIGHT: f32 = 800.0;

pub const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

/// A running center and what it talks to.
pub struct Running {
    pub harness: Harness,
    pub feeds: fake::Feeds,
    pub link: CenterLink,
}

impl Running {
    /// Every command the center has sent since the last call.
    pub fn commands(&mut self) -> Vec<NotificationsCommand> {
        std::iter::from_fn(|| self.feeds.notification_commands.try_recv().ok()).collect()
    }

    /// Let `time` pass.
    pub fn advance(&mut self, ms: u64) {
        self.harness.advance(Duration::from_millis(ms));
    }

    /// Advance until `done` holds ([`settle`]): what a click test waits on.
    pub fn until(&mut self, done: impl Fn(&Harness) -> bool) {
        settle(&mut self.harness, done);
    }

    /// The commands sent, advancing until `count` have come, at most [`STEPS`] frames.
    pub fn commands_until(&mut self, count: usize) -> Vec<NotificationsCommand> {
        let mut sent = self.commands();
        for _ in 0..STEPS {
            if sent.len() >= count {
                break;
            }
            self.harness.advance(Duration::from_millis(20));
            sent.extend(self.commands());
        }
        sent
    }
}

/// The center on `file`'s settings over `history`, closed.
pub fn start(file: ShellFile, history: History) -> Running {
    let (services, feeds) = fake::services();
    feeds.notifications.send_modify(|n| n.history = history);
    let link = CenterLink::new();
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file),
            link: link.clone(),
        });
    });
    let harness =
        Harness::with_config(app, HarnessConfig::new(VIEWPORT).with_clock(Clock::Virtual));
    Running {
        harness,
        feeds,
        link,
    }
}

/// The center opened over `history`, once its entrance has played.
pub fn opened(history: History) -> Running {
    let mut running = start(ShellFile::default(), history);
    running.advance(40);
    running.link.open(fake::output().id);
    settle(&mut running.harness, |h| h.count(".ds-panel") == 1);
    running.advance(600);
    running
}

/// Advance until `done` holds (`settle_until`, which fails with the document if it never does);
/// then one more frame's worth.
pub fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    settle_until(harness, done);
    harness.advance(Duration::from_millis(20));
}

/// The most 20 ms frames a wait advances: four seconds of the harness's clock.
const STEPS: usize = 200;

/// A notification from `app`.
pub fn note(id: u32, app: &str, summary: &str) -> Notification {
    Notification {
        id: NotificationId(id),
        arrived: UnixMs(1_000 * i64::from(id)),
        app: AppRef {
            name: app.to_owned(),
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary: summary.to_owned(),
        body: Body::default(),
        actions: Vec::new(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    }
}

/// Three from Mail (ids 5, 3, 1, newest first) and one from Chat (4), Mail's group first.
pub fn two_apps() -> History {
    History {
        groups: vec![
            AppGroup {
                key: AppKey("mail".to_owned()),
                notes: vec![
                    note(5, "Mail", "Lunch?"),
                    note(3, "Mail", "Re: plans"),
                    note(1, "Mail", "Hello"),
                ],
            },
            AppGroup {
                key: AppKey("chat".to_owned()),
                notes: vec![note(4, "Chat", "ping")],
            },
        ],
    }
}

/// The one row (card or group header) that carries the keyboard's ring, as its card's stack
/// size or `head`; `None` without a ring.
pub fn ringed(harness: &Harness) -> Option<String> {
    match harness.count(".sill-nc-row[*|data-focus=ring]") {
        1 => harness.attr(".sill-nc-row[*|data-focus=ring]", "data-stack"),
        _ => (harness.count(".sill-nc-group-head[*|data-focus=ring]") == 1)
            .then(|| "head".to_owned()),
    }
}
