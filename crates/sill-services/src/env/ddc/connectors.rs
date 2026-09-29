//! The kernel's connected connectors, read from `/sys/class/drm`: what the DDC probe matches
//! buses against and what the displays service describes.

use std::path::Path;

use sill_model::displays::drm::{Connector, connector_name};

/// Every connected connector under `root` (`/sys/class/drm`), in name order.
pub fn connected(root: &Path) -> Vec<Connector> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found: Vec<Connector> = entries
        .flatten()
        .filter_map(|entry| {
            let name = connector_name(&entry.file_name().to_string_lossy())?;
            let dir = entry.path();
            let status = std::fs::read_to_string(dir.join("status")).ok()?;
            (status.trim() == "connected").then_some(())?;
            Some(Connector {
                name,
                edid: std::fs::read(dir.join("edid")).unwrap_or_default(),
            })
        })
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}
