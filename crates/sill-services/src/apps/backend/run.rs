//! The apps task: scan, publish, then watch every applications directory and rescan once quiet.

use std::path::{Path, PathBuf};
use std::time::Duration;

use ds_helpers::EntryLocale;
use sill_model::apps::{AppIndex, AppsCommand, AppsEvent, step};
use tokio::sync::{mpsc, watch};

use super::icons;
use super::locales::session_locales;
use crate::apps::{IconRoots, icon_theme, scan};
use crate::debounced::{Debounced, EventFilter, Place};
use crate::error::ServiceError;
use crate::machine::Published;

/// How long the applications directories (and the icon theme file) must be quiet before a
/// rescan.
const RESCAN_QUIET: Duration = Duration::from_millis(500);

/// Scan once, publish, then watch every applications directory and the settings file the icon
/// theme lives in (all coalesced onto one signal), rescanning [`RESCAN_QUIET`] after the
/// last one settles or on [`AppsCommand::Rescan`], until every receiver and control is gone.
pub(crate) async fn run(
    tx: watch::Sender<AppIndex>,
    mut commands: mpsc::UnboundedReceiver<AppsCommand>,
    config: PathBuf,
    dirs: Vec<PathBuf>,
    roots: Vec<PathBuf>,
) {
    let tx = Published::new(tx, step);
    let locales = session_locales();
    publish(&tx, &config, &dirs, &roots, &locales).await;

    // Nothing to watch: nothing will ever say to rescan, so the task is done.
    let Ok(mut rescans) = watch_dirs(&config, &dirs) else {
        return;
    };

    let mut listening = true;
    while !tx.is_closed() {
        if listening {
            tokio::select! {
                settled = rescans.settled() => {
                    if settled.is_none() {
                        return;
                    }
                    publish(&tx, &config, &dirs, &roots, &locales).await;
                }
                command = commands.recv() => match command {
                    Some(AppsCommand::Rescan) => publish(&tx, &config, &dirs, &roots, &locales).await,
                    None => listening = false,
                },
            }
        } else if rescans.settled().await.is_some() {
            publish(&tx, &config, &dirs, &roots, &locales).await;
        } else {
            return;
        }
    }
}

/// Rescan (off the async worker: filesystem and theme-file reads are blocking) and publish,
/// skipping the send entirely when the fresh index equals the one already shown
/// (`Published`, so a receiver only wakes on a real change).
async fn publish(
    tx: &Published<AppIndex, AppsEvent>,
    config: &Path,
    dirs: &[PathBuf],
    roots: &[PathBuf],
    locales: &[EntryLocale],
) {
    let config = config.to_path_buf();
    let dirs = dirs.to_vec();
    let roots = roots.to_vec();
    let locales = locales.to_vec();
    let found = tokio::task::spawn_blocking(move || {
        let theme = icon_theme(&config);
        scan(&dirs, &theme, &IconRoots::new(roots), &locales)
    })
    .await
    .unwrap_or_default();
    tx.feed(AppsEvent::Scanned(found));
}

/// Every applications directory that exists, plus the directory the GTK settings file lives in
/// (a theme change rescans exactly like a new `.desktop` file, since [`publish`] re-reads
/// [`icon_theme`] every time), as one watch. A directory that does not exist is skipped:
/// `sill reload` and the next daemon start both still find what it would have caught. A close
/// after a write counts, because the scan itself opens and closes every `.desktop` file for
/// reading and counting those made each rescan signal the next one, forever (a busy loop of
/// scans and blocking-pool threads at idle).
fn watch_dirs(config: &Path, dirs: &[PathBuf]) -> Result<Debounced, ServiceError> {
    let mut places: Vec<Place> = dirs
        .iter()
        .filter(|dir| dir.is_dir())
        .cloned()
        .map(Place::Tree)
        .collect();
    let theme_file = icons::icon_theme_path(config);
    if let Some(parent) = theme_file.parent()
        && parent.is_dir()
        && !places.contains(&Place::Tree(parent.to_path_buf()))
    {
        places.push(Place::Tree(parent.to_owned()));
    }
    Debounced::start(places, EventFilter::ClosedAfterWrite, RESCAN_QUIET)
}

#[cfg(test)]
mod tests {
    use crate::apps::AppsService;

    use sill_model::apps::AppIndex;

    /// Starting must never panic (matching every other service: `services.rs`'s own test reads
    /// every stub's default and this one for real). Like every other async-first-value service
    /// here (`tray`, `battery`, `network`), the real scan runs on the runtime after `spawn`
    /// returns, not before, so the receiver's value right after `spawn` is racy with `run`'s
    /// first `publish().await` — this only asserts the receiver is usable, not what it holds
    /// yet.
    #[test]
    fn spawn_does_not_panic_and_the_receiver_is_usable() {
        let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let scratch = sill_testkit::TempDir::new("apps-spawn");
        let apps = crate::service::spawn::<AppsService>(
            rt.handle(),
            &crate::env::Env::isolated(scratch.path()),
            (),
        );
        let (rx, control) = (apps.state, apps.control);
        let _: AppIndex = rx.borrow().clone();
        drop(control);
        drop(rx);
    }
}
