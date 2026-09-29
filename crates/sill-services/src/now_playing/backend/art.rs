//! Cover art: a local file as it is, a remote one fetched once into the cache.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sill_model::now_playing::{
    Art, ArtSource, Media, MediaEvent, PlayerId, art_cache_name, art_source,
};
use tokio::sync::{mpsc, watch};
use zbus::zvariant::OwnedValue;

use super::run::{apply, metadata_string};

/// The art `metadata` names, resolved as far as a bus read can take it: a local file is used as
/// it is, a remote one already in the cache is used from there, and a remote one not yet cached
/// is a job for [`spawn_art_fetch`].
pub(crate) fn resolve_art(
    metadata: &HashMap<String, OwnedValue>,
    cache_dir: Option<&Path>,
) -> (Art, Option<(String, PathBuf)>) {
    let url = metadata_string(metadata, "mpris:artUrl");
    match art_source(&url) {
        ArtSource::File(path) => (Art::File(path), None),
        ArtSource::Remote(url) => match cache_dir {
            Some(dir) => {
                let cache_path = dir.join(art_cache_name(&url));
                if cache_path.is_file() {
                    (Art::File(cache_path), None)
                } else {
                    (Art::None, Some((url, cache_path)))
                }
            }
            None => (Art::None, None),
        },
        ArtSource::Unusable => (Art::None, None),
    }
}

/// Downloads `url` into `cache_path` with `curl` (Fedora ships it) and reports success on `art_tx`.
/// Never downloads twice: [`resolve_art`] only schedules this
/// when the file was not already there.
pub(crate) fn spawn_art_fetch(
    id: PlayerId,
    url: String,
    cache_path: PathBuf,
    art_tx: mpsc::UnboundedSender<(PlayerId, PathBuf)>,
) {
    tokio::spawn(async move {
        if let Some(parent) = cache_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let status = tokio::process::Command::new("curl")
            .arg("-sfL")
            .arg("-o")
            .arg(&cache_path)
            .arg(&url)
            .status()
            .await;
        if matches!(status, Ok(status) if status.success()) {
            let _ = art_tx.send((id, cache_path));
        }
    });
}

/// A download finished: folds the cached path into whatever that player's track looks like now.
pub(crate) fn apply_art(tx: &watch::Sender<Media>, id: PlayerId, path: PathBuf) {
    let media = tx.borrow().clone();
    let Some(player) = media.players.iter().find(|p| p.id == id) else {
        return;
    };
    let mut track = player.track.clone();
    track.art = Art::File(path);
    apply(tx, MediaEvent::Track { id, track });
}
