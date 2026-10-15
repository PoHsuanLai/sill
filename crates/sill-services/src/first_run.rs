//! Whether a store was ever written: the one fact a seed (`sill_model::seed`) is decided by.

use std::path::Path;

use sill_model::seed::Store;

/// Whether `file` exists.
pub fn store_of(file: &Path) -> Store {
    if file.exists() {
        Store::Present
    } else {
        Store::Missing
    }
}
