//! What a request can reach: the services, the outputs' surfaces, the links the surfaces follow
//! and the settings the daemon holds.

use ds_settings::{Store, UserStyle};
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
}
