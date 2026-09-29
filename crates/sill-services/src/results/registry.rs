//! The providers the daemon searches, in Tab order, sharing the runner's history and its
//! policy.

use std::sync::Arc;

use sill_launcher::Provider;
use tokio::sync::watch;

use super::ResultsLinks;
use super::history::History;
use super::policy::LauncherPolicy;
use crate::env::Env;
use crate::providers::files::IndexSource;
use crate::providers::{
    AppsProvider, CalculatorProvider, ClipboardProvider, EmojiProvider, FilesProvider,
    SettingsProvider, SystemProvider, WebProvider, WindowsProvider,
};

/// Every provider. Apps, files and emoji rank with `history`, so a choice counts from the next
/// keystroke on; the settings-gated providers read `policy` (all off until the daemon's first).
pub(crate) fn providers(
    links: &ResultsLinks,
    env: &Env,
    history: &History,
    policy: &watch::Receiver<LauncherPolicy>,
) -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(AppsProvider {
            apps: links.apps.clone(),
            frecency: history.shared(),
        }),
        Arc::new(WindowsProvider {
            cosmic_wl: links.cosmic_wl.clone(),
            apps: links.apps.clone(),
        }),
        Arc::new(CalculatorProvider {
            policy: policy.clone(),
            rates_file: crate::providers::calculator::fetch::cache_path(&env.dirs),
            ..CalculatorProvider::default()
        }),
        Arc::new(SettingsProvider::new(&env.dirs.data)),
        Arc::new(SystemProvider::new(env.buses.system.clone())),
        Arc::new(FilesProvider {
            policy: policy.clone(),
            frecency: history.shared(),
            source: IndexSource::Detect(env.buses.session.clone()),
            home: env.dirs.home.clone(),
        }),
        Arc::new(ClipboardProvider {
            history: links.clipboard.clone(),
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
