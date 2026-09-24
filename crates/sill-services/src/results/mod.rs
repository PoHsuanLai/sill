//! The launcher's search: providers run off the UI thread, batches merge into one ranked list,
//! stale generations are dropped (plan "Launcher v1": "stale generations dropped"), and every
//! activation is counted in `$XDG_STATE_HOME/sill/frecency.json`.
//!
//! Each [`SearchCommand::Query`] begins a generation (`Cancel::begin`, so every older query
//! reads as cancelled) and runs every provider on its own blocking task; their batches merge
//! through [`reduce_results`] as they arrive and the merged rows are published in the command
//! menu's groups (`arrange`: Top hit, then one run per provider in Tab order; with nothing
//! typed, the System rows then Recent). Batches of an older generation are dropped however
//! late they arrive.

mod arrange;
mod frecency_file;
mod history;
mod model;
mod registry;
mod run;
mod sink;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use frecency_file::{FILE_NAME as FRECENCY_FILE, load_frecency, save_frecency};
pub use model::{Results, ResultsEvent, reduce_results};

use std::path::PathBuf;
use std::sync::Arc;

use sill_launcher::{Choice, ItemKey, Provider};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::apps::AppIndex;
use crate::control::Control;
use crate::cosmic_wl::Toplevels;
use history::{History, Persist};

/// What the search can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchCommand {
    /// The text changed: begin a generation and query every provider.
    Query(String),
    /// A row was chosen: count it for frecency.
    Chosen { key: ItemKey, choice: Choice },
    /// The launcher closed: cancel the running generation.
    Cancel,
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
    start(rt, providers, history, dir)
}

/// The daemon's search: the five providers over `apps` and `toplevels`, the apps provider
/// ranking with the same history the runner records choices into.
pub(crate) fn spawn_with_providers(
    rt: &Handle,
    apps: &watch::Receiver<AppIndex>,
    toplevels: &watch::Receiver<Toplevels>,
) -> (watch::Receiver<Results>, Control<SearchCommand>) {
    let dir = state_dir();
    let history = History::new(dir.as_deref().map(load_frecency).unwrap_or_default());
    let providers = registry::providers(apps, toplevels, &history);
    start(rt, providers, history, dir)
}

/// Run the search on `rt` with `history`, saving it under `dir` when there is one.
fn start(
    rt: &Handle,
    providers: Vec<Arc<dyn Provider>>,
    history: History,
    dir: Option<PathBuf>,
) -> (watch::Receiver<Results>, Control<SearchCommand>) {
    let (tx, rx) = watch::channel(Results::default());
    let (control, commands) = Control::channel();
    let initial = history.read(Clone::clone);
    let _entered = rt.enter();
    let persist = Persist::start(dir, initial);
    let (search, arrivals) = run::Search::new(providers, history, persist, tx);
    rt.spawn(run::run(search, commands, arrivals));
    (rx, control)
}

/// `$XDG_STATE_HOME/sill`, else `$HOME/.local/state/sill`; `None` when neither is set.
fn state_dir() -> Option<PathBuf> {
    dirs::state_dir().map(|dir| dir.join("sill"))
}
