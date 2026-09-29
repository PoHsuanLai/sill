//! The launcher's search: providers run off the UI thread, batches merge into one ranked list,
//! stale generations are dropped, and every activation is counted in
//! `$XDG_STATE_HOME/sill/frecency.json`.
//!
//! Each [`SearchCommand::Query`] begins a generation (`Cancel::begin`, so every older query
//! reads as cancelled) and runs every provider on its own blocking task; their batches merge
//! through [`reduce_results`] as they arrive and the merged rows are published in the command
//! menu's groups (`arrange`: Top hit, then one run per provider in Tab order; with nothing
//! typed, the System rows then Recent). Batches of an older generation are dropped however
//! late they arrive.

mod arrange;
mod effect;
mod frecency_file;
mod history;
mod model;
mod policy;
mod registry;
mod run;
mod sink;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use frecency_file::{FILE_NAME as FRECENCY_FILE, load_frecency, save_frecency};
pub use model::{Results, ResultsEvent, reduce_results};
pub use policy::{CurrencyPolicy, FilesChoice, LauncherPolicy, WebPolicy};

use std::path::PathBuf;
use std::sync::Arc;

use sill_launcher::{Choice, ItemKey, Provider, ProviderKind};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::apps::AppIndex;
use crate::clipboard::Clipboard;
use crate::control::Control;
use crate::cosmic_wl::{Toplevels, WlCommand, Workspaces};
use crate::launch::LaunchCommand;
use effect::Outlets;
use history::{History, Persist};

/// What the search can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchCommand {
    /// The text changed: begin a generation and query every provider.
    Query(String),
    /// The text changed under the Tab filter: begin a generation and query `provider` alone
    /// (`Scope::Only`), so its rows are not capped by the other providers'.
    Filtered {
        text: String,
        provider: ProviderKind,
    },
    /// A row was chosen: count it for frecency.
    Chosen { key: ItemKey, choice: Choice },
    /// The launcher closed: cancel the running generation.
    Cancel,
    /// The settings changed what the settings-gated providers may do; the daemon sends it at start,
    /// and until then every one of them is off.
    Policy(LauncherPolicy),
}

/// Start the search runner with `providers` (queried in parallel on `rt`'s blocking pool),
/// its history loaded from `$XDG_STATE_HOME/sill`. Providers built by the caller rank with
/// their own frecency handle; [`spawn_with_providers`] builds the five sharing this runner's.
pub fn spawn(
    rt: &Handle,
    providers: Vec<Arc<dyn Provider>>,
) -> (watch::Receiver<Results>, Control<SearchCommand>) {
    let dir = state_dir();
    let history = History::new(dir.as_deref().map(load_frecency).unwrap_or_default());
    let policy = watch::Sender::new(LauncherPolicy::default());
    start(rt, providers, history, dir, Outlets::default(), policy)
}

/// What the daemon's search reads and sends to: the services its providers query and the
/// ones a chosen row's activation goes to.
pub(crate) struct Wiring<'a> {
    pub(crate) apps: &'a watch::Receiver<AppIndex>,
    pub(crate) toplevels: &'a watch::Receiver<Toplevels>,
    pub(crate) workspaces: &'a watch::Receiver<Workspaces>,
    pub(crate) clipboard: &'a watch::Receiver<Clipboard>,
    pub(crate) wl: &'a Control<WlCommand>,
    pub(crate) launch: &'a Control<LaunchCommand>,
}

/// The daemon's search: the five providers, the apps provider ranking with the same history
/// the runner records choices into; windows are focused and closed through `wl`, apps started
/// through `launch`.
pub(crate) fn spawn_with_providers(
    rt: &Handle,
    wiring: Wiring<'_>,
) -> (watch::Receiver<Results>, Control<SearchCommand>) {
    let dir = state_dir();
    let history = History::new(dir.as_deref().map(load_frecency).unwrap_or_default());
    let (policy, policy_rx) = watch::channel(LauncherPolicy::default());
    let providers = registry::providers(&wiring, &history, &policy_rx);
    let outlets = Outlets {
        wl: Some(wiring.wl.clone()),
        launch: Some(wiring.launch.clone()),
    };
    start(rt, providers, history, dir, outlets, policy)
}

/// Run the search on `rt` with `history`, saving it under `dir` when there is one.
fn start(
    rt: &Handle,
    providers: Vec<Arc<dyn Provider>>,
    history: History,
    dir: Option<PathBuf>,
    outlets: Outlets,
    policy: watch::Sender<LauncherPolicy>,
) -> (watch::Receiver<Results>, Control<SearchCommand>) {
    let (tx, rx) = watch::channel(Results::default());
    let (control, commands) = Control::channel();
    let initial = history.read(Clone::clone);
    let _entered = rt.enter();
    let persist = Persist::start(dir, initial);
    let (search, arrivals) = run::Search::new(providers, history, persist, outlets, policy, tx);
    rt.spawn(run::run(search, commands, arrivals));
    (rx, control)
}

/// `$XDG_STATE_HOME/sill`, else `$HOME/.local/state/sill`; `None` when neither is set.
fn state_dir() -> Option<PathBuf> {
    dirs::state_dir().map(|dir| dir.join("sill"))
}
