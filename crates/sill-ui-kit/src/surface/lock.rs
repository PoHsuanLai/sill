//! The session lock's surface: one document per output, made by shell-host's session lock.

use std::fmt;
use std::marker::PhantomData;

use dioxus::prelude::{Element, use_context_provider};
use shell_host::{LockConfig, OutputInfo};
use sill_settings::ShellFile;

use crate::root::{SurfaceName, use_host_signals};

/// A surface the compositor's session lock draws instead of a layer surface.
pub trait LockSurface: 'static {
    /// The surface's name, as [`super::Surface::NAME`].
    const NAME: &'static str;
    /// The link that drives it, as [`super::Surface::Link`].
    type Link: Clone + Send + Sync + 'static;
    /// How shell-host sets its surfaces up.
    fn config(settings: &ShellFile) -> LockConfig;
    /// The document on `output`; it runs as a root component, so it may call hooks.
    fn root(output: OutputInfo) -> Element;
}

/// What [`mount_lock`] is given: the output whose lock surface it draws.
pub struct MountLockProps<S: LockSurface> {
    pub output: OutputInfo,
    surface: PhantomData<fn() -> S>,
}

impl<S: LockSurface> MountLockProps<S> {
    /// The props for the lock surface of `S` on `output`.
    pub fn new(output: OutputInfo) -> MountLockProps<S> {
        MountLockProps {
            output,
            surface: PhantomData,
        }
    }
}

// By hand: a derive would ask `S` itself to be `Clone`.
impl<S: LockSurface> Clone for MountLockProps<S> {
    fn clone(&self) -> Self {
        MountLockProps::new(self.output.clone())
    }
}

// By hand: a derive would ask `S` itself to be `Debug`.
impl<S: LockSurface> fmt::Debug for MountLockProps<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MountLockProps")
            .field("surface", &S::NAME)
            .field("output", &self.output)
            .finish()
    }
}

/// The root component of a lock surface `S`: quire's document host and the seat's signals, then
/// its `root`, in the component's scope.
pub fn mount_lock<S: LockSurface>(props: MountLockProps<S>) -> Element {
    ds_blitz::provide_host();
    crate::root::provide_keymap();
    use_context_provider(|| SurfaceName(S::NAME));
    use_host_signals();
    S::root(props.output)
}
