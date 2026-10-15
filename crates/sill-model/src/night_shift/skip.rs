//! Which outputs casement is told to leave alone: the monitors sill warms itself.

use std::collections::BTreeSet;

use super::Backend;
use crate::displays::ddc::DdcDisplay;
use crate::ids::OutputName;

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
