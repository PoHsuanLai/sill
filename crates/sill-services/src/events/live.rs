//! The events task (M10 lane a fills it): waits for `Sources`, builds the provider
//! (`provider_for`, `auto` reading `$HOME`, `$XDG_DATA_HOME` and Akonadi's rc), reads it on the
//! blocking pool over `held_window(today)` in the system zone, watches `watched()` with `notify`
//! (debounced, like the settings watch), re-reads on a change, a `Refresh` and the day's turn,
//! and publishes through `reduce_agenda`. Ends when every receiver is gone.
//!
//! Frozen as a stub that returns at once, so the agenda holds today and `SourceStatus::Off`.

use tokio::sync::{mpsc, watch};

use super::EventsCommand;
use super::model::Agenda;

pub(super) async fn run(
    tx: watch::Sender<Agenda>,
    commands: mpsc::UnboundedReceiver<EventsCommand>,
) {
    let _ = (tx, commands);
}
