//! Which surfaces exist on which output, and the pure reconciliation from the host's output
//! list to spawn and close actions.

use shell_host::{LayerSize, OutputId, OutputInfo, SurfaceId};

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

/// One output's surfaces (each `None` until spawned).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutputSurfaces {
    /// Every surface spawned through `Surface`, in spawn order.
    pub spawned: Vec<Spawned>,
    pub bar: Option<SurfaceId>,
    pub dock: Option<SurfaceId>,
    pub launcher: Option<SurfaceId>,
    pub catcher: Option<SurfaceId>,
    /// The banner column and the notification center, warm and hidden.
    pub banners: Option<SurfaceId>,
    pub notification_center: Option<SurfaceId>,
    /// The screenshot thumbnail (warm and hidden) and the desktop widget layer (while
    /// `widgets.desktop` is on).
    pub thumbnail: Option<SurfaceId>,
    pub desktop_widgets: Option<SurfaceId>,
    /// Edit Widgets (the widget gallery), warm and hidden.
    pub edit_widgets: Option<SurfaceId>,
}

impl OutputSurfaces {
    /// Every field, for code that treats them uniformly (closing them all, searching by id).
    fn fields(&self) -> [Option<SurfaceId>; 9] {
        [
            self.bar,
            self.dock,
            self.launcher,
            self.catcher,
            self.banners,
            self.notification_center,
            self.thumbnail,
            self.desktop_widgets,
            self.edit_widgets,
        ]
    }

    /// The id of this row's spawned surface `name` (a `Surface::NAME`), if it has one. A slotted
    /// kind has several; this is the first.
    pub fn spawned_id(&self, name: &str) -> Option<SurfaceId> {
        self.spawned
            .iter()
            .find(|surface| surface.name == name)
            .map(|surface| surface.id)
    }

    /// Whether any surface of this row is `id`.
    pub fn contains(&self, id: SurfaceId) -> bool {
        self.ids().any(|held| held == id)
    }

    /// Every surface actually spawned, for closing them all.
    pub fn ids(&self) -> impl Iterator<Item = SurfaceId> {
        let spawned: Vec<SurfaceId> = self.spawned.iter().map(|surface| surface.id).collect();
        self.fields().into_iter().flatten().chain(spawned)
    }

    /// Forget `id`, whichever surface held it.
    pub fn clear(&mut self, id: SurfaceId) {
        self.spawned.retain(|surface| surface.id != id);
        for field in [
            &mut self.bar,
            &mut self.dock,
            &mut self.launcher,
            &mut self.catcher,
            &mut self.banners,
            &mut self.notification_center,
            &mut self.thumbnail,
            &mut self.desktop_widgets,
            &mut self.edit_widgets,
        ] {
            if *field == Some(id) {
                *field = None;
            }
        }
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

/// The layer-shell namespace the bar on `info` gets (`sill-bar-DP-1`), and the name a debug request
/// addresses it by. Mirrors `sill_ui_kit::surfaces::naming::namespace("bar", output)`, which is
/// `pub(crate)` to that crate: `sill_ui_kit::surfaces::bar::bar_layer` sets a spawned bar's
/// namespace this way, so the daemon's debug lookups must agree with it exactly.
pub fn bar_namespace(info: &OutputInfo) -> String {
    match &info.name {
        Some(name) => format!("sill-bar-{name}"),
        None => format!("sill-bar-{}", info.id.0),
    }
}

#[cfg(test)]
mod tests {
    use super::{OutputSurfaces, Outputs, Spawned, bar_namespace, reconcile};
    use shell_host::{LayerExtent, LayerSize, OutputId, OutputInfo, Rect, Scale, SurfaceId};

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

    fn surfaces(bar: u64) -> OutputSurfaces {
        OutputSurfaces {
            bar: Some(SurfaceId(bar)),
            ..OutputSurfaces::default()
        }
    }

    #[test]
    fn spawned_surfaces_are_found_and_forgotten_like_the_rest() {
        let mut row = surfaces(1);
        row.thumbnail = Some(SurfaceId(2));
        row.spawned.push(Spawned {
            name: "hot-corner",
            slot: "BottomRight".to_owned(),
            id: SurfaceId(3),
            namespace: "sill-hot-corner-bottom-right-DP-1".to_owned(),
            size: LayerSize {
                width: LayerExtent::Px(2),
                height: LayerExtent::Px(2),
            },
        });
        assert!(row.contains(SurfaceId(2)));
        assert!(row.contains(SurfaceId(3)));
        assert_eq!(row.ids().count(), 3);
        row.clear(SurfaceId(3));
        assert!(!row.contains(SurfaceId(3)));
        assert_eq!(row.ids().count(), 2);
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

    #[test]
    fn the_bar_namespace_uses_the_connector_name_or_falls_back_to_the_id() {
        assert_eq!(bar_namespace(&output(1, "DP-1", 1920)), "sill-bar-DP-1");
        let mut nameless = output(7, "unused", 1920);
        nameless.name = None;
        assert_eq!(bar_namespace(&nameless), "sill-bar-7");
    }
}
