//! The lock screen as a [`LockSurface`]: one document per output, made by shell-host's session lock.

use dioxus::prelude::Element;
use shell_host::{LockConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::config;
use super::view::lock;
use sill_ui_kit::links::LockLink;
use sill_ui_kit::surface::LockSurface;

/// sill's own lock screen: the clock on every output, the password prompt on one.
#[derive(Debug)]
pub enum Lock {}

impl LockSurface for Lock {
    const NAME: &'static str = "lock";
    type Link = LockLink;

    fn config(settings: &ShellFile) -> LockConfig {
        config(settings)
    }

    fn root(output: OutputInfo) -> Element {
        lock(output)
    }
}
