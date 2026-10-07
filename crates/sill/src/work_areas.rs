//! Each output's work area, worked out from what the shell has spawned: the output less the
//! exclusive zones of its bar and dock (the same layer configurations the surfaces are spawned
//! with, so the numbers cannot drift from what the compositor reserves), and published to the
//! work-areas service for `org.quire.Outputs1`.

use shell_host::{Anchor, ExclusiveZone, LayerConfig, OutputInfo, Rect as HostRect};
use sill_bar::Bar;
use sill_dock::Dock;
use sill_model::work_areas::{Edge, Rect, WorkArea, WorkAreaCommand, Zone, work_area};
use sill_settings::ShellFile;
use sill_ui_kit::services::Services;
use sill_ui_kit::surface::Surface;

use crate::outputs::{OutputSurfaces, Outputs};

/// The strip `config` keeps clear, if it keeps one: it reserves pixels along exactly one edge
/// (the compositor ignores a zone on a surface anchored to a corner, or to two opposite edges).
pub fn zone_of(config: &LayerConfig) -> Option<Zone> {
    let ExclusiveZone::Reserve(px) = config.exclusive else {
        return None;
    };
    let lone = |edge: Anchor, opposite: Anchor| {
        config.anchor.contains(edge) && !config.anchor.contains(opposite)
    };
    let edges = [
        (lone(Anchor::TOP, Anchor::BOTTOM), Edge::Top),
        (lone(Anchor::BOTTOM, Anchor::TOP), Edge::Bottom),
        (lone(Anchor::LEFT, Anchor::RIGHT), Edge::Left),
        (lone(Anchor::RIGHT, Anchor::LEFT), Edge::Right),
    ];
    let mut anchored = edges.iter().filter(|(on, _)| *on);
    match (anchored.next(), anchored.next()) {
        (Some((_, edge)), None) if px > 0 => Some(Zone { edge: *edge, px }),
        _ => None,
    }
}

fn rect(rect: HostRect) -> Rect {
    Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width.unsigned_abs(),
        height: rect.height.unsigned_abs(),
    }
}

/// The zones of the panels spawned on `surfaces`.
fn zones(info: &OutputInfo, surfaces: &OutputSurfaces, settings: &ShellFile) -> Vec<Zone> {
    let bar = surfaces
        .surface_of::<Bar>()
        .map(|_| Bar::layer(info, settings, ()));
    let dock = surfaces
        .surface_of::<Dock>()
        .map(|_| Dock::layer(info, settings, ()));
    [bar, dock].iter().flatten().filter_map(zone_of).collect()
}

/// Every output's work area, in the host's order.
pub fn areas(outputs: &Outputs, settings: &ShellFile) -> Vec<WorkArea> {
    outputs
        .outputs
        .iter()
        .map(|(info, surfaces)| {
            work_area(
                info.id.0,
                info.name.as_deref().unwrap_or_default(),
                rect(info.logical),
                info.scale.0,
                &zones(info, surfaces, settings),
            )
        })
        .collect()
}

/// Tell the service the areas as they now are; nothing is sent when it already holds them.
pub fn publish(services: &Services, outputs: &Outputs, settings: &ShellFile) {
    let now = areas(outputs, settings);
    if services.work_areas.state.borrow().areas == now {
        return;
    }
    // A stopped service has nobody to tell.
    let _ = services
        .work_areas
        .control
        .send(WorkAreaCommand::Publish(now));
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_host::{BlurSpec, RegionSpec};
    use shell_host::{KeyboardMode, LayerExtent, LayerLevel, LayerSize, Margin, OutputTarget};

    fn config(anchor: Anchor, exclusive: ExclusiveZone) -> LayerConfig {
        LayerConfig {
            namespace: String::new(),
            layer: LayerLevel::Top,
            anchor,
            size: LayerSize {
                width: LayerExtent::Fill,
                height: LayerExtent::Fill,
            },
            exclusive,
            margin: Margin::ZERO,
            keyboard: KeyboardMode::None,
            output: OutputTarget::CompositorPicks,
            blur: BlurSpec::None,
            input: RegionSpec::Whole,
        }
    }

    #[test]
    fn a_panel_reserves_along_the_one_edge_it_hugs() {
        let reserve = ExclusiveZone::Reserve(32);
        let zone = |edge| Some(Zone { edge, px: 32 });
        // name, anchor, exclusive, zone
        let cases = [
            (
                "bar",
                Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                reserve,
                zone(Edge::Top),
            ),
            (
                "dock",
                Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
                reserve,
                zone(Edge::Bottom),
            ),
            (
                "side panel",
                Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM,
                reserve,
                zone(Edge::Left),
            ),
            ("lone edge", Anchor::RIGHT, reserve, zone(Edge::Right)),
            (
                "a corner is ambiguous",
                Anchor::TOP | Anchor::LEFT,
                reserve,
                None,
            ),
            (
                "opposite edges",
                Anchor::TOP | Anchor::BOTTOM,
                reserve,
                None,
            ),
            ("no anchor", Anchor::NONE, reserve, None),
            (
                "hidden dock",
                Anchor::BOTTOM,
                ExclusiveZone::Reserve(0),
                None,
            ),
            ("auto", Anchor::TOP, ExclusiveZone::Auto, None),
            ("ignore", Anchor::TOP, ExclusiveZone::Ignore, None),
        ];
        for (name, anchor, exclusive, want) in cases {
            assert_eq!(zone_of(&config(anchor, exclusive)), want, "{name}");
        }
    }
}
