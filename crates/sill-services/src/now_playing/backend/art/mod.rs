//! Cover art: every cover is read from sill's own art cache, which is a folder the shell may draw
//! from. A remote one is fetched once into it ([`curl`], run by [`ArtFetcher`]), a local `file:` one
//! outside every folder the shell may draw from is copied into it ([`cache::copy_cover`]), each
//! within a size, and the cache is kept to a number of files and bytes.

mod cache;
mod curl;
mod fetcher;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use sill_model::now_playing::{
    Art, ArtSource, Media, MediaEvent, PlayerId, art_cache_name, art_source, local_art_cache_name,
};
use zbus::zvariant::OwnedValue;

pub(crate) use fetcher::ArtFetcher;

use super::run::metadata_string;
use crate::machine::Published;

/// The largest cover a fetch or a copy may bring: a cover is drawn a few hundred pixels across.
const ART_MAX_BYTES: u64 = 5 * 1024 * 1024;

/// What it takes to bring one cover into the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArtJob {
    /// Download `url` to `cache_path`.
    Fetch { url: String, cache_path: PathBuf },
    /// Copy the local file `from` to `cache_path`.
    Copy { from: PathBuf, cache_path: PathBuf },
}

impl ArtJob {
    /// Where the cover will be.
    pub(crate) fn cache_path(&self) -> &Path {
        match self {
            ArtJob::Fetch { cache_path, .. } | ArtJob::Copy { cache_path, .. } => cache_path,
        }
    }
}

/// The art `metadata` names, resolved as far as a bus read can take it: a cover already in the
/// cache is used from there, and one that is not is a job for [`ArtFetcher::spawn`]. A local file
/// that is not a regular file of a sane size is no art, and neither is anything without a cache
/// folder to keep it in.
pub(crate) fn resolve_art(
    metadata: &HashMap<String, OwnedValue>,
    cache_dir: Option<&Path>,
) -> (Art, Option<ArtJob>) {
    let url = metadata_string(metadata, "mpris:artUrl");
    let (Some(dir), source) = (cache_dir, art_source(&url)) else {
        return (Art::None, None);
    };
    let job = match source {
        ArtSource::File(from) => {
            local_cache_path(&from, dir).map(|cache_path| ArtJob::Copy { from, cache_path })
        }
        ArtSource::Remote(url) => {
            let cache_path = dir.join(art_cache_name(&url));
            Some(ArtJob::Fetch { url, cache_path })
        }
        ArtSource::Unusable => None,
    };
    match job {
        Some(job) if job.cache_path().is_file() => (Art::File(job.cache_path().to_owned()), None),
        Some(job) => (Art::None, Some(job)),
        None => (Art::None, None),
    }
}

/// Where the local file `from` would be kept in `dir`, named by its path, time and size; `None`
/// when it is not a regular file no larger than [`ART_MAX_BYTES`].
fn local_cache_path(from: &Path, dir: &Path) -> Option<PathBuf> {
    let meta = std::fs::metadata(from).ok()?;
    if !meta.is_file() || meta.len() > ART_MAX_BYTES {
        return None;
    }
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(dir.join(local_art_cache_name(from, modified, meta.len())))
}

/// A cover arrived: folds the cached path into whatever that player's track looks like now.
pub(crate) fn apply_art(tx: &Published<Media, MediaEvent>, id: PlayerId, path: PathBuf) {
    let media = tx.borrow().clone();
    let Some(player) = media.players.iter().find(|p| p.id == id) else {
        return;
    };
    let mut track = player.track.clone();
    track.art = Art::File(path);
    tx.feed(MediaEvent::Track { id, track });
}

#[cfg(test)]
mod tests;
