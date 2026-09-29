//! The effects end of the dim: follow the idle service's published `Idle.dim` and keep the last
//! scrim level, so a wake changes only the phase.

use dioxus::prelude::*;
use sill_model::idle::DimLevel;
use sill_services::IdleService;

use super::model::{DimScene, scene_of, scrim_of};
use crate::hooks::use_service;

/// The scene the scrim draws now: what `Idle.dim` says, the last level kept across a `Clear`.
pub(super) fn use_dim() -> DimScene {
    let dim = use_service::<IdleService>()().dim;
    let mut last = use_hook(|| CopyValue::new(scrim_of(DimLevel(50))));
    let scene = scene_of(dim, *last.peek());
    if *last.peek() != scene.level {
        last.set(scene.level);
    }
    scene
}
