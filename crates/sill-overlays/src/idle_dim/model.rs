//! What the scrim draws for a published dim, and whether the daemon maps the surface.

use ds_shell::IdleDimPhase;
use shell_host::Visibility;
use sill_model::idle::{DimLevel, DimShown};

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

#[cfg(test)]
mod tests {
    use ds_shell::IdleDimPhase;
    use shell_host::Visibility;
    use sill_model::idle::{DimLevel, DimShown};

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
