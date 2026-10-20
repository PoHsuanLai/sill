//! The focus task: the mode follows the settled settings, and a set from any surface is written
//! into `settings.toml`. The link the surfaces hold is `sill_ui_kit::links::FocusLink`.
//!
//! Setting goes through the file, never around it: the write is seen by the settings watch like
//! any other edit, the feed settles, and the mode follows. So the file stays the one truth, and a
//! hand edit and a toggle cannot disagree.

use ds_settings::Store;
use sill_settings::{FocusMode, ShellFile, focus_mode, with_focus_mode};
use sill_ui_kit::links::FocusLink;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch};

use super::patch::{KeyEdit, patch};
use super::store::load;

/// Start the focus task on `rt`: its mode follows `settings` through [`focus_mode`], and each set
/// patches `notifications.dnd` of the file in `store` (the mode is stored with
/// [`with_focus_mode`]).
pub fn spawn_focus(rt: &Handle, settings: watch::Receiver<ShellFile>, store: Store) -> FocusLink {
    let now = focus_mode(&settings.borrow());
    let (mode_tx, mode_rx) = watch::channel(now);
    let (set_tx, set_rx) = mpsc::unbounded_channel();
    rt.spawn(run(settings, store, mode_tx, set_rx));
    FocusLink::new(mode_rx, set_tx)
}

/// Follows `settings` into `mode_tx` and carries out every queued set, until either channel
/// closes.
async fn run(
    mut settings: watch::Receiver<ShellFile>,
    store: Store,
    mode_tx: watch::Sender<FocusMode>,
    mut sets: mpsc::UnboundedReceiver<FocusMode>,
) {
    loop {
        tokio::select! {
            changed = settings.changed() => {
                if changed.is_err() {
                    return;
                }
                let mode = focus_mode(&settings.borrow());
                if mode_tx.send(mode).is_err() {
                    return;
                }
            }
            set = sets.recv() => match set {
                Some(mode) => write(&store, mode),
                None => return,
            }
        }
    }
}

/// Writes `mode` into the store's `settings.toml`: the one key `notifications.dnd`, in place.
fn write(store: &Store, mode: FocusMode) {
    let dnd = with_focus_mode(load(store), mode).notifications.dnd;
    let done = KeyEdit::of("notifications.dnd", &dnd).and_then(|edit| patch(store, &[edit]));
    if let Err(error) = done {
        eprintln!("sill: focus: could not save settings.toml: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sill_settings::{FocusMode, ShellFile, focus_mode};
    use sill_ui_kit::links::Delivery;
    use tokio::sync::watch;

    use super::spawn_focus;
    use crate::daemon::settings::store::{load, store};

    #[test]
    fn a_set_is_stored_and_the_mode_follows_the_feed() {
        let dir = std::env::temp_dir().join(format!("sill-focus-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = store(&dir);
        let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let (feed_tx, feed) = watch::channel(ShellFile::default());
        let link = spawn_focus(rt.handle(), feed, store.clone());
        let mut mode = link.mode();
        assert_eq!(*mode.borrow_and_update(), FocusMode::Off);

        assert_eq!(link.set(FocusMode::DoNotDisturb), Delivery::Queued);
        // The write lands in the file; the daemon's watch would then publish it.
        let stored = (0..50).find_map(|_| {
            std::thread::sleep(Duration::from_millis(20));
            let file = load(&store);
            (focus_mode(&file) == FocusMode::DoNotDisturb).then_some(file)
        });
        let stored = stored.unwrap_or_else(|| panic!("the set never reached settings.toml"));
        assert_eq!(
            *mode.borrow(),
            FocusMode::Off,
            "not before the feed settles"
        );

        feed_tx.send_replace(stored);
        rt.block_on(async {
            tokio::time::timeout(Duration::from_secs(1), mode.changed())
                .await
                .unwrap_or_else(|_| panic!("the mode did not follow the feed"))
                .unwrap_or_else(|e| panic!("{e}"));
        });
        assert_eq!(*mode.borrow(), FocusMode::DoNotDisturb);
        let _ = std::fs::remove_dir_all(dir);
    }
}
