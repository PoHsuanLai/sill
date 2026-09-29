//! [`LockHarness`]: any [`LockSurface`] mounted in a `ds_native::Harness` outside a lock document
//! (no shell-host `LockHandle`, as in every test), on the fake services, a pinned appearance and
//! the virtual clock, with the surface's link in the test's hand.

use std::fmt;
use std::rc::Rc;

use dioxus::prelude::*;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use shell_host::OutputInfo;
use sill_settings::{SettingsFeed, ShellFile};

use super::fake;
use super::surface::{Build, WORLD, app};
use crate::hooks::PinnedAppearance;
use crate::services::Services;
use crate::surface::{LockSurface, MountLockProps, mount_lock};

/// A running lock surface and what it talks to.
pub struct LockHarness<S: LockSurface> {
    /// The document: `advance`, `html`, `count`, ...
    pub harness: Harness,
    /// The fake services' senders.
    pub feeds: fake::Feeds,
    /// The surface's link, the daemon's end.
    pub link: S::Link,
}

impl<S: LockSurface> fmt::Debug for LockHarness<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LockHarness")
            .field("surface", &S::NAME)
            .field("harness", &self.harness)
            .finish_non_exhaustive()
    }
}

/// How a [`LockHarness`] starts: the default settings on a fixed 1920 x 1080 output, whose size
/// is the viewport.
pub struct LockSetup<S: LockSurface> {
    link: S::Link,
    settings: ShellFile,
    output: OutputInfo,
    appearance: PinnedAppearance,
    services: Box<dyn FnOnce(&mut Services)>,
}

impl<S: LockSurface> fmt::Debug for LockSetup<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LockSetup")
            .field("surface", &S::NAME)
            .finish_non_exhaustive()
    }
}

impl<S: LockSurface> LockHarness<S> {
    /// The setup of `S` on `link`.
    pub fn setup(link: S::Link) -> LockSetup<S> {
        LockSetup {
            link,
            settings: ShellFile::default(),
            output: fake::output(),
            appearance: PinnedAppearance::default(),
            services: Box::new(|_| {}),
        }
    }
}

impl<S: LockSurface> LockSetup<S> {
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
    pub fn start(self) -> LockHarness<S> {
        let LockSetup {
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
        let feed = SettingsFeed::fixed(settings);
        let context_link = link.clone();
        let build: Build = Rc::new(move || {
            use_context_provider(|| services.clone());
            use_context_provider(|| feed.clone());
            use_context_provider(|| appearance.clone());
            use_context_provider(|| context_link.clone());
            mount_lock::<S>(MountLockProps::new(output.clone()))
        });
        WORLD.with(|world| *world.borrow_mut() = Some(build));
        let harness =
            Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
        LockHarness {
            harness,
            feeds,
            link,
        }
    }
}
