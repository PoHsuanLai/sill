//! The fake search runner: every `SearchCommand::Query` answered on the results `watch` with a
//! new generation, chosen rows carried out as the real runner does. On its own thread for the
//! demo's window, or as a task on a harness's thread so a virtual-clock test sees each answer in
//! the `advance` that asked: off-thread work never lands on the virtual clock.

use std::sync::{Arc, Mutex};

use sill_launcher::{Generation, Item};
use sill_model::cosmic_wl::WlCommand;
use sill_model::launch::LaunchCommand;
use sill_model::results::{Results, SearchCommand};
use sill_services::Control;
use sill_ui_kit::surfaces::launcher::LauncherProviders;
use tokio::sync::{mpsc, watch};

use super::{carry, placed, search};

/// The runner's state: where answers go, and the rows of the last query.
pub(super) struct Answer {
    pub(super) results: watch::Sender<Results>,
    pub(super) searches: Arc<Mutex<Vec<SearchCommand>>>,
    pub(super) wl: Control<WlCommand>,
    pub(super) launch: Control<LaunchCommand>,
    pub(super) providers: LauncherProviders,
    pub(super) generation: u64,
    pub(super) rows: Vec<Item>,
}

impl Answer {
    /// Carry out one command and log it.
    fn take(&mut self, command: SearchCommand) {
        match &command {
            SearchCommand::Query(text) | SearchCommand::Filtered { text, .. } => {
                self.generation += 1;
                self.rows = search(text);
                let only = match &command {
                    SearchCommand::Filtered { provider, .. } => Some(*provider),
                    _ => None,
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
