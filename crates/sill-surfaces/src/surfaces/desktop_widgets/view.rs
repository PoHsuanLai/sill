//! The layer's root: a transparent root in the Widget material as large as the layer
//! (`RootExtent::Viewport`), holding the grid. The grid measures itself once laid out, places
//! `widgets.desktop_widgets` with `place` over `grid_size` (saved places first, from the
//! [`PlacementFile`] when the root was given one), writes the placements back when they changed,
//! draws each widget at its box, and keeps the surface's input and blur regions on the frames
//! (`RegionSpec::Elements`), so a press between them reaches the desktop below. Nothing here
//! runs a timer: the widgets redraw on their services' changes only.

use dioxus::prelude::*;
use ds::{BlurState, Ds, Material, RootChrome, RootExtent, use_rect};
use shell_host::{BlurSpec, ElementId, ElementRegion, OutputInfo, RegionSpec, SurfaceHandle};
use sill_services::OutputName;
use sill_settings::WidgetKind;

use super::DESKTOP_WIDGETS_CSS;
use super::geometry::{GridMetrics, place_box};
use super::placement::{Placement, grid_size, place};
use super::saved::PlacementFile;
use crate::hooks::{use_appearance, use_ds_scale, use_settings, use_workspaces};
use crate::surfaces::launcher::panel::active_look;
use crate::widgets::{Widget, WidgetHost, size_of, widget_element_id, widgets_css};

/// The Widget material's corner (quire's recipe, `--m-radius` 20 px): the frames' regions round
/// their corners by it.
pub const WIDGET_RADIUS: i32 = 20;

/// The layer on `output`.
pub(super) fn desktop(output: OutputInfo) -> Element {
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
    let widgets = file.widgets;
    rsx! {
        Ds {
            scale: ds_scale,
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            look,
            material: Material::Widget,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            blur: BlurState::Available,
            tint_alpha: Some(environment.tint_alpha()),
            stack: Some(environment.material_stack()),
            style { {DESKTOP_WIDGETS_CSS} }
            style { {widgets_css()} }
            Grid {
                kinds: widgets.desktop_widgets,
                cell: widgets.desktop_cell_px.0,
                gap: widgets.desktop_gap_px.0,
            }
        }
    }
}

/// The widgets on their grid, once the layer's size is known.
#[component]
fn Grid(kinds: Vec<WidgetKind>, cell: u16, gap: u16) -> Element {
    let probe = use_rect();
    let file = try_use_context::<PlacementFile>();
    let mut kept =
        use_hook(|| CopyValue::new(file.as_ref().map(PlacementFile::load).unwrap_or_default()));
    let laid = probe.rect().map(|rect| {
        let width = logical(rect.size.width.0);
        let grid = grid_size(width, logical(rect.size.height.0), cell, gap);
        let sized: Vec<_> = kinds
            .iter()
            .map(|kind| (*kind, size_of(*kind, WidgetHost::Desktop)))
            .collect();
        let placements = place(&sized, &kept.peek(), grid);
        let metrics = GridMetrics {
            grid,
            layer_width: width,
            cell,
            gap,
        };
        (placements, metrics)
    });
    let placements = laid.as_ref().map(|(p, _)| p.clone()).unwrap_or_default();
    let merged = merged(&placements, &kept.peek());
    if laid.is_some() && merged != *kept.peek() {
        if let Some(file) = &file
            && let Err(error) = file.save(&merged)
        {
            eprintln!("sill: saving {}: {error}", file.0.display());
        }
        kept.set(merged);
    }
    use_regions(&placements);
    let boxes: Vec<_> = laid
        .map(|(placements, metrics)| {
            placements
                .into_iter()
                .map(|p| {
                    let size = size_of(p.kind, WidgetHost::Desktop);
                    (p.kind, place_box(p.slot, size, metrics).style_attr())
                })
                .collect()
        })
        .unwrap_or_default();
    rsx! {
        div { class: "sill-dw-grid", onmounted: move |event| probe.on_mounted(event),
            for (kind, style) in boxes {
                div { key: "{kind:?}", class: "sill-dw-place", style,
                    Widget { kind, host: WidgetHost::Desktop }
                }
            }
        }
    }
}

/// A measured length in whole logical px.
fn logical(length: f32) -> u32 {
    // Clamped into u32's range first, so the cast only drops the fraction.
    length.round().clamp(0.0, 1.0e9) as u32
}

/// The file's placements: this layer's, then every saved one of a kind it does not show (a
/// widget taken off the desktop keeps its place for when it comes back).
fn merged(placed: &[Placement], saved: &[Placement]) -> Vec<Placement> {
    let others = saved
        .iter()
        .filter(|s| placed.iter().all(|p| p.kind != s.kind));
    placed.iter().chain(others).copied().collect()
}

/// The input and blur regions `placements` take: each widget's frame.
pub fn widget_regions(placements: &[Placement]) -> (RegionSpec, BlurSpec) {
    if placements.is_empty() {
        return (RegionSpec::Empty, BlurSpec::None);
    }
    let regions: Vec<ElementRegion> = placements
        .iter()
        .map(|p| ElementRegion {
            id: ElementId::new(widget_element_id(p.kind)),
            radius: WIDGET_RADIUS,
            inflate: 0,
        })
        .collect();
    (
        RegionSpec::Elements(regions.clone()),
        BlurSpec::Region(RegionSpec::Elements(regions)),
    )
}

/// Keep the surface's regions on the placed frames, sending them only when they change. Nothing
/// to do without a host (a test, an SSR render).
fn use_regions(placements: &[Placement]) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut last = use_hook(|| CopyValue::new(None::<Vec<Placement>>));
    if last.peek().as_deref() == Some(placements) {
        return;
    }
    if let Some(surface) = surface {
        let (input, blur) = widget_regions(placements);
        surface.set_input_region(input);
        surface.set_blur(blur);
    }
    last.set(Some(placements.to_vec()));
}

#[cfg(test)]
mod tests {
    use shell_host::{BlurSpec, RegionSpec};
    use sill_settings::WidgetKind;

    use super::{merged, widget_regions};
    use crate::surfaces::desktop_widgets::placement::{GridSlot, Placement};

    fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
        Placement {
            kind,
            slot: GridSlot { column, row },
        }
    }

    #[test]
    fn a_widget_taken_off_keeps_its_saved_place() {
        let placed = [at(WidgetKind::Battery, 9, 0)];
        let saved = [at(WidgetKind::Battery, 8, 0), at(WidgetKind::UpNext, 1, 1)];
        assert_eq!(
            merged(&placed, &saved),
            vec![at(WidgetKind::Battery, 9, 0), at(WidgetKind::UpNext, 1, 1)]
        );
        assert_eq!(merged(&[], &[]), Vec::new());
    }

    #[test]
    fn the_regions_are_the_frames_or_nothing() {
        assert_eq!(widget_regions(&[]), (RegionSpec::Empty, BlurSpec::None));
        let (input, blur) = widget_regions(&[
            at(WidgetKind::Calendar, 0, 0),
            at(WidgetKind::Battery, 1, 0),
        ]);
        let RegionSpec::Elements(regions) = &input else {
            panic!("element regions: {input:?}");
        };
        let ids: Vec<_> = regions.iter().map(|r| r.id.clone()).collect();
        assert_eq!(
            ids,
            vec![
                shell_host::ElementId::new("widget-calendar"),
                shell_host::ElementId::new("widget-battery"),
            ]
        );
        assert_eq!(blur, BlurSpec::Region(input.clone()));
    }
}
