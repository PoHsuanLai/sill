//! Which surfaces exist on which output, and the pure reconciliation from the host's output
//! list to spawn and close actions.

use std::sync::Arc;

use shell_host::{KeyboardFocus, LayerSize, OutputId, OutputInfo, SurfaceId};
use sill_model::desktop::ShellRole;
use sill_model::ids::OutputName;
use sill_ui_kit::services::Services;
use sill_ui_kit::surface::{FocusLoss, OutputPolicy, Surface};

/// What a surface does when its keyboard focus changes, made from its `Surface::KEYBOARD` and its
/// link at spawn time, so the daemon tells every surface the same way without knowing its kind.
#[derive(Clone, Default)]
pub struct FocusRoute(Option<Arc<dyn Fn(KeyboardFocus) + Send + Sync>>);

impl FocusRoute {
    /// The route of `S`'s surface on `output`, holding `link`.
    pub fn of<S: Surface>(link: &S::Link, output: OutputId) -> FocusRoute {
        match S::KEYBOARD {
            FocusLoss::Ignore => FocusRoute(None),
            FocusLoss::CloseLink(close) => {
                let link = link.clone();
                FocusRoute(Some(Arc::new(move |focus| close(&link, output, focus))))
            }
        }
    }

    /// The surface's keyboard focus changed.
    pub fn tell(&self, focus: KeyboardFocus) {
        if let Some(route) = &self.0 {
            route(focus);
        }
    }
}

impl std::fmt::Debug for FocusRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            Some(_) => "FocusRoute(closes)",
            None => "FocusRoute(ignores)",
        })
    }
}

impl PartialEq for FocusRoute {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for FocusRoute {}

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
    /// The role the compositor knows it by (`Surface::ROLE`).
    pub role: Option<ShellRole>,
    /// What its keyboard focus changes do (`Surface::KEYBOARD`).
    pub focus: FocusRoute,
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

    /// The surface `id`, if this row holds it.
    pub fn spawned_by(&self, id: SurfaceId) -> Option<&Spawned> {
        self.spawned.iter().find(|surface| surface.id == id)
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

    /// The output called `name`, when it is connected.
    pub fn named(&self, name: &str) -> Option<OutputId> {
        self.outputs
            .iter()
            .find(|(info, _)| info.name.as_deref() == Some(name))
            .map(|(info, _)| info.id)
    }

    /// The output `policy` names. This is the one place that decides it, for the daemon's
    /// spawns and for every opener of a panel (verbs, chords, the companion, the notifications).
    ///
    /// A panel opens on the focused window's output (`focus`). The rule wants the pointer's
    /// output next, but the daemon does not know the pointer's output, so the fallback, and the
    /// main display, is the first output the compositor announced.
    pub fn pick(&self, policy: OutputPolicy, focus: Option<&OutputName>) -> Option<OutputId> {
        let focused = focus.and_then(|focus| {
            self.outputs
                .iter()
                .find(|(info, _)| info.name.as_deref() == Some(focus.0.as_str()))
        });
        match policy {
            OutputPolicy::FocusedWindow => focused.or_else(|| self.outputs.first()),
            OutputPolicy::Main | OutputPolicy::Every => self.outputs.first(),
        }
        .map(|(info, _)| info.id)
    }

    /// The output a panel of kind `S` opens on.
    pub fn open_on<S: Surface>(&self, services: &Services) -> Option<OutputId> {
        let focus = services.windows.state.borrow().toplevels.focused_output();
        self.pick(S::OUTPUTS, focus.as_ref())
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
    use super::{FocusRoute, OutputSurfaces, Outputs, Spawned, reconcile};
    use shell_host::{LayerExtent, LayerSize, OutputId, OutputInfo, Rect, Scale120, SurfaceId};
    use sill_model::ids::OutputName;
    use sill_overlays::HotCorners;
    use sill_ui_kit::surface::OutputPolicy;
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
            scale: Scale120(120),
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
            role: None,
            focus: FocusRoute::default(),
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
    fn a_panel_opens_where_the_focused_window_is_else_on_the_first_output() {
        let mut outputs = Outputs::default();
        outputs.set(output(1, "DP-1", 1920), OutputSurfaces::default());
        outputs.set(output(2, "HDMI-A-1", 1920), OutputSurfaces::default());
        let on = |name: &str| Some(OutputName(name.to_owned()));
        let cases = [
            (
                "the focused window's output",
                OutputPolicy::FocusedWindow,
                on("HDMI-A-1"),
                2,
            ),
            (
                "the first when it is there",
                OutputPolicy::FocusedWindow,
                on("DP-1"),
                1,
            ),
            (
                "the first with no focused window",
                OutputPolicy::FocusedWindow,
                None,
                1,
            ),
            (
                "the first for an output it does not know",
                OutputPolicy::FocusedWindow,
                on("eDP-1"),
                1,
            ),
            (
                "the main display ignores the focus",
                OutputPolicy::Main,
                on("HDMI-A-1"),
                1,
            ),
        ];
        for (name, policy, focus, want) in cases {
            assert_eq!(
                outputs.pick(policy, focus.as_ref()),
                Some(OutputId(want)),
                "{name}"
            );
        }
        assert_eq!(
            Outputs::default().pick(OutputPolicy::Main, None),
            None,
            "no outputs"
        );
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
