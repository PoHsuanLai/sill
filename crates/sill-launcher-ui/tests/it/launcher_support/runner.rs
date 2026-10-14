//! The fake search runner: every `SearchCommand::Query` answered on the results `watch` with a
//! new generation, chosen rows carried out as the real runner does. On its own thread for the
//! demo's window, or as a task on a harness's thread so a virtual-clock test sees each answer in
//! the `advance` that asked: off-thread work never lands on the virtual clock.

use std::sync::{Arc, Mutex};

use sill_launcher::{Cancel, Generation, Item, Provider, ProviderKind, Query, Scope, VecSink};
use sill_launcher_ui::launcher::LauncherProviders;
use sill_model::companion::Companion;
use sill_model::launch::LaunchCommand;
use sill_model::results::{Results, SearchCommand};
use sill_model::windows::WlCommand;
use sill_search::providers::{CompanionProvider, RunsProvider};
use sill_services::Control;
use tokio::sync::{mpsc, watch};

use super::{carry, placed, search};

/// The runner's state: where answers go, and the rows of the last query.
pub(super) struct Answer {
    pub(super) results: watch::Sender<Results>,
    pub(super) searches: Arc<Mutex<Vec<SearchCommand>>>,
    pub(super) wl: Control<WlCommand>,
    pub(super) launch: Control<LaunchCommand>,
    pub(super) providers: LauncherProviders,
    /// The fake companion service's state, which the companion's providers read.
    pub(super) companion: watch::Receiver<Companion>,
    pub(super) generation: u64,
    pub(super) rows: Vec<Item>,
}

impl Answer {
    /// What the real provider of `kind` lists for `text` asked alone.
    fn companion_rows(&self, kind: ProviderKind, text: &str) -> Vec<Item> {
        let provider: Box<dyn Provider> = match kind {
            ProviderKind::Runs => Box::new(RunsProvider {
                companion: self.companion.clone(),
            }),
            _ => Box::new(CompanionProvider {
                companion: self.companion.clone(),
            }),
        };
        let query = Query {
            scope: Scope::Only(kind),
            ..Query::begin(text, &Cancel::new())
        };
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        sink.0.into_iter().flat_map(|batch| batch.items).collect()
    }

    /// Carry out one command and log it.
    fn take(&mut self, command: SearchCommand) {
        match &command {
            SearchCommand::Query(text) | SearchCommand::Filtered { text, .. } => {
                self.generation += 1;
                let only = match &command {
                    SearchCommand::Filtered { provider, .. } => Some(*provider),
                    _ => None,
                };
                self.rows = match only {
                    // The companion's rows are the real providers', over the fake service's state.
                    Some(kind @ (ProviderKind::Companion | ProviderKind::Runs)) => {
                        self.companion_rows(kind, text)
                    }
                    _ => search(text),
                };
                let placement = placed(&self.rows, text, only);
                self.results.send_replace(Results {
                    generation: Generation(self.generation),
                    items: placement.items,
                    sections: placement.spans,
                });
            }
            SearchCommand::Chosen { key, choice } => {
                carry(
                    &self.rows,
                    key,
                    choice,
                    &self.providers,
                    &self.wl,
                    &self.launch,
                );
            }
            SearchCommand::Cancel => self.rows.clear(),
            SearchCommand::Policy(_) => {}
        }
        if let Ok(mut log) = self.searches.lock() {
            log.push(command);
        }
    }
}

/// A search runner that has not started.
pub struct Runner {
    pub(super) commands: mpsc::UnboundedReceiver<SearchCommand>,
    pub(super) answer: Answer,
}

impl Runner {
    /// Answer on a thread of its own until the services go.
    pub fn on_thread(self) {
        let Runner {
            mut commands,
            mut answer,
        } = self;
        std::thread::spawn(move || {
            while let Some(command) = commands.blocking_recv() {
                answer.take(command);
            }
        });
    }

    /// Answer as a task on whichever executor polls it (a harness's, on its own thread).
    pub async fn run(self) {
        let Runner {
            mut commands,
            mut answer,
        } = self;
        while let Some(command) = commands.recv().await {
            answer.take(command);
        }
    }
}
