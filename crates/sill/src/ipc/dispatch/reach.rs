//! What a request can reach: the services, the outputs' surfaces, the links the surfaces follow
//! and the settings the daemon holds.

use std::path::Path;

use sill_settings::ShellFile;
use sill_surfaces::links::Links;
use sill_surfaces::services::Services;
use tokio::sync::watch;

use crate::outputs::Outputs;

/// Everything [`super::route`] may touch to answer one request.
#[derive(Debug)]
pub struct Reach<'a> {
    pub services: &'a Services,
    pub outputs: &'a Outputs,
    /// The shared links.
    pub links: &'a Links,
    /// The settings directory; `None` when neither `XDG_CONFIG_HOME` nor `HOME` is set.
    pub settings_dir: Option<&'a Path>,
    pub settings: &'a watch::Sender<ShellFile>,
}
