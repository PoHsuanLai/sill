//! Which surfaces exist on which output, and the pure reconciliation from the host's output
//! list to spawn and close actions.

use shell_host::{OutputId, OutputInfo, SurfaceId};
use sill_surfaces::links::Corner;

/// One output's surfaces (each `None` until spawned).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutputSurfaces {
    pub bar: Option<SurfaceId>,
    pub wallpaper: Option<SurfaceId>,
    pub dock: Option<SurfaceId>,
    pub launcher: Option<SurfaceId>,
    pub catcher: Option<SurfaceId>,
    pub osd: Option<SurfaceId>,
    /// The power menu, warm and hidden.
    pub power_menu: Option<SurfaceId>,
    /// The banner column and the notification center, warm and hidden.
    pub banners: Option<SurfaceId>,
    pub notification_center: Option<SurfaceId>,
    /// The screenshot thumbnail (warm and hidden), the desktop widget layer (while
    /// `widgets.desktop` is on), and a surface per active hot corner.
    pub thumbnail: Option<SurfaceId>,
    pub desktop_widgets: Option<SurfaceId>,
    pub corners: CornerSurfaces,
    /// The polkit prompt and the app switcher, warm and hidden. The lock surfaces are
    /// shell-host's session lock's, not a row's.
    pub polkit_prompt: Option<SurfaceId>,
    pub switcher: Option<SurfaceId>,
    /// Edit Widgets (the widget gallery), warm and hidden.
    pub edit_widgets: Option<SurfaceId>,
    /// The idle dim, warm and mapped only while the idle service dims.
    pub idle_dim: Option<SurfaceId>,
}

/// One output's hot corner surfaces, one per corner whose action is not `None`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CornerSurfaces {
    pub top_left: Option<SurfaceId>,
    pub top_right: Option<SurfaceId>,
    pub bottom_left: Option<SurfaceId>,
    pub bottom_right: Option<SurfaceId>,
}

impl CornerSurfaces {
    /// `corner`'s surface.
    pub fn get(&self, corner: Corner) -> Option<SurfaceId> {
        *self.slot(corner)
    }

    /// Set (or forget, with `None`) `corner`'s surface.
    pub fn set(&mut self, corner: Corner, id: Option<SurfaceId>) {
        *self.slot_mut(corner) = id;
    }

    fn slot(&self, corner: Corner) -> &Option<SurfaceId> {
        match corner {
            Corner::TopLeft => &self.top_left,
            Corner::TopRight => &self.top_right,
            Corner::BottomLeft => &self.bottom_left,
            Corner::BottomRight => &self.bottom_right,
        }
    }

    fn slot_mut(&mut self, corner: Corner) -> &mut Option<SurfaceId> {
        match corner {
            Corner::TopLeft => &mut self.top_left,
            Corner::TopRight => &mut self.top_right,
            Corner::BottomLeft => &mut self.bottom_left,
            Corner::BottomRight => &mut self.bottom_right,
        }
    }
}

impl OutputSurfaces {
    /// Every field, for code that treats them uniformly (closing them all, searching by id).
    fn fields(&self) -> [Option<SurfaceId>; 19] {
        [
            self.bar,
            self.wallpaper,
            self.dock,
            self.launcher,
            self.catcher,
            self.osd,
            self.power_menu,
            self.banners,
            self.notification_center,
            self.thumbnail,
            self.desktop_widgets,
            self.corners.top_left,
            self.corners.top_right,
            self.corners.bottom_left,
            self.corners.bottom_right,
            self.polkit_prompt,
            self.switcher,
            self.edit_widgets,
            self.idle_dim,
        ]
    }

    /// Whether any field of this row is `id`.
    pub fn contains(&self, id: SurfaceId) -> bool {
        self.fields().contains(&Some(id))
    }

    /// Every surface actually spawned, for closing them all.
    pub fn ids(&self) -> impl Iterator<Item = SurfaceId> {
        self.fields().into_iter().flatten()
    }

    /// Forget `id`, whichever field held it.
    pub fn clear(&mut self, id: SurfaceId) {
        for field in [
            &mut self.bar,
            &mut self.wallpaper,
            &mut self.dock,
            &mut self.launcher,
            &mut self.catcher,
            &mut self.osd,
            &mut self.power_menu,
            &mut self.banners,
            &mut self.notification_center,
            &mut self.thumbnail,
            &mut self.desktop_widgets,
            &mut self.corners.top_left,
            &mut self.corners.top_right,
            &mut self.corners.bottom_left,
            &mut self.corners.bottom_right,
            &mut self.polkit_prompt,
            &mut self.switcher,
            &mut self.edit_widgets,
            &mut self.idle_dim,
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
/// addresses it by. Mirrors `sill_surfaces::surfaces::naming::namespace("bar", output)`, which is
/// `pub(crate)` to that crate: `sill_surfaces::surfaces::bar::bar_layer` sets a spawned bar's
/// namespace this way, so the daemon's debug lookups must agree with it exactly.
pub fn bar_namespace(info: &OutputInfo) -> String {
    match &info.name {
        Some(name) => format!("sill-bar-{name}"),
        None => format!("sill-bar-{}", info.id.0),
    }
}

/// The wallpaper's own namespace on `info` (mirrors `bar_namespace`; the wallpaper's own
/// `sill_surfaces::surfaces::wallpaper::wallpaper_layer` sets it the same way).
pub fn wallpaper_namespace(info: &OutputInfo) -> String {
    match &info.name {
        Some(name) => format!("sill-wallpaper-{name}"),
        None => format!("sill-wallpaper-{}", info.id.0),
    }
}

#[cfg(test)]
mod tests {
    use super::{OutputSurfaces, Outputs, bar_namespace, reconcile, wallpaper_namespace};
    use shell_host::{OutputId, OutputInfo, Rect, Scale, SurfaceId};

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
    fn m10_surfaces_are_found_and_forgotten_like_the_rest() {
        use sill_surfaces::links::Corner;
        let mut row = surfaces(1);
        row.thumbnail = Some(SurfaceId(2));
        row.corners.set(Corner::BottomRight, Some(SurfaceId(3)));
        assert!(row.contains(SurfaceId(2)));
        assert_eq!(row.corners.get(Corner::BottomRight), Some(SurfaceId(3)));
        assert_eq!(row.ids().count(), 3);
        row.clear(SurfaceId(3));
        assert_eq!(row.corners.get(Corner::BottomRight), None);
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

    #[test]
    fn the_wallpaper_namespace_uses_the_connector_name_or_falls_back_to_the_id() {
        assert_eq!(
            wallpaper_namespace(&output(1, "DP-1", 1920)),
            "sill-wallpaper-DP-1"
        );
        let mut nameless = output(7, "unused", 1920);
        nameless.name = None;
        assert_eq!(wallpaper_namespace(&nameless), "sill-wallpaper-7");
    }
}
