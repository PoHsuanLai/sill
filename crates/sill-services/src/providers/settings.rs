//! Settings pages and keys: quire schemas (design/22 §9.3 "deep links") and the
//! `cosmic-settings` page table when that binary exists.

#![allow(unused_variables, dead_code)] // Freeze stubs

use sill_launcher::{Activation, Choice, Item, Provider, ProviderKind, Query, ResultSink};

/// Rows for settings pages and keys matching the query.
#[derive(Debug, Clone, Default)]
pub struct SettingsProvider {
    /// `(page id, title, keywords)` of `cosmic-settings` pages; empty when it is not installed.
    pub cosmic_pages: Vec<(String, String, Vec<String>)>,
}

impl Provider for SettingsProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Settings
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        todo!("M4 providers-b")
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        todo!("M4 providers-b: OpenSettings")
    }
}
