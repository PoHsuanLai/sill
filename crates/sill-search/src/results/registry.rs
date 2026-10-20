//! The providers the daemon searches, in Tab order, sharing the runner's history and its
//! policy.

use std::sync::Arc;

use sill_launcher::Provider;
use sill_model::results::LauncherPolicy;
use tokio::sync::watch;

use super::ResultsLinks;
use super::history::History;
use crate::providers::files::IndexSource;
use crate::providers::{
    AppsProvider, CalculatorProvider, ClipboardProvider, CompanionProvider, EmojiProvider,
    FilesProvider, HelpersProvider, MemoryProvider, RunsProvider, SettingsProvider, SystemProvider,
    ThingsProvider, WebProvider, WindowsProvider,
};
use sill_services::env::Env;

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
            windows: links.windows.clone(),
            apps: links.apps.clone(),
        }),
        Arc::new(CalculatorProvider {
            policy: policy.clone(),
            rates_file: crate::providers::calculator::fetch::cache_path(&env.dirs),
            ..CalculatorProvider::default()
        }),
        Arc::new(SettingsProvider::new(&env.dirs.data)),
        Arc::new(SystemProvider::new(links.session.clone())),
        Arc::new(FilesProvider {
            policy: policy.clone(),
            frecency: history.shared(),
            source: IndexSource::Detect(env.buses.session.clone()),
            home: env.dirs.home.clone(),
            viewer_store: env.dirs.data.join(anyview_store::STORE_FOLDER),
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
        Arc::new(ThingsProvider {
            companion: links.companion.clone(),
            control: links.companion_control.clone(),
        }),
        Arc::new(CompanionProvider {
            companion: links.companion.clone(),
        }),
        Arc::new(HelpersProvider {
            companion: links.companion.clone(),
        }),
        Arc::new(RunsProvider {
            companion: links.companion.clone(),
        }),
        Arc::new(MemoryProvider {
            companion: links.companion.clone(),
            control: links.companion_control.clone(),
        }),
    ]
}
