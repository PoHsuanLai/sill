//! Session commands: lock, log out, suspend, restart, shut down, reload.

#![allow(unused_variables, dead_code)] // Freeze stubs

use sill_launcher::{Activation, Choice, Item, Provider, ProviderKind, Query, ResultSink};

/// Rows for the session commands whose names match the query.
#[derive(Debug, Clone, Default)]
pub struct SystemProvider;

impl Provider for SystemProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::System
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        todo!("M4 providers-a")
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        todo!("M4 providers-a: System(SystemCommand)")
    }
}
