//! The launcher's search: providers run off the UI thread, batches merge into one ranked list,
//! stale generations are dropped (plan "Launcher v1": "stale generations dropped"), and every
//! activation is counted in `$XDG_STATE_HOME/sill/frecency.json`.

#![allow(unused_variables, dead_code)] // Freeze stubs

mod frecency_file;
mod model;

pub use frecency_file::{FILE_NAME as FRECENCY_FILE, load_frecency, save_frecency};
pub use model::{Results, ResultsEvent, reduce_results};

use std::sync::Arc;

use sill_launcher::{Choice, ItemKey, Provider};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

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

/// Start the search runner with `providers` (queried in parallel on `rt`'s blocking pool).
pub fn spawn(
    rt: &Handle,
    providers: Vec<Arc<dyn Provider>>,
) -> (watch::Receiver<Results>, Control<SearchCommand>) {
    todo!(
        "M4 launcher-core: Cancel::begin per Query, spawn_blocking per provider, ResultsEvent -> reduce_results"
    )
}
