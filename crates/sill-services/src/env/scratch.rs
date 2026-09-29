//! A fresh folder for one unit test, removed when dropped, and an [`Env`] inside it.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use super::Env;

/// A folder under the temp directory, unique to this process and call.
#[derive(Debug)]
pub(crate) struct Scratch(PathBuf);

impl Scratch {
    /// A new empty folder named after `name`.
    pub(crate) fn new(name: &str) -> Scratch {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("sill-{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        Scratch(dir)
    }

    /// [`Env::isolated`] in this folder.
    pub(crate) fn env(&self) -> Env {
        Env::isolated(&self.0)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
