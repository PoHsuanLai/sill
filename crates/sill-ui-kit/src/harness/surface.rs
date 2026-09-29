//! [`SurfaceHarness`]: any [`Surface`] mounted in a `ds_native::Harness` the way the daemon
//! mounts it, on the fake services, a pinned appearance and the virtual clock, with the
//! surface's link in the test's hand.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use dioxus::prelude::*;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use shell_host::{LayerExtent, OutputInfo};
use sill_settings::{SettingsFeed, ShellFile};

use super::fake;
use crate::hooks::PinnedAppearance;
use crate::surface::{MountProps, Surface, mount};

/// Builds the surface's document; the harness's root runs it on every render.
type Build = Rc<dyn Fn() -> Element>;

thread_local! {
    /// The document the thread's live harness draws. One harness is live per thread (a test
    /// runs on its own thread), so a second one started while the first is still driven would
    /// replace what the first draws.
    static WORLD: RefCell<Option<Build>> = const { RefCell::new(None) };
}

fn app() -> Element {
    match WORLD.with(|world| world.borrow().clone()) {
        Some(build) => build(),
        None => rsx! {},
    }
}

/// A running surface and what it talks to. A field, not a `Deref`, so that
/// `run.harness.pointer_move(inside(&run.harness))` borrows as it reads.
pub struct SurfaceHarness<S: Surface> {
    /// The document: `advance`, `html`, `pointer_move`, `count`, ...
    pub harness: Harness,
    /// The fake services' senders: set a state to make the surface see it, read what it sent.
    pub feeds: fake::Feeds,
    /// The surface's link, the daemon's end.
    pub link: S::Link,
}

impl<S: Surface> fmt::Debug for SurfaceHarness<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfaceHarness")
            .field("surface", &S::NAME)
            .field("harness", &self.harness)
            .finish_non_exhaustive()
    }
}

/// How a [`SurfaceHarness`] starts; the defaults are the daemon's on a fixed 1920 x 1080 output:
/// the default settings, and a viewport of the surface's own layer size.
pub struct SurfaceSetup<S: Surface> {
    link: S::Link,
    slot: S::Slot,
    settings: ShellFile,
    output: OutputInfo,
    viewport: Option<Viewport>,
    appearance: PinnedAppearance,
}

impl<S: Surface> fmt::Debug for SurfaceSetup<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfaceSetup")
            .field("surface", &S::NAME)
            .field("slot", &self.slot)
            .finish_non_exhaustive()
    }
}

impl<S: Surface> SurfaceHarness<S> {
    /// The setup for `slot` of `S` (`()` for a surface with one) on `link`.
    pub fn setup(link: S::Link, slot: S::Slot) -> SurfaceSetup<S> {
        SurfaceSetup {
            link,
            slot,
            settings: ShellFile::default(),
            output: fake::output(),
            viewport: None,
            appearance: PinnedAppearance::default(),
        }
    }
}

impl<S: Surface> SurfaceSetup<S> {
    /// The settings file the surface reads.
    pub fn settings(mut self, settings: ShellFile) -> Self {
        self.settings = settings;
        self
    }

    /// A viewport other than the surface's layer size.
    pub fn viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = Some(viewport);
        self
    }

    /// An appearance other than the default one.
    pub fn appearance(mut self, appearance: PinnedAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Mount the surface and draw its first frame.
    pub fn start(self) -> SurfaceHarness<S> {
        let SurfaceSetup {
            link,
            slot,
            settings,
            output,
            viewport,
            appearance,
        } = self;
        let viewport = viewport.unwrap_or_else(|| layer_viewport::<S>(&output, &settings, slot));
        let (services, feeds) = fake::services();
        let feed = SettingsFeed::fixed(settings);
        let context_link = link.clone();
        let build: Build = Rc::new(move || {
            use_context_provider(|| services.clone());
            use_context_provider(|| feed.clone());
            use_context_provider(|| appearance.clone());
            use_context_provider(|| context_link.clone());
            mount::<S>(MountProps::new(output.clone(), slot))
        });
        WORLD.with(|world| *world.borrow_mut() = Some(build));
        let harness =
            Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
        SurfaceHarness {
            harness,
            feeds,
            link,
        }
    }
}

/// The viewport the surface's layer asks for on `output`; a stretched axis takes the output's.
fn layer_viewport<S: Surface>(
    output: &OutputInfo,
    settings: &ShellFile,
    slot: S::Slot,
) -> Viewport {
    let size = S::layer(output, settings, slot).size;
    let extent = |extent: LayerExtent, whole: i32| match extent {
        LayerExtent::Px(px) => px,
        LayerExtent::Fill => whole.unsigned_abs(),
    };
    Viewport {
        width: extent(size.width, output.logical.width),
        height: extent(size.height, output.logical.height),
        scale_percent: 100,
    }
}
