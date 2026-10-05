//! The pure half: `kcminputrc` text to cosmic-comp's `input_devices` map, and the map to RON.
//!
//! Plasma saves a device's settings under `[Libinput][<vendor>][<product>][<name>]` and writes only
//! the keys that differ from libinput's default for that device. Read here: `NaturalScroll`,
//! `ScrollFactor`, `PointerAcceleration` and the profile (`PointerAccelerationProfileFlat` /
//! `PointerAccelerationProfileAdaptive`). A device with any of them is imported with natural
//! scroll stated outright: a key Plasma left out means libinput's default, which is "off", and
//! the seeded `input_default` turns it on for every device that does not say.

use std::collections::BTreeMap;

use ron::ser::PrettyConfig;

use super::wire::{AccelConfig, AccelProfile, DeviceState, InputConfig, ScrollConfig};

/// The import of every device group in `kcminputrc`, by device name. A name that appears under
/// two vendor/product pairs takes the later group.
pub fn import(kcminputrc: &str) -> BTreeMap<String, InputConfig> {
    let mut groups: Vec<(String, Vec<(&str, &str)>)> = Vec::new();
    for line in kcminputrc.lines().map(str::trim) {
        if line.starts_with('[') {
            groups.push((line.to_owned(), Vec::new()));
        } else if let (Some((_, entries)), Some((key, value))) =
            (groups.last_mut(), line.split_once('='))
        {
            entries.push((key.trim(), value.trim()));
        }
    }
    groups
        .iter()
        .filter_map(|(header, entries)| Some((device_name(header)?, device_config(entries)?)))
        .collect()
}

/// `[Libinput][1][2][name]` to `name`; anything else is not a device group. The name keeps its
/// spaces: libinput reports the kernel's name as it is, and cosmic-comp matches it exactly.
fn device_name(header: &str) -> Option<String> {
    let rest = header.strip_prefix("[Libinput]")?;
    let rest = number_group(rest)?;
    let rest = number_group(rest)?;
    let name = rest.strip_prefix('[')?.strip_suffix(']')?;
    Some(name.replace("\\x5b", "[").replace("\\x5d", "]")).filter(|name| !name.is_empty())
}

/// `[123]rest` to `rest`.
fn number_group(text: &str) -> Option<&str> {
    let (digits, rest) = text.strip_prefix('[')?.split_once(']')?;
    (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())).then_some(rest)
}

fn device_config(entries: &[(&str, &str)]) -> Option<InputConfig> {
    let value = |key: &str| {
        entries
            .iter()
            .rev()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
    };
    let toggle = |key: &str| match value(key)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    };
    let number = |key: &str| value(key)?.parse::<f64>().ok().filter(|n| n.is_finite());
    let natural = toggle("NaturalScroll");
    let factor = number("ScrollFactor");
    let speed = number("PointerAcceleration");
    let profile = match (
        toggle("PointerAccelerationProfileFlat"),
        toggle("PointerAccelerationProfileAdaptive"),
    ) {
        (Some(true), _) | (None, Some(false)) => Some(AccelProfile::Flat),
        (_, Some(true)) | (Some(false), None) => Some(AccelProfile::Adaptive),
        (None, None) | (Some(false), Some(false)) => None,
    };
    if natural.is_none() && factor.is_none() && speed.is_none() && profile.is_none() {
        return None;
    }
    Some(InputConfig {
        state: DeviceState::Enabled,
        acceleration: (speed.is_some() || profile.is_some()).then(|| AccelConfig {
            profile,
            speed: speed.unwrap_or(0.0),
        }),
        scroll_config: Some(ScrollConfig {
            method: None,
            natural_scroll: Some(natural.unwrap_or(false)),
            scroll_button: None,
            scroll_factor: factor,
        }),
    })
}

/// The file's text: the RON cosmic-config reads for `input_devices`.
pub fn render(devices: &BTreeMap<String, InputConfig>) -> Result<String, ron::Error> {
    ron::ser::to_string_pretty(devices, PrettyConfig::default())
}

#[cfg(test)]
#[path = "plasma_tests.rs"]
mod tests;
