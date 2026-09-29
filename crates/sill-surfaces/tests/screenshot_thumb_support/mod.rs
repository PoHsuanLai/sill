//! The real thumbnail in a `ds_native::Harness` (a Blitz document, no window) on the fake
//! services, showing on the test output through a `BannerLink` the test holds, fed the shots
//! the screenshots service would publish, and reading back what it tells the service.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../support/fake.rs"]
pub mod fake;

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

use dioxus::prelude::*;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_model::screenshots::{Shot, ShotCommand, ShotId, ShotPreview, Shots};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::ScreenshotThumb;
use sill_surfaces::hooks::PinnedAppearance;
use sill_surfaces::services::Services;
use sill_surfaces::surfaces::banners::{BannerLink, BannerOutput};
use sill_surfaces::surfaces::screenshot_thumb::layer::thumb_layer;

/// The services' own fixture screenshot, drawn as a ready preview.
pub const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../sill-services/tests/fixtures/shots/Screenshot_fixture.png"
);

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
    // Not the desktop's portal and appearance file: they answer off-thread.
    use_context_provider(PinnedAppearance::default);
    use_context_provider(|| world.settings.clone());
    use_context_provider(|| world.link.clone());
    rsx! {
        ScreenshotThumb { output: fake::output() }
    }
}

/// A running thumbnail and what it talks to.
pub struct Running {
    pub harness: Harness,
    pub feeds: fake::Feeds,
    pub link: BannerLink,
}

/// Whether a wait ended in what it waited for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Done {
    Yes,
    No,
}

impl Running {
    /// Publish shot `id` as current with `preview`, taken at this instant (the next `advance`
    /// runs the woken surface first).
    pub fn shot(&mut self, id: u64, preview: ShotPreview) {
        self.feeds.shots.send_replace(Shots {
            current: Some(Shot {
                id: ShotId(id),
                path: PathBuf::from(format!("/tmp/shots/Screenshot_{id}.png")),
                preview,
            }),
            last_id: ShotId(id),
            ..Shots::default()
        });
    }

    /// Publish shot `id` with the fixture as its ready preview.
    pub fn ready(&mut self, id: u64) {
        self.shot(id, ShotPreview::Ready(PathBuf::from(FIXTURE)));
    }

    /// The service dropping its current shot (it did, after a `Leave`).
    pub fn cleared(&mut self) {
        self.feeds.shots.send_replace(Shots::default());
    }

    /// Every command sent since the last call.
    pub fn commands(&mut self) -> Vec<ShotCommand> {
        std::iter::from_fn(|| self.feeds.shot_commands.try_recv().ok()).collect()
    }

    /// How many cards quire's stack draws (shown or playing out).
    pub fn cards(&self) -> usize {
        self.harness.count(".ds-shot")
    }

    /// How many cards (their own exit) or rows (after a swipe) are playing their exit.
    pub fn leaving(&self) -> usize {
        self.harness.count(".ds-shot[*|data-presence=leaving]")
            + self.harness.count(".ds-banner[*|data-presence=leaving]")
    }

    /// The card's centre.
    pub fn card(&self) -> ds::Point {
        self.harness.centre(".ds-shot-picture").expect("a card") // test-only
    }

    /// Advance in 20 ms steps until `done`, at most `limit` of the harness's clock.
    pub fn until(&mut self, limit: Duration, done: impl Fn(&Running) -> bool) -> Done {
        let end = self.harness.now() + limit;
        while self.harness.now() < end {
            self.harness.advance(Duration::from_millis(20));
            if done(self) {
                return Done::Yes;
            }
        }
        Done::No
    }

    /// Shot 1, ready, and its entrance played.
    pub fn one_up(&mut self) {
        self.ready(1);
        assert_eq!(
            self.until(Duration::from_millis(900), |r| r.cards() == 1),
            Done::Yes,
            "the card slides in"
        );
        self.harness.advance(Duration::from_millis(400));
    }
}

/// The thumbnail on `file`'s settings, shown on the test output, no shot yet; the viewport is
/// the surface's own size.
pub fn start(file: ShellFile) -> Running {
    let (services, feeds) = fake::services();
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file),
            link: link.clone(),
        });
    });
    let size = thumb_layer(&fake::output()).size;
    let px = |extent| match extent {
        shell_host::Extent::Px(px) => px,
        _ => 400,
    };
    let viewport = Viewport {
        width: px(size.width),
        height: px(size.height),
        scale_percent: 100,
    };
    Running {
        harness: Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual)),
        feeds,
        link,
    }
}
