//! The launcher's search: providers run off the UI thread, batches merge into one ranked list,
//! stale generations are dropped, and every activation is counted in
//! `$XDG_STATE_HOME/sill/frecency.json`.
//!
//! Each [`SearchCommand::Query`] begins a generation (`Cancel::begin`, so every older query
//! reads as cancelled) and runs every provider on its own blocking task; their batches merge
//! through `step` as they arrive and the merged rows are published in the command
//! menu's groups (`arrange`: Top hit, then one run per provider in Tab order; with nothing
//! typed, the System rows then Recent). Batches of an older generation are dropped however
//! late they arrive.

mod effect;
mod frecency_file;
mod history;
mod registry;
mod run;
mod sink;
#[cfg(test)]
mod tests;

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use effect::Outlets;
use history::{History, Persist};
use sill_launcher::Provider;
use sill_model::apps::AppIndex;
use sill_model::clipboard::Clipboard;
use sill_model::companion::{Companion, CompanionCommand};
use sill_model::results::{LauncherPolicy, Results, SearchCommand};
use sill_model::seed::Store;
use sill_model::session::Session;
use sill_model::windows::{WindowModel, WlCommand};
use tokio::sync::watch;

use sill_services::control::Control;
use sill_services::env::Env;
use sill_services::error::ServiceError;
use sill_services::first_run::store_of;
use sill_services::service::{Service, ServiceIo};

pub use frecency_file::{FILE_NAME as FRECENCY_FILE, load_frecency, save_frecency};

/// What the search runs over.
#[derive(Clone)]
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
/// ones a chosen row's activation goes to (windows are focused and closed through `wl`; the
/// launcher surface starts apps itself, with an activation token).
#[derive(Debug, Clone)]
pub struct ResultsLinks {
    pub apps: watch::Receiver<AppIndex>,
    pub windows: watch::Receiver<WindowModel>,
    pub clipboard: watch::Receiver<Clipboard>,
    /// The companion's state, which its three providers read, and where its two lookups go.
    pub companion: watch::Receiver<Companion>,
    pub companion_control: Control<CompanionCommand>,
    /// What logind offers, which the system rows list (the session service's state).
    pub session: watch::Receiver<Session>,
    pub wl: Control<WlCommand>,
}

/// The launcher's search as a [`Service`]: its providers queried in parallel on the blocking
/// pool, its history loaded from sill's folder of `env`'s state directory. Tracker (the files
/// index) is reached on `env`'s session bus; the rates cache and
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
        let first_run = match &deps {
            Sources::Services(links) if store_of(&dir.join(FRECENCY_FILE)) == Store::Missing => {
                Some(links.apps.clone())
            }
            Sources::Services(_) | Sources::Providers(_) => None,
        };
        let (policy, policy_rx) = watch::channel(LauncherPolicy::default());
        let (providers, outlets) = match deps {
            Sources::Services(links) => (
                registry::providers(&links, env, &history, &policy_rx),
                Outlets { wl: Some(links.wl) },
            ),
            Sources::Providers(providers) => (providers, Outlets::default()),
        };
        async move {
            runner(
                providers,
                history,
                Some(dir),
                outlets,
                policy,
                first_run,
                io,
            )
            .await;
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
    first_run: Option<watch::Receiver<AppIndex>>,
    io: ServiceIo<ResultsService>,
) {
    let initial = history.read(Clone::clone);
    let persist = Persist::start(dir, initial);
    let (mut search, arrivals) =
        run::Search::new(providers, history, persist, outlets, policy, io.state);
    search.first_run = first_run;
    run::run(search, io.commands, arrivals).await;
}
