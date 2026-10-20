//! What a layer surface is, to the daemon that spawns it and the harness that drives it: the
//! [`Surface`] trait, where it goes on an output ([`Place`], [`OutputRank`]), whether it starts
//! mapped ([`Start`]), the session lock's twin ([`LockSurface`]) and the one component that
//! mounts any of them ([`mount`]).
//!
//! A surface's own component tree stays in its directory under `surfaces/`; this module only
//! names the seam. The seam supplies no `Ds` boilerplate of its own: a surface's `root` is its
//! view, which puts its material, chrome and extent on a [`crate::root::SurfaceRoot`].

mod activation;
mod lock;
mod mount;
mod naming;
mod place;
mod start;

pub use activation::{
    ACTIVATION_DEADLINE, SETTINGS_APP, activation_token, launch_with_token,
    open_settings_with_token,
};
pub use lock::{LockSurface, MountLockProps, mount_lock};
pub use mount::{MountProps, mount};
pub use naming::namespace;
pub use place::{OutputRank, Place};
pub use start::Start;

use std::fmt::Debug;
use std::hash::Hash;

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

/// One kind of layer surface: the OSD, the dock, a hot corner. The daemon spawns, resizes and
/// closes any `S: Surface` per output and per slot from these five answers, and
/// [`SurfaceHarness`](crate::harness::SurfaceHarness) mounts the same `root` in a test.
pub trait Surface: 'static {
    /// The surface's name: its layer namespace is `sill-<NAME>-<output>` (a slotted surface
    /// adds its slot), and the stylesheet selectors a person writes name it.
    const NAME: &'static str;
    /// The link the daemon or another surface drives it through, a type in [`crate::links`];
    /// `()` when it has none. The daemon gives every root its link as a root context.
    type Link: Clone + Send + Sync + 'static;
    /// Which of its surfaces on one output: `()` for one, a `Corner` for a hot corner.
    type Slot: Copy + Eq + Hash + Debug + 'static;
    /// The slots this output gets, from the settings and the output's place; empty is none.
    fn slots(place: &Place) -> Vec<Self::Slot>;
    /// The layer configuration of `slot` on `output`.
    fn layer(output: &OutputInfo, settings: &ShellFile, slot: Self::Slot) -> LayerConfig;
    /// Whether a new surface is mapped at once or waits to be shown.
    fn start() -> Start;
    /// The surface's document. Runs inside [`mount`], so it may call hooks.
    fn root(output: OutputInfo, slot: Self::Slot) -> Element;
}
