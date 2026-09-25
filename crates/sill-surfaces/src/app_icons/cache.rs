//! The re-tinted icon cache: one per process, shared by every surface (effects: a lock, tokio's
//! blocking pool, the filesystem).
//!
//! A raster is made once per [`IconKey`] (icon, size at the output scale, style, tint) on
//! tokio's blocking pool and kept as a file in `$XDG_CACHE_HOME/sill/icons/`, which the view
//! loads by `file://` URL. Files rather than memory: an `IconUrl` is a `file:` or a `data:`
//! URL, and a `data:` URL puts some twenty kilobytes of base64 into a tile's inline style,
//! which the dock rewrites on every magnification frame; a file URL is a few dozen bytes, and
//! the files outlive the process, so a restart re-tints nothing.
//!
//! The first render of a key shows the untinted file while the raster is made; the render
//! after it (every [`use_icon_resolver`] caller re-renders when one finishes) shows the cached
//! one. A new Space tint is a new key, so a workspace switch loads new entries rather than
//! re-tinting on every frame.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard};

use dioxus::prelude::*;
use shell_host::use_watch;
use tokio::sync::watch;

use crate::app_icons::key::{IconKey, Origin};
use crate::app_icons::plan::Art;
use crate::app_icons::render::{fresh, render, write_atomic};

/// How many rasters the cache has finished, successfully or not: what a resolver re-renders on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Generation(pub u64);

/// One key's state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Slot {
    /// Being made on the blocking pool.
    Pending,
    /// Made: this file.
    Ready(PathBuf),
    /// Could not be made (an unreadable or undecodable source): the untinted file stays.
    Failed,
}

#[derive(Debug)]
struct Cache {
    slots: Mutex<HashMap<IconKey, Slot>>,
    done: watch::Sender<Generation>,
}

static CACHE: LazyLock<Cache> = LazyLock::new(|| Cache {
    slots: Mutex::new(HashMap::new()),
    done: watch::Sender::new(Generation::default()),
});

/// `$XDG_CACHE_HOME/sill/icons`.
fn cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|dir| dir.join("sill").join("icons"))
}

fn slots() -> MutexGuard<'static, HashMap<IconKey, Slot>> {
    // A panic while holding the lock leaves a map that is still a valid map.
    CACHE
        .slots
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Turns an [`Art`] into the file to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconResolver {
    /// Read so the caller re-renders when a raster lands.
    pub seen: Generation,
}

/// The resolver for this component: it re-renders whenever the cache finishes a raster.
pub fn use_icon_resolver() -> IconResolver {
    let done = use_hook(|| CACHE.done.subscribe());
    IconResolver {
        seen: use_watch(done)(),
    }
}

impl IconResolver {
    /// The file `art` shows now: a plain file as it is; a styled one's cached raster when it is
    /// made, else its untinted source, starting the raster when nobody has.
    pub fn path(&self, art: &Art) -> PathBuf {
        match art {
            Art::Plain(path) => path.clone(),
            Art::Styled { key, source } => styled(key, source),
        }
    }
}

fn styled(key: &IconKey, source: &Path) -> PathBuf {
    let mut slots = slots();
    match slots.get(key) {
        Some(Slot::Ready(path)) => path.clone(),
        Some(Slot::Pending | Slot::Failed) => source.to_owned(),
        None => {
            // No runtime (a headless render) or no cache directory: show the source, and try
            // again on a later render rather than marking the key.
            if let (Ok(runtime), Some(dir)) = (tokio::runtime::Handle::try_current(), cache_dir()) {
                slots.insert(key.clone(), Slot::Pending);
                let (key, source) = (key.clone(), source.to_owned());
                runtime.spawn_blocking(move || make(key, source, dir));
            }
            source.to_owned()
        }
    }
}

/// Make `key`'s raster from `source` into `dir` (or find it there, newer than its source),
/// record it and wake every resolver.
fn make(key: IconKey, source: PathBuf, dir: PathBuf) {
    let name = key.file_name();
    let target = dir.join(&name);
    let slot = if fresh(&target, &source) {
        Slot::Ready(target)
    } else {
        match render(&source, key.px, key.style, tint_of(&key))
            .map_err(|e| e.to_string())
            .and_then(|png| write_atomic(&dir, &name, &png).map_err(|e| e.to_string()))
        {
            Ok(path) => Slot::Ready(path),
            Err(why) => {
                eprintln!("sill: icon {} not re-tinted: {why}", describe(&key.origin));
                Slot::Failed
            }
        }
    };
    slots().insert(key, slot);
    CACHE.done.send_modify(|generation| generation.0 += 1);
}

/// The tint a key was made from, back from its whole steps.
fn tint_of(key: &IconKey) -> ds::icon::Tint {
    ds::icon::Tint {
        hue: f32::from(key.tint.hue_tenths) / 10.0,
        chroma: f32::from(key.tint.chroma_ten_thousandths) / 10_000.0,
    }
}

fn describe(origin: &Origin) -> String {
    match origin {
        Origin::Shipped(app) => app.slug().to_owned(),
        Origin::Theme(path) => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{make, styled};
    use crate::app_icons::key::{IconKey, Origin, TintKey};
    use crate::app_icons::size::AssetPx;
    use ds::icon::{IconStyle, Tint};

    /// A made key is shown from its cache file; one never started shows its source (no
    /// runtime in a unit test, so nothing is started either).
    #[test]
    fn a_made_raster_replaces_the_source() {
        let dir = std::env::temp_dir().join(format!("sill-icon-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        let source = dir.join("app.png");
        image::RgbaImage::from_pixel(48, 48, image::Rgba([200, 60, 40, 255]))
            .save(&source)
            .unwrap_or_else(|e| panic!("{e}"));
        let key = IconKey {
            origin: Origin::Theme(source.clone()),
            px: AssetPx(48),
            style: IconStyle::Monochrome,
            tint: TintKey::of(Tint {
                hue: 150.0,
                chroma: 0.06,
            }),
        };
        let other = IconKey {
            px: AssetPx(72),
            ..key.clone()
        };
        assert_eq!(styled(&other, &source), source, "not made: the source");
        make(key.clone(), source.clone(), dir.join("cache"));
        let shown = styled(&key, &source);
        assert_eq!(shown, dir.join("cache").join(key.file_name()));
        let raster = image::open(&shown)
            .unwrap_or_else(|e| panic!("{e}"))
            .to_rgba8();
        let [r, g, _, _] = raster.get_pixel(24, 24).0;
        assert!(g > r, "re-tinted toward green");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
