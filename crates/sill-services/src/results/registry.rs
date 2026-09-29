//! The providers the daemon searches, in Tab order, sharing the runner's history and its
//! policy.

use std::path::PathBuf;
use std::sync::Arc;

use sill_launcher::Provider;
use tokio::sync::watch;

use super::Wiring;
use super::history::History;
use super::policy::LauncherPolicy;
use crate::providers::files::IndexSource;
use crate::providers::{
    AppsProvider, CalculatorProvider, ClipboardProvider, EmojiProvider, FilesProvider,
    SettingsProvider, SystemProvider, WebProvider, WindowsProvider,
};

/// Every provider. Apps, files and emoji rank with `history`, so a choice counts from the next
/// keystroke on; the settings-gated providers read `policy` (all off until the daemon's first).
pub(crate) fn providers(
    wiring: &Wiring<'_>,
    history: &History,
    policy: &watch::Receiver<LauncherPolicy>,
) -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(AppsProvider {
            apps: wiring.apps.clone(),
            frecency: history.shared(),
        }),
        Arc::new(WindowsProvider {
            toplevels: wiring.toplevels.clone(),
            apps: wiring.apps.clone(),
            workspaces: wiring.workspaces.clone(),
        }),
        Arc::new(CalculatorProvider {
            policy: policy.clone(),
            ..CalculatorProvider::default()
        }),
        Arc::new(SettingsProvider::new()),
        Arc::new(SystemProvider::new()),
        Arc::new(FilesProvider {
            policy: policy.clone(),
            frecency: history.shared(),
            source: IndexSource::Detect,
            home: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        }),
        Arc::new(ClipboardProvider {
            history: wiring.clipboard.clone(),
        }),
        Arc::new(EmojiProvider {
            policy: policy.clone(),
            frecency: history.shared(),
        }),
        Arc::new(WebProvider {
            policy: policy.clone(),
        }),
    ]
}
