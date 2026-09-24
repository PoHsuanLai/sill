//! Installed applications, ranked by name, generic name and keywords with frecency.

#![allow(unused_variables, dead_code)] // Freeze stubs

use std::sync::{Arc, Mutex};

use sill_launcher::{
    Activation, Choice, Frecency, Item, Provider, ProviderKind, Query, ResultSink,
};
use tokio::sync::watch;

use crate::apps::AppIndex;

/// Rows for every listed app matching the query; its desktop actions are the row's actions.
#[derive(Debug)]
pub struct AppsProvider {
    pub apps: watch::Receiver<AppIndex>,
    pub frecency: Arc<Mutex<Frecency>>,
}

impl Provider for AppsProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Apps
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        todo!("M4 providers-a: Needle::parse, rank_text over name/generic/keywords, one batch")
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        todo!("M4 providers-a: Launch{{app, Default | Named(action)}}")
    }
}
