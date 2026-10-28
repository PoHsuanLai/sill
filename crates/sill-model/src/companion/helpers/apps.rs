//! The apps a helper may be given: those whose actions every agent may be shown and that have
//! some. An app that is host-only, or has nothing to do, is not offered.

use std::collections::BTreeSet;

use docket_manifest::{ValidManifest, Visibility};
use prov::AppName;

/// The apps of `manifests` a helper may be given.
pub fn offered_apps(manifests: &[ValidManifest]) -> BTreeSet<AppName> {
    manifests
        .iter()
        .map(ValidManifest::manifest)
        .filter(|manifest| manifest.visibility == Visibility::Everyone)
        .filter(|manifest| !manifest.actions.is_empty())
        .map(|manifest| manifest.app.clone())
        .collect()
}
