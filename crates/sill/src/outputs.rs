//! Which surfaces exist on which output, and the pure reconciliation from the host's output
//! list to spawn and close actions.

use shell_host::{LayerSize, OutputId, OutputInfo, SurfaceId};
use sill_ui_kit::surface::Surface;

/// A layer surface spawned through `Surface`, as `daemon::surfaces` records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spawned {
    /// `Surface::NAME` of the kind.
    pub name: &'static str,
    /// The slot, as its `Debug` text (the row holds many kinds, so the slot type is erased).
    pub slot: String,
    pub id: SurfaceId,
    /// The layer namespace, which `debug stats` reports.
    pub namespace: String,
    /// The size last asked of the compositor.
    pub size: LayerSize,
}

/// One output's surfaces: every one is spawned through `Surface`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutputSurfaces {
    /// Every surface spawned through `Surface`, in spawn order.
    pub spawned: Vec<Spawned>,
}

impl OutputSurfaces {
    /// Whether any surface of this row is `id`.
    pub fn contains(&self, id: SurfaceId) -> bool {
        self.ids().any(|held| held == id)
    }

    /// The id of `S`'s surface in this row, if one is spawned. A slotted kind has several; this
    /// is the first.
    pub fn surface_of<S: Surface>(&self) -> Option<SurfaceId> {
        self.spawned
            .iter()
            .find(|surface| surface.name == S::NAME)
            .map(|surface| surface.id)
    }

    /// Every surface actually spawned, for closing them all.
    pub fn ids(&self) -> impl Iterator<Item = SurfaceId> {
        self.spawned.iter().map(|surface| surface.id)
    }

    /// Forget `id`, whichever surface held it.
    pub fn clear(&mut self, id: SurfaceId) {
        self.spawned.retain(|surface| surface.id != id);
    }
}

/// Every output's surfaces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outputs {
    pub outputs: Vec<(OutputInfo, OutputSurfaces)>,
}

impl Outputs {
    /// `info`'s surfaces, if this output is known.
    pub fn get(&self, id: OutputId) -> Option<&OutputSurfaces> {
        self.outputs
            .iter()
            .find(|(info, _)| info.id == id)
            .map(|(_, surfaces)| surfaces)
    }

    /// Insert or replace `info`'s row with `surfaces`.
    pub fn set(&mut self, info: OutputInfo, surfaces: OutputSurfaces) {
        let existing = self.outputs.iter_mut().find(|(row, _)| row.id == info.id);
        match existing {
            Some(row) => *row = (info, surfaces),
            None => self.outputs.push((info, surfaces)),
        }
    }

    /// Remove `id`'s row, returning its surfaces.
    pub fn remove(&mut self, id: OutputId) -> Option<OutputSurfaces> {
        let index = self.outputs.iter().position(|(info, _)| info.id == id)?;
        Some(self.outputs.remove(index).1)
    }

    /// The row whose surfaces contain `id`, if any.
    pub fn row_with(&mut self, id: SurfaceId) -> Option<&mut (OutputInfo, OutputSurfaces)> {
        self.outputs
            .iter_mut()
            .find(|(_, surfaces)| surfaces.contains(id))
    }
}

/// What reconciling asks the daemon to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputAction {
    SpawnAll(OutputInfo),
    /// Close every surface of a removed output.
    CloseAll(OutputId),
    /// Size or scale changed: re-apply sizes, re-prepare the wallpaper.
    Resize(OutputInfo),
}

/// The actions that turn `current` into one set of surfaces per output in `now`.
pub fn reconcile(current: &Outputs, now: &[OutputInfo]) -> Vec<OutputAction> {
    let existing = |id| current.outputs.iter().find(|(info, _)| info.id == id);
    let mut actions: Vec<OutputAction> = now
        .iter()
        .filter_map(|info| match existing(info.id) {
            None => Some(OutputAction::SpawnAll(info.clone())),
            Some((old, _)) if old == info => None,
            Some(_) => Some(OutputAction::Resize(info.clone())),
        })
        .collect();
    actions.extend(
        current
            .outputs
            .iter()
            .filter(|(info, _)| !now.iter().any(|kept| kept.id == info.id))
            .map(|(info, _)| OutputAction::CloseAll(info.id)),
    );
    actions
}

