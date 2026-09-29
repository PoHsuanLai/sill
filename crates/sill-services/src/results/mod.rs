//! The launcher's search: providers run off the UI thread, batches merge into one ranked list,
//! stale generations are dropped, and every activation is counted in
//! `$XDG_STATE_HOME/sill/frecency.json`.
//!
//! Each [`SearchCommand::Query`] begins a generation (`Cancel::begin`, so every older query
//! reads as cancelled) and runs every provider on its own blocking task; their batches merge
//! through [`step`] as they arrive and the merged rows are published in the command
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
mod tests;

pub use frecency_file::{FILE_NAME as FRECENCY_FILE, load_frecency, save_frecency};
pub use model::{Results, ResultsEvent, step};
pub use policy::{CurrencyPolicy, FilesChoice, LauncherPolicy, WebPolicy};

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use sill_launcher::{Choice, ItemKey, Provider, ProviderKind};
use tokio::sync::watch;

use crate::apps::AppIndex;
use crate::clipboard::Clipboard;
use crate::control::Control;
use crate::cosmic_wl::{CosmicWl, WlCommand};
use crate::env::Env;
use crate::error::ServiceError;
use crate::launch::LaunchCommand;
use crate::service::{Service, ServiceIo};
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

/// What the search runs over.
pub enum Sources {
    /// The daemon's providers, reading and sending to these services; the apps, files and emoji
    /// providers rank with the same history the runner records choices into.
    Services(ResultsLinks),
    /// These providers alone, ranking with their own frecency handles (a benchmark).
    Providers(Vec<Arc<dyn Provider>>),
}

impl fmt::Debug for Sources {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sources::Services(links) => f.debug_tuple("Services").field(links).finish(),
            Sources::Providers(providers) => {
                f.debug_tuple("Providers").field(&providers.len()).finish()
            }
        }
    }
}

/// The services the daemon's search reads and sends to: the ones its providers query, and the
/// ones a chosen row's activation goes to (windows are focused and closed through `wl`, apps
/// started through `launch`).
#[derive(Debug, Clone)]
pub struct ResultsLinks {
    pub apps: watch::Receiver<AppIndex>,
    pub cosmic_wl: watch::Receiver<CosmicWl>,
    pub clipboard: watch::Receiver<Clipboard>,
    pub wl: Control<WlCommand>,
    pub launch: Control<LaunchCommand>,
}

/// The launcher's search as a [`Service`]: its providers queried in parallel on the blocking
/// pool, its history loaded from sill's folder of `env`'s state directory. logind (the system
/// rows' offers) and Tracker (the files index) are reached on `env`'s buses; the rates cache and
/// the files provider's home are `env`'s directories.
#[derive(Debug)]
pub enum ResultsService {}

impl Service for ResultsService {
    const NAME: &'static str = "results";
    type State = Results;
    type Command = SearchCommand;
    type Deps = Sources;

    fn run(
        env: &Env,
        deps: Sources,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let dir = env.dirs.sill_state();
        let history = History::new(load_frecency(&dir));
        let (policy, policy_rx) = watch::channel(LauncherPolicy::default());
        let (providers, outlets) = match deps {
            Sources::Services(links) => (
                registry::providers(&links, env, &history, &policy_rx),
                Outlets {
                    wl: Some(links.wl),
                    launch: Some(links.launch),
                },
            ),
            Sources::Providers(providers) => (providers, Outlets::default()),
        };
        async move {
            runner(providers, history, Some(dir), outlets, policy, io).await;
            Ok(())
        }
    }
}

/// Run the search with `history`, saving it under `dir` when there is one.
async fn runner(
    providers: Vec<Arc<dyn Provider>>,
    history: History,
    dir: Option<PathBuf>,
    outlets: Outlets,
    policy: watch::Sender<LauncherPolicy>,
    io: ServiceIo<ResultsService>,
) {
    let initial = history.read(Clone::clone);
    let persist = Persist::start(dir, initial);
    let (search, arrivals) =
        run::Search::new(providers, history, persist, outlets, policy, io.state);
    run::run(search, io.commands, arrivals).await;
}
