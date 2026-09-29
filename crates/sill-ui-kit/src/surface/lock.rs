//! The session lock's surface: one document per output, made by shell-host's session lock.

use dioxus::prelude::Element;
use shell_host::{LockConfig, OutputInfo};
use sill_settings::ShellFile;

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
