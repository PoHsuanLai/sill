//! The apps task: scan, publish, then watch every applications directory and rescan once quiet.

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use sill_model::apps::{AppIndex, AppsCommand, AppsEvent, step};
use tokio::sync::{mpsc, watch};

use super::icons;
use crate::apps::{applications_dirs, icon_theme, scan};

/// How long the applications directories (and the icon theme file) must be quiet before a
/// rescan.
pub const RESCAN_DEBOUNCE: Duration = Duration::from_millis(500);

/// Scan once, publish, then watch every applications directory and the CosmicTk file the icon
/// theme lives in (all coalesced onto one signal), rescanning [`RESCAN_DEBOUNCE`] after the
/// last one settles or on [`AppsCommand::Rescan`], until every receiver and control is gone.
pub(crate) async fn run(
    tx: watch::Sender<AppIndex>,
    mut commands: mpsc::UnboundedReceiver<AppsCommand>,
    config: PathBuf,
    data: PathBuf,
) {
    let dirs = applications_dirs(&data);
    let locales = freedesktop_desktop_entry::get_languages_from_env();
    publish(&tx, &config, &dirs, &locales).await;

    let (signal, mut signals) = mpsc::unbounded_channel::<()>();
    let _watchers = watch_dirs(&config, &dirs, signal);

    let mut listening = true;
    while !tx.is_closed() {
        if listening {
            tokio::select! {
                changed = settle(&mut signals) => {
                    if !changed {
                        return;
                    }
                    publish(&tx, &config, &dirs, &locales).await;
                }
                command = commands.recv() => match command {
                    Some(AppsCommand::Rescan) => publish(&tx, &config, &dirs, &locales).await,
                    None => listening = false,
                },
            }
        } else if settle(&mut signals).await {
            publish(&tx, &config, &dirs, &locales).await;
        } else {
            return;
        }
    }
}

/// Rescan (off the async worker: filesystem and theme-file reads are blocking) and publish,
/// skipping the send entirely when the fresh index equals the one already shown
/// (`watch::Sender::send_if_modified`, so a receiver only wakes on a real change).
async fn publish(
    tx: &watch::Sender<AppIndex>,
    config: &Path,
    dirs: &[PathBuf],
    locales: &[String],
) {
    let config = config.to_path_buf();
    let dirs = dirs.to_vec();
    let locales = locales.to_vec();
    let found = tokio::task::spawn_blocking(move || {
        let theme = icon_theme(&config);
        scan(&dirs, &theme, &locales)
    })
    .await
    .unwrap_or_default();
    tx.send_if_modified(|index| {
        let next = step(index.clone(), AppsEvent::Scanned(found));
        if next == *index {
            false
        } else {
            *index = next;
            true
        }
    });
}

/// Wait for the first signal, then coalesce the rest of the burst for [`RESCAN_DEBOUNCE`] (the
/// same shape as `ds_settings::Store::watch`'s debounce); `false` once every watcher has dropped, so
/// nothing is left to ever signal again.
async fn settle(signals: &mut mpsc::UnboundedReceiver<()>) -> bool {
    if signals.recv().await.is_none() {
        return false;
    }
    loop {
        match tokio::time::timeout(RESCAN_DEBOUNCE, signals.recv()).await {
            Ok(Some(())) => continue,
            Ok(None) => return true,
            Err(_timed_out) => return true,
        }
    }
}

/// A `notify` watch on every applications directory that exists, plus the directory the
/// CosmicTk icon-theme file lives in (a theme change rescans exactly like a new `.desktop`
/// file, since [`publish`] re-reads [`icon_theme`] every time), coalesced onto one `signal`. A
/// directory whose watch fails to start (most often: it does not exist) is silently skipped —
/// `sill reload` and the next daemon start both still find what it would have caught.
fn watch_dirs(
    config: &Path,
    dirs: &[PathBuf],
    signal: mpsc::UnboundedSender<()>,
) -> Vec<RecommendedWatcher> {
    let mut targets: Vec<PathBuf> = dirs.iter().filter(|dir| dir.is_dir()).cloned().collect();
    let theme_file = icons::cosmic_tk_icon_theme_path(config);
    if let Some(parent) = theme_file.parent()
        && parent.is_dir()
        && !targets.contains(&parent.to_path_buf())
    {
        targets.push(parent.to_owned());
    }
    targets
        .into_iter()
        .filter_map(|dir| {
            let tx = signal.clone();
            let mut watcher =
                notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                    if res.is_ok() {
                        // A dropped signal means the task ended: nobody is left to rescan for.
                        let _ = tx.send(());
                    }
                })
                .ok()?;
            watcher.watch(&dir, RecursiveMode::Recursive).ok()?;
            Some(watcher)
        })
        .collect()
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
