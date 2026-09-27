//! The real banner column in a `ds_native::Harness` (a Blitz document, no window) on the fake
//! services, showing on the test output through a `BannerLink` the test holds, fed live banners
//! as the notifications service publishes them, and reading back the commands it sends.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../support/fake.rs"]
pub mod fake;

use std::cell::RefCell;

use dioxus::prelude::*;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_services::{
    Action, ActionKey, AppRef, BannerLife, Body, Expiry, Hints, IconRef, LiveBanner, Notification,
    NotificationId, Notifications, NotificationsCommand, Services, UnixMs,
};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::Banners;
use sill_surfaces::surfaces::banners::{BannerLink, BannerOutput, ROOM};

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
        Banners { output: fake::output() }
    }
}

/// A running column and what it talks to.
pub struct Running {
    pub harness: Harness,
    pub feeds: fake::Feeds,
    pub link: BannerLink,
}

impl Running {
    /// Publish `banners` (newest first) as the service would, taken at this instant (the next
    /// `advance` runs the woken column first, quire G311).
    pub fn show(&mut self, banners: Vec<LiveBanner>) {
        self.feeds.notifications.send_replace(Notifications {
            banners,
            ..Notifications::default()
        });
    }

    /// Every command sent since the last call.
    pub fn commands(&mut self) -> Vec<NotificationsCommand> {
        std::iter::from_fn(|| self.feeds.notification_commands.try_recv().ok()).collect()
    }

    /// How many cards quire's stack draws (listed or playing out).
    pub fn cards(&self) -> usize {
        self.harness.count(".ds-banner")
    }

    /// How many of them are playing their exit.
    pub fn leaving(&self) -> usize {
        self.harness.count(".ds-banner[*|data-presence=leaving]")
    }

    /// The first card's `data-hover` (`on` under the pointer).
    pub fn hover(&self) -> Option<String> {
        self.harness.attr(".ds-notification", "data-hover")
    }

    /// The first card's plate: where a press opens it.
    pub fn plate(&self) -> ds::Point {
        self.harness
            .centre(".ds-notification-plate")
            .expect("a card") // test-only
    }

    /// Advance in 20 ms steps until `done`, at most `limit` of the harness's clock; whether it
    /// happened.
    pub fn until(&mut self, limit: std::time::Duration, done: impl Fn(&Running) -> bool) -> Done {
        let end = self.harness.now() + limit;
        while self.harness.now() < end {
            self.harness.advance(std::time::Duration::from_millis(20));
            if done(self) {
                return Done::Yes;
            }
        }
        Done::No
    }
}

/// Whether a wait ended in what it waited for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Done {
    Yes,
    No,
}

/// The column on `file`'s settings, shown on the test output, nothing posted yet. The viewport
/// is the surface's width and a tall column.
pub fn start(file: ShellFile) -> Running {
    let (services, feeds) = fake::services();
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file.clone()),
            link: link.clone(),
        });
    });
    let viewport = Viewport {
        width: u32::from(file.notifications.banner_width_px.0) + 2 * ROOM,
        height: 700,
        scale_percent: 100,
    };
    Running {
        harness: Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual)),
        feeds,
        link,
    }
}

/// A notification `id` from `app` with `summary`, `body` and `actions` (`key=label`).
pub fn note(
    id: u32,
    app: &str,
    summary: &str,
    body: &str,
    actions: &[(&str, &str)],
) -> Notification {
    Notification {
        id: NotificationId(id),
        arrived: UnixMs(sill_surfaces::surfaces::notification_card::wall_now().0),
        app: AppRef {
            name: app.to_owned(),
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary: summary.to_owned(),
        body: Body::plain(body),
        actions: actions
            .iter()
            .map(|(key, label)| Action {
                key: ActionKey((*key).to_owned()),
                label: (*label).to_owned(),
            })
            .collect(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    }
}

/// `note` as a banner that expires.
pub fn live(note: Notification) -> LiveBanner {
    LiveBanner {
        note,
        life: BannerLife::Expires,
    }
}
