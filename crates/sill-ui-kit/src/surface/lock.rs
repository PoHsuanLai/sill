//! The session lock's surface: one document per output, made by shell-host's session lock.

use dioxus::prelude::{Element, use_context_provider};
use shell_host::{HostRoot, LockConfig, OutputInfo};
use sill_settings::ShellFile;

use crate::links::Links;
use crate::root::{SurfaceName, use_host_signals};

/// A surface the compositor's session lock draws instead of a layer surface: sill has one, the
/// lock screen. Its link is [`crate::links::LockLink`].
#[derive(Debug, Clone, Copy)]
pub struct LockSurface {
    /// The surface's name, as [`super::Surface::NAME`].
    pub name: &'static str,
    /// How shell-host sets its surfaces up.
    pub config: fn(&ShellFile) -> LockConfig,
    /// The links its document reads by type, given to each root (the lock's `Surface::needs`).
    pub needs: fn(HostRoot, &Links) -> HostRoot,
    /// The document on `output`; it runs as a root component, so it may call hooks.
    pub root: fn(OutputInfo) -> Element,
}

/// What [`mount_lock`] is given: the lock surface and the output whose surface it draws.
#[derive(Debug, Clone)]
pub struct MountLockProps {
    pub surface: LockSurface,
    pub output: OutputInfo,
}

impl MountLockProps {
    /// The props for `surface` on `output`.
    pub fn new(surface: LockSurface, output: OutputInfo) -> MountLockProps {
        MountLockProps { surface, output }
    }
}

/// The root component of a lock surface: quire's document host and the seat's signals, then its
/// `root`, in the component's scope.
pub fn mount_lock(props: MountLockProps) -> Element {
    ds_blitz::provide_host();
    crate::root::provide_keymap();
    let name = props.surface.name;
    use_context_provider(|| SurfaceName(name));
    use_host_signals();
    (props.surface.root)(props.output)
}
