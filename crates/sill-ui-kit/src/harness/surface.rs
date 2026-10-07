//! [`SurfaceHarness`]: any [`Surface`] mounted in a `ds_harness::Harness` the way the daemon
//! mounts it, on the fake services, a pinned appearance and the virtual clock, with the
//! surface's link in the test's hand.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::settings::SettingsFeed;
use dioxus::prelude::*;
use ds_blitz::FocusFallback;
use ds_harness::{Clock, Harness, HarnessConfig, Viewport};
use ds_settings::{Environment, UserStyle};
use shell_host::{LayerExtent, OutputInfo};
use sill_settings::ShellFile;
use tokio::sync::watch;

use super::fake;
use crate::hooks::{PinnedAppearance, SwitchableAppearance};
use crate::services::Services;
use crate::surface::{MountProps, Surface, mount};

/// Builds the surface's document; the harness's root runs it on every render.
pub(super) type Build = Rc<dyn Fn() -> Element>;

thread_local! {
    /// The document the thread's live harness draws. One harness is live per thread (a test
    /// runs on its own thread), so a second one started while the first is still driven would
    /// replace what the first draws.
    pub(super) static WORLD: RefCell<Option<Build>> = const { RefCell::new(None) };
}

thread_local! {
    /// The signal behind the live harness's appearance (see [`SurfaceHarness::set_appearance`]).
    static APPEARANCE: RefCell<Option<Signal<Environment>>> = const { RefCell::new(None) };
}

pub(super) fn app() -> Element {
    match WORLD.with(|world| world.borrow().clone()) {
        Some(build) => build(),
        None => rsx! {},
    }
}

/// A running surface and what it talks to. A field, not a `Deref`, so that
/// `run.harness.send(Input::pointer_move(inside(&run.harness)))` borrows as it reads.
pub struct SurfaceHarness<S: Surface> {
    /// The document: `advance`, `html`, `pointer_move`, `count`, ...
    pub harness: Harness,
    /// The fake services' senders: set a state to make the surface see it, read what it sent.
    pub feeds: fake::Feeds,
    /// The surface's link, the daemon's end.
    pub link: S::Link,
    /// The settings file the surface follows: send a new one to change a setting live.
    pub settings: watch::Sender<ShellFile>,
    /// The person's `style.css` the surface follows: send a new one to restyle it live.
    pub user_style: watch::Sender<UserStyle>,
    /// The size and scale the document draws at.
    pub(super) viewport: Viewport,
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
    services: Option<(Services, fake::Feeds)>,
    root_hook: Option<Rc<dyn Fn()>>,
    user_style: UserStyle,
    focus: FocusFallback,
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
    /// Change the appearance the surface follows while it runs, as the control center's theme
    /// control does, and draw what follows (the surface is not settled: `advance` it).
    pub fn set_appearance(&mut self, environment: Environment) {
        let signal = APPEARANCE.with(|slot| *slot.borrow());
        self.harness.within(|| {
            if let Some(mut signal) = signal {
                signal.set(environment);
            }
        });
    }

    /// The setup for `slot` of `S` (`()` for a surface with one) on `link`.
    pub fn setup(link: S::Link, slot: S::Slot) -> SurfaceSetup<S> {
        SurfaceSetup {
            link,
            slot,
            settings: ShellFile::default(),
            output: fake::output(),
            viewport: None,
            appearance: PinnedAppearance::default(),
            services: None,
            root_hook: None,
            user_style: UserStyle::default(),
            focus: FocusFallback::default(),
        }
    }
}

impl<S: Surface> SurfaceSetup<S> {
    /// An output other than [`fake::output`] (`DP-1`), as a second monitor's surface.
    pub fn output(mut self, output: OutputInfo) -> Self {
        self.output = output;
        self
    }

    /// The settings file the surface reads.
    pub fn settings(mut self, settings: ShellFile) -> Self {
        self.settings = settings;
        self
    }

    /// The person's `style.css` the surface starts with.
    pub fn user_style(mut self, text: &str) -> Self {
        self.user_style = UserStyle(text.to_owned());
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

    /// Services other than [`fake::services`], and the senders behind them: for a test that
    /// answers a service itself (the launcher's search runner).
    pub fn services(mut self, services: Services, feeds: fake::Feeds) -> Self {
        self.services = Some((services, feeds));
        self
    }

    /// A hook run in the root before the surface's own, on every render: for the extra contexts
    /// (`use_context_provider`) and tasks (`use_hook` around `spawn`) a test gives it.
    pub fn in_root(mut self, hook: impl Fn() + 'static) -> Self {
        self.root_hook = Some(Rc::new(hook));
        self
    }

    /// Where the keyboard goes when the element that had it leaves the document. The harness
    /// hands it on to an ancestor, as a window of quire's own does; `BlitzDefault` is what a
    /// shell-host surface has today, which wires no focus keeper.
    pub fn focus(mut self, focus: FocusFallback) -> Self {
        self.focus = focus;
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
            services,
            root_hook,
            user_style,
            focus,
        } = self;
        let viewport = viewport.unwrap_or_else(|| layer_viewport::<S>(&output, &settings, slot));
        let (services, feeds) = services.unwrap_or_else(fake::services);
        let (settings, settings_rx) = watch::channel(settings);
        let (user_style_tx, user_style_rx) = watch::channel(user_style);
        let feed = SettingsFeed {
            settings: settings_rx,
            user_style: user_style_rx,
        };
        let context_link = link.clone();
        let build: Build = Rc::new(move || {
            use_context_provider(|| services.clone());
            use_context_provider(|| feed.clone());
            let live =
                use_context_provider(|| SwitchableAppearance(Signal::new(appearance.0.clone())));
            APPEARANCE.with(|slot| *slot.borrow_mut() = Some(live.0));
            use_context_provider(|| context_link.clone());
            if let Some(hook) = &root_hook {
                hook();
            }
            mount::<S>(MountProps::new(output.clone(), slot))
        });
        WORLD.with(|world| *world.borrow_mut() = Some(build));
        let harness = Harness::new(
            app,
            HarnessConfig::new(viewport)
                .with_clock(Clock::Virtual)
                .with_focus_fallback(focus),
        );
        SurfaceHarness {
            harness,
            feeds,
            link,
            settings,
            user_style: user_style_tx,
            viewport,
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