#[cfg(test)]
mod tests {
    use super::{OutputSurfaces, Outputs, Spawned, reconcile};
    use shell_host::{LayerExtent, LayerSize, OutputId, OutputInfo, Rect, Scale, SurfaceId};
    use sill_overlays::HotCorners;
    use sill_ui_kit::surface::Surface;

    fn output(id: u32, name: &str, width: i32) -> OutputInfo {
        OutputInfo {
            id: OutputId(id),
            name: Some(name.to_owned()),
            description: None,
            logical: Rect {
                x: 0,
                y: 0,
                width,
                height: 1080,
            },
            scale: Scale(120),
            refresh_mhz: 60_000,
        }
    }

    fn spawned(name: &'static str, id: u64) -> Spawned {
        Spawned {
            name,
            slot: "BottomRight".to_owned(),
            id: SurfaceId(id),
            namespace: format!("sill-{name}-DP-1"),
            size: LayerSize {
                width: LayerExtent::Px(2),
                height: LayerExtent::Px(2),
            },
        }
    }

    fn surfaces(id: u64) -> OutputSurfaces {
        OutputSurfaces {
            spawned: vec![spawned("bar", id)],
        }
    }

    #[test]
    fn spawned_surfaces_are_found_and_forgotten() {
        let mut row = surfaces(1);
        row.spawned.push(spawned(HotCorners::NAME, 3));
        assert!(row.contains(SurfaceId(1)));
        assert!(row.contains(SurfaceId(3)));
        assert_eq!(row.surface_of::<HotCorners>(), Some(SurfaceId(3)));
        assert_eq!(row.ids().count(), 2);
        row.clear(SurfaceId(3));
        assert!(!row.contains(SurfaceId(3)));
        assert_eq!(row.surface_of::<HotCorners>(), None);
        assert_eq!(row.ids().count(), 1);
    }

    #[test]
    fn a_new_output_is_spawned() {
        let current = Outputs::default();
        let now = [output(1, "DP-1", 1920)];
        assert_eq!(
            reconcile(&current, &now),
            vec![super::OutputAction::SpawnAll(now[0].clone())]
        );
    }

    #[test]
    fn an_unchanged_output_asks_for_nothing() {
        let info = output(1, "DP-1", 1920);
        let mut current = Outputs::default();
        current.set(info.clone(), surfaces(1));
        assert_eq!(reconcile(&current, &[info]), vec![]);
    }

    #[test]
    fn a_changed_output_is_resized() {
        let old = output(1, "DP-1", 1920);
        let new = output(1, "DP-1", 2560);
        let mut current = Outputs::default();
        current.set(old, surfaces(1));
        assert_eq!(
            reconcile(&current, std::slice::from_ref(&new)),
            vec![super::OutputAction::Resize(new)]
        );
    }

    #[test]
    fn a_removed_output_is_closed() {
        let info = output(1, "DP-1", 1920);
        let mut current = Outputs::default();
        current.set(info, surfaces(1));
        assert_eq!(
            reconcile(&current, &[]),
            vec![super::OutputAction::CloseAll(OutputId(1))]
        );
    }

    #[test]
    fn adds_changes_and_removals_combine() {
        let kept = output(1, "DP-1", 1920);
        let removed = output(2, "HDMI-A-1", 1280);
        let added = output(3, "DP-2", 1920);
        let mut current = Outputs::default();
        current.set(kept.clone(), surfaces(1));
        current.set(removed, surfaces(2));
        assert_eq!(
            reconcile(&current, &[kept, added.clone()]),
            vec![
                super::OutputAction::SpawnAll(added),
                super::OutputAction::CloseAll(OutputId(2)),
            ]
        );
    }
}
