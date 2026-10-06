//! The bridges from the settings to the services: a [`Policy`] says what one service should be
//! told for a settings file, and [`follow`] tells it at start and again whenever the answer
//! changes. The services that could touch the person's session stay passive until the first.

pub mod calendar;
pub mod clipboard;
pub mod companion;
pub mod displays;
mod follow;
pub mod idle;
pub mod keyboard;
pub mod notifications;
pub mod screenshots;
pub mod search;
pub mod voice;

pub use follow::follow;

use std::future::Future;

use sill_settings::ShellFile;
use tokio::sync::watch::error::RecvError;

/// What one service is told for a settings file.
pub trait Policy: Send + 'static {
    /// The service's command.
    type Command: Send + 'static;
    /// What the settings decide; a new value is sent, an equal one is not.
    type Out: PartialEq + Clone + Send + Sync + 'static;

    /// What `file` decides.
    fn of(&self, file: &ShellFile) -> Self::Out;

    /// The command that carries `out`.
    fn command(out: Self::Out) -> Self::Command;

    /// Waits until something besides the settings changes what [`Policy::of`] answers (the
    /// focus mode for notifications); an error once that source is gone. Most policies read the
    /// settings alone and never wake here.
    fn changed(&mut self) -> impl Future<Output = Result<(), RecvError>> + Send {
        std::future::pending()
    }
}
