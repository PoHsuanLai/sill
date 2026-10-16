//! Which outputs casement is told to leave alone: the monitors sill warms itself.

use std::collections::BTreeSet;

use super::Backend;
use crate::displays::ddc::DdcDisplay;
use crate::ids::{DdcId, OutputName};

/// The connectors casement's night light must not warm (`night_light.skip_outputs`): where sill
/// warms the monitors itself, each one that answers DDC/CI with RGB gains (the ones sill's warmth
/// writes reach) and whose connector is known; otherwise none. Sorted, each connector once.
pub fn skip_outputs(backend: Backend, ddc: &[DdcDisplay]) -> Vec<OutputName> {
    match backend {
        Backend::Software => Vec::new(),
        Backend::Hardware => ddc
            .iter()
            .filter(|display| display.controls.gain.is_some())
            .filter_map(|display| display.endpoint.connector.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    }
}

/// The monitors sill warms whose connector it could not match: casement's software light warms
/// them too (it cannot be told to skip what sill cannot name). Sorted by bus.
pub fn unmatched(backend: Backend, ddc: &[DdcDisplay]) -> Vec<DdcId> {
    match backend {
        Backend::Software => Vec::new(),
        Backend::Hardware => ddc
            .iter()
            .filter(|display| display.controls.gain.is_some())
            .filter(|display| display.endpoint.connector.is_none())
            .map(|display| display.endpoint.id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    }
}
