//! The idle dim: before the displays go off for idleness, every output gets a black scrim at
//! `100 - idle.dim_level_pct` percent, quire's `IdleDim`. It is an overlay, never a brightness
//! change, so a crash while dimmed leaves nothing dark.
//!
//! It follows the idle service's published `Idle.dim` alone: `Dimmed(level)` fades the scrim in
//! over `--t-idle-dim`, `Clear` (any input, the displays going off, an inhibitor) snaps it out.
//! The surface ([`idle_dim_layer`]) is on the Overlay layer, so the session-lock surfaces, which
//! the compositor always stacks above every layer surface, cover it; its input region is empty
//! and it takes no keyboard, so it never counts as activity or steals a click. The daemon maps
//! it only while dimmed ([`visibility_of`]), so an awake output draws nothing at all.

pub mod layer;

pub use layer::idle_dim_layer;

use dioxus::prelude::*;
use ds::detail::IdleDimPhase;
use ds::{Ds, IdleDim, Material, RootChrome, RootExtent};
use shell_host::{OutputInfo, Visibility};
use sill_services::idle::{DimLevel, DimShown};

use crate::hooks::{use_appearance, use_ds_scale, use_idle};

/// What the scrim draws: its opacity and whether it is dimming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DimScene {
    /// The scrim's opacity at rest while dimmed, quire's `IdleDim.level`.
    pub level: ds::Percent,
    pub phase: IdleDimPhase,
}

/// The scrim's opacity for a display kept at `kept` percent of its brightness
/// (`idle.dim_level_pct`, 10 to 90, held there again here): the rest is covered.
pub fn scrim_of(kept: DimLevel) -> ds::Percent {
    let DimLevel(kept) = kept;
    ds::Percent(100 - kept.clamp(10, 90))
}

/// The scene for `dim`. Clear keeps the last level, so waking only moves the phase and quire
/// snaps the scrim out instead of reading a level edit.
pub fn scene_of(dim: DimShown, last: ds::Percent) -> DimScene {
    match dim {
        DimShown::Clear => DimScene {
            level: last,
            phase: IdleDimPhase::Awake,
        },
        DimShown::Dimmed(kept) => DimScene {
            level: scrim_of(kept),
            phase: IdleDimPhase::Dimmed,
        },
    }
}

/// Whether the daemon maps the surface: only while the service dims.
pub fn visibility_of(dim: DimShown) -> Visibility {
    match dim {
        DimShown::Clear => Visibility::Hidden,
        DimShown::Dimmed(_) => Visibility::Shown,
    }
}

/// The dim on `output`: quire's `IdleDim` alone in a transparent viewport root, following
/// `use_idle().dim`.
#[component]
pub fn IdleDimOverlay(output: OutputInfo) -> Element {
    let _ = output;
    let dim = use_idle()().dim;
    let environment = use_appearance()();
    let ds_scale = use_ds_scale();
    let mut last = use_hook(|| CopyValue::new(scrim_of(DimLevel(50))));
    let scene = scene_of(dim, *last.peek());
    if *last.peek() != scene.level {
        last.set(scene.level);
    }
    rsx! {
        Ds {
            scale: ds_scale,
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            material: Material::Sheet,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            IdleDim { level: scene.level, phase: scene.phase }
        }
    }
}

#[cfg(test)]
mod tests {
    use ds::detail::IdleDimPhase;
    use shell_host::Visibility;
    use sill_services::idle::{DimLevel, DimShown};

    use super::{DimScene, scene_of, scrim_of, visibility_of};

    #[test]
    fn the_scrim_covers_what_the_display_does_not_keep() {
        const CASES: &[(u8, u8)] = &[(50, 50), (10, 90), (90, 10), (70, 30), (0, 90), (100, 10)];
        for &(kept, want) in CASES {
            assert_eq!(scrim_of(DimLevel(kept)), ds::Percent(want), "{kept}");
        }
    }

    #[test]
    fn the_scene_and_the_mapping_follow_the_published_dim() {
        let last = ds::Percent(40);
        let cases = [
            (
                DimShown::Clear,
                DimScene {
                    level: last,
                    phase: IdleDimPhase::Awake,
                },
                Visibility::Hidden,
            ),
            (
                DimShown::Dimmed(DimLevel(30)),
                DimScene {
                    level: ds::Percent(70),
                    phase: IdleDimPhase::Dimmed,
                },
                Visibility::Shown,
            ),
        ];
        for (dim, scene, shown) in cases {
            assert_eq!(scene_of(dim, last), scene, "{dim:?}");
            assert_eq!(visibility_of(dim), shown, "{dim:?}");
        }
    }
}
