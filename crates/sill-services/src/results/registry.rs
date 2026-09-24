//! The five providers the daemon searches, in Tab order, sharing the runner's history.

use std::sync::Arc;

use sill_launcher::Provider;
use tokio::sync::watch;

use super::history::History;
use crate::apps::AppIndex;
use crate::cosmic_wl::Toplevels;
use crate::providers::{
    AppsProvider, CalculatorProvider, SettingsProvider, SystemProvider, WindowsProvider,
};

/// Apps, windows, calculator, settings and system. The apps provider ranks with `history`, so
/// a choice counts from the next keystroke on.
pub(crate) fn providers(
    apps: &watch::Receiver<AppIndex>,
    toplevels: &watch::Receiver<Toplevels>,
    history: &History,
) -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(AppsProvider {
            apps: apps.clone(),
            frecency: history.shared(),
        }),
        Arc::new(WindowsProvider {
            toplevels: toplevels.clone(),
            apps: apps.clone(),
        }),
        Arc::new(CalculatorProvider),
        Arc::new(SettingsProvider::default()),
        Arc::new(SystemProvider),
    ]
}
