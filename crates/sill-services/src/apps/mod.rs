//! The application index: desktop entries from every XDG applications directory, their icons
//! resolved once, rescanned on change, through freedesktop-desktop-entry and freedesktop-icons; the
//! theme from CosmicTk `icon_theme`, else Cosmic, else hicolor;
//! `notify` on the applications directories, 500 ms.

mod exec;
mod icons;
mod index;
mod match_app;
mod scan;

pub use exec::{ExecArgs, ExecError, expand_exec};
pub use icons::{IconTheme, icon_theme, pixmap_dirs, resolve_icon};
pub use index::{
    App, AppIcon, AppIndex, AppsEvent, DbusActivation, DesktopActionEntry, Listing, Terminal, step,
};
pub use match_app::match_app;
pub use scan::{applications_dirs, parse_entry, scan};

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, watch};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// How long the applications directories (and the icon theme file) must be quiet before a
/// rescan.
pub const RESCAN_DEBOUNCE: Duration = Duration::from_millis(500);

/// What the apps service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppsCommand {
    /// Rescan now (`sill reload`).
    Rescan,
}

/// The desktop entries and their icons as a [`Service`]: scanned once and published, then every
/// applications directory watched and rescanned after [`RESCAN_DEBOUNCE`] of quiet.
#[derive(Debug)]
pub enum AppsService {}

impl Service for AppsService {
    const NAME: &'static str = "apps";
    type State = AppIndex;
    type Command = AppsCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let (config, data) = (env.dirs.config.clone(), env.dirs.data.clone());
        async move {
            run(io.state, io.commands, config, data).await;
            Ok(())
        }
    }
}

/// Scan once, publish, then watch every applications directory and the CosmicTk file the icon
/// theme lives in (all coalesced onto one signal), rescanning [`RESCAN_DEBOUNCE`] after the
/// last one settles or on [`AppsCommand::Rescan`], until every receiver and control is gone.
async fn run(
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
/// same shape as `sill_settings::watch`'s debounce); `false` once every watcher has dropped, so
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
    use super::AppsService;
    use crate::apps::AppIndex;

    /// Starting must never panic (matching every other service: `services.rs`'s own test reads
    /// every stub's default and this one for real). Like every other async-first-value service
    /// here (`tray`, `battery`, `network`), the real scan runs on the runtime after `spawn`
    /// returns, not before, so the receiver's value right after `spawn` is racy with `run`'s
    /// first `publish().await` — this only asserts the receiver is usable, not what it holds
    /// yet.
    #[test]
    fn spawn_does_not_panic_and_the_receiver_is_usable() {
        let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let scratch = crate::env::scratch::Scratch::new("apps-spawn");
        let apps = crate::service::spawn::<AppsService>(rt.handle(), &scratch.env(), ());
        let (rx, control) = (apps.state, apps.control);
        let _: AppIndex = rx.borrow().clone();
        drop(control);
        drop(rx);
    }
}
