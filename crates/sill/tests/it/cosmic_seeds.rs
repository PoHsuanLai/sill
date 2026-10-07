//! The COSMIC seed files under `dist/cosmic` parse the way cosmic-comp reads them.
//!
//! cosmic-comp reads `input_default` (one device's settings) as RON through cosmic-config; a seed
//! that does not parse is ignored by the compositor, so the person would be left with the
//! defaults and no message. The input settings are read with sill's own declaration of that wire
//! form (`sill::cli::import_input::wire`); every other seed need only be well-formed RON.

use std::fs;
use std::path::{Path, PathBuf};

use sill::cli::import_input::wire::{DeviceState, InputConfig};

fn dist_cosmic() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dist/cosmic")
}

#[test]
fn input_default_seed_parses_as_one_device_setting() {
    let path = dist_cosmic().join("com.system76.CosmicComp/v1/input_default");
    let text = fs::read_to_string(&path).expect("input_default is seeded");
    let default: InputConfig = ron::from_str(&text).expect("input_default is cosmic-comp RON");
    assert_eq!(default.state, DeviceState::Enabled);
    let scroll = default.scroll_config.expect("scroll_config");
    assert_eq!(scroll.natural_scroll, Some(true));
}

#[test]
fn no_seed_names_a_device_of_this_machine() {
    // Device names are the person's own hardware: `sill session import-input` reads them.
    let path = dist_cosmic().join("com.system76.CosmicComp/v1/input_devices");
    assert!(!path.exists(), "{} must not be shipped", path.display());
}

#[test]
fn every_seed_file_under_dist_cosmic_is_ron() {
    fn files(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for entry in fs::read_dir(dir)
            .expect("dist/cosmic is readable")
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                out.extend(files(&path));
            } else {
                out.push(path);
            }
        }
        out
    }
    let all = files(&dist_cosmic());
    assert!(all.len() >= 4, "found {all:?}");
    for path in all {
        let text = fs::read_to_string(&path).expect("seed is UTF-8");
        let parsed: Result<ron::Value, _> = ron::from_str(&text);
        assert!(parsed.is_ok(), "{}: {:?}", path.display(), parsed.err());
    }
}
