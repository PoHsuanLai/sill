//! Open windows, by title and app name.

#![allow(unused_variables, dead_code)] // Freeze stubs

use sill_launcher::{Activation, Choice, Item, Provider, ProviderKind, Query, ResultSink};
use tokio::sync::watch;

use crate::apps::AppIndex;
use crate::cosmic_wl::Toplevels;

/// Rows for every toplevel matching the query, most recent first on ties.
#[derive(Debug)]
pub struct WindowsProvider {
    pub toplevels: watch::Receiver<Toplevels>,
    pub apps: watch::Receiver<AppIndex>,
}

impl Provider for WindowsProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Windows
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        todo!("M4 providers-a")
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        todo!("M4 providers-a: FocusWindow")
    }
}
