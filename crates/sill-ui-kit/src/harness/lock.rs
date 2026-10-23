//! [`LockHarness`]: a [`LockSurface`] mounted in a `ds_harness::Harness` outside a lock document
//! (no shell-host `LockHandle`, as in every test), on the fake services, a pinned appearance and
//! the virtual clock, with the surface's link in the test's hand.

use std::fmt;
use std::rc::Rc;

use crate::settings::SettingsFeed;
use dioxus::prelude::*;
use ds::style::kit::Styles;
use ds_harness::{Clock, Harness, HarnessConfig, Viewport};
use shell_host::OutputInfo;
use sill_settings::ShellFile;
use tokio::sync::watch;

use super::fake;
use super::surface::{Build, WORLD, app};
use crate::hooks::{PinnedAppearance, PinnedLocale};
use crate::links::LockLink;
use crate::services::Services;
use crate::surface::{LockSurface, MountLockProps, mount_lock};

/// A running lock surface and what it talks to.
pub struct LockHarness {
    /// The document: `advance`, `html`, `count`, ...
    pub harness: Harness,
    /// The fake services' senders.
    pub feeds: fake::Feeds,
    /// The surface mounted.
    surface: LockSurface,
    /// The surface's link, the daemon's end.
    pub link: LockLink,
    /// The styles the surface follows: send new ones to restyle it live.
    pub styles: watch::Sender<Styles>,
}

impl fmt::Debug for LockHarness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LockHarness")
            .field("surface", &self.surface.name)
            .field("harness", &self.harness)
            .finish_non_exhaustive()
    }
}

/// How a [`LockHarness`] starts: the default settings on a fixed 1920 x 1080 output, whose size
/// is the viewport.
pub struct LockSetup {
    surface: LockSurface,
    link: LockLink,
    settings: ShellFile,
    output: OutputInfo,
    appearance: PinnedAppearance,
    services: Box<dyn FnOnce(&mut Services)>,
}

impl fmt::Debug for LockSetup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LockSetup")
            .field("surface", &self.surface.name)
            .finish_non_exhaustive()
    }
}

impl LockHarness {
    /// The setup of `surface` on `link`.
    pub fn setup(surface: LockSurface, link: LockLink) -> LockSetup {
        LockSetup {
            surface,
            link,
            settings: ShellFile::default(),
            output: fake::output(),
            appearance: PinnedAppearance::default(),
            services: Box::new(|_| {}),
        }
    }
}

impl LockSetup {
    /// The settings file the surface reads.
    pub fn settings(mut self, settings: ShellFile) -> Self {
        self.settings = settings;
        self
    }

    /// An appearance other than the default one.
    pub fn appearance(mut self, appearance: PinnedAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Change the fake services before the surface sees them (a state the feeds do not carry).
    pub fn services(mut self, change: impl FnOnce(&mut Services) + 'static) -> Self {
        self.services = Box::new(change);
        self
    }

    /// Mount the surface and draw its first frame.
    pub fn start(self) -> LockHarness {
        let LockSetup {
            surface,
            link,
            settings,
            output,
            appearance,
            services: change,
        } = self;
        let viewport = Viewport {
            width: output.logical.width.unsigned_abs(),
            height: output.logical.height.unsigned_abs(),
            scale_percent: 100,
        };
        let (mut services, feeds) = fake::services();
        change(&mut services);
        let (styles, styles_rx) = watch::channel(Styles::default());
        let feed = SettingsFeed {
            styles: styles_rx,
            ..SettingsFeed::fixed(settings)
        };
        let context_link = link.clone();
        let build: Build = Rc::new(move || {
            use_context_provider(|| services.clone());
            // Times read the same on every machine: the C locale, 24-hour.
            use_context_provider(|| PinnedLocale("C".to_owned()));
            use_context_provider(|| feed.clone());
            use_context_provider(|| appearance.clone());
            use_context_provider(|| context_link.clone());
            mount_lock(MountLockProps::new(surface, output.clone()))
        });
        WORLD.with(|world| *world.borrow_mut() = Some(build));
        let harness = Harness::new(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
        LockHarness {
            harness,
            feeds,
            surface,
            link,
            styles,
        }
    }
}
