//! What a request can reach: the services, the outputs' surfaces, the links the surfaces follow
//! and the settings the daemon holds.

use ds_settings::{Store, UserStyle};
use sill_model::desktop::DesktopCommand;
use sill_settings::ShellFile;
use sill_ui_kit::links::Links;
use sill_ui_kit::services::Services;
use tokio::sync::watch;

use crate::outputs::Outputs;

/// Everything [`super::route`] may touch to answer one request.
#[derive(Debug)]
pub struct Reach<'a> {
    pub services: &'a Services,
    pub outputs: &'a Outputs,
    /// The shared links.
    pub links: &'a Links,
    /// The settings file's store.
    pub store: &'a Store,
    pub settings: &'a watch::Sender<ShellFile>,
    pub user_style: &'a watch::Sender<UserStyle>,
    /// The desktop's own compositor, for the requests that ask it for something.
    pub desktop: DesktopLine<'a>,
}

/// Whether the desktop's own compositor took a command: it does not when the session runs
/// another compositor, or its arm has stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carried {
    /// The compositor has it.
    Yes,
    /// There is no compositor to give it to.
    No,
}

/// The way a request sends a [`DesktopCommand`] to the compositor.
pub struct DesktopLine<'a>(pub &'a dyn Fn(DesktopCommand) -> Carried);

impl DesktopLine<'_> {
    /// Send `command`.
    pub fn send(&self, command: DesktopCommand) -> Carried {
        (self.0)(command)
    }
}

impl std::fmt::Debug for DesktopLine<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopLine").finish_non_exhaustive()
    }
}
