//! One dock tile: the plated icon at its magnified size and bounce lift, the running dot, the
//! count badge (`Count`, `999+`), the progress ring, the pressed scrim.
//!
//! The tile's box is its slot at the current size, standing on the baseline and lifted by the
//! bounce; the plate is 80.5 % of it (design/08 §2.2), a quire `IconView` plate (the squircle
//! with its gradient, highlight, rim and drop; the macOS polish pass) in the neutral paper family
//! (§4.1), the icon inside at the design inset, the fallback glyph at its 56 % (§4.3), in the
//! icon style the dock provides as [`TileLook`] (`icon.rs`, `app_icons`). Under Muted or Monochrome the plate takes
//! the same `(IconStyle, Tint)` pair `app_icons` hands `ds::icon::retint`, through `IconView`'s
//! `plate_tint: PlateTint::of(style, tint)`: the plate's two stops and its
//! glyph ink are re-coloured by quire itself, so a Muted or Monochrome dock is one hue, plate
//! included; `PlateTint::of` is `None` under Colour, which changes nothing. The badge
//! and the ring hang off the plate and scale with it (design/10 §10.3.2); quire's `RunningDot`
//! sits in a box at the tile's rest place, so it stays on the baseline while the tile bounces. The hover label is a ds
//! `Tooltip{Fly}` over the plate, shown and hidden by the dock machine alone (`shown`): up
//! from its `ShowLabel` (the fly delay, the warm skip) to its `HideLabel` (the pointer left, a
//! button went down, a context menu opened). The dock provides the machine's shown label as
//! context; the frozen `Tile` props are unchanged. No pressed scrim: the frozen props carry no
//! press.

use dioxus::prelude::*;
use ds::{IconPx, IconSize, IconView, PlateFamily, PlateTint, Shown, Tooltip, TooltipKind};
use ds_shell::RunningDot;
use sill_model::badges::Badge;
use sill_model::units::{Cpx, Permille};
use sill_settings::{DockSettings, Indicators, ProgressStyle};

use crate::app_icons::{IconLook, use_icon_resolver};
use crate::hooks::{use_section, use_service};
use crate::surfaces::dock::frame::Rows;
use crate::surfaces::dock::icon::{IconContext, TileSizes, drawn, tile_art};
use crate::surfaces::dock::items::{DockItem, ItemKind};
use crate::surfaces::dock::machine::{TileApp, TileId};
use sill_services::AppsService;

/// The tile whose hover label the dock machine shows, if any: the context the dock provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelShown(pub Option<TileId>);

/// The icon style the dock draws its tiles in: the context the dock provides (Colour when a
/// tile renders without one).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TileLook(pub IconLook);

/// A tile at `left` with side `size`, lifted `lift` by a bounce.
#[component]
pub fn Tile(item: DockItem, left: Cpx, size: Cpx, lift: Cpx, badge: Option<Badge>) -> Element {
    let apps = use_service::<AppsService>();
    let dock = use_section::<DockSettings>()();
    let index = apps();
    let app = item.kind.app_id().and_then(|id| index.get(id));
    let name = match (&item.kind, app) {
        (_, Some(app)) => app.name.clone(),
        (ItemKind::Pinned(id) | ItemKind::Running(id), None) => id.0.clone(),
        (ItemKind::Unmatched(raw), None) => raw.clone(),
        (ItemKind::Trash, None) => "Trash".to_owned(),
    };
    let side = px(size);
    let sizes = TileSizes::new(side, &dock);
    let plate = sizes.plate();
    let look = try_use_context::<Signal<TileLook>>().map_or(TileLook::default(), |look| look());
    let cx = IconContext {
        look: look.0,
        scale: ds::use_scale(),
        resolver: use_icon_resolver(),
    };
    let source = drawn(tile_art(&item.kind, app), sizes, &cx);
    let rows = Rows::new(&dock);
    let top = px(rows.baseline) - side - px(lift);
    let running = item.app != TileApp::NotRunning && dock.indicators == Indicators::On;
    let grow = side / f64::from(dock.tile_size_px.0.max(1));
    let count = badge.and_then(|b| b.shown_count());
    let progress = badge.and_then(|b| b.shown_progress());
    let label = try_use_context::<Signal<LabelShown>>().map_or(LabelShown(None), |shown| shown());
    let shown = if label.0 == Some(item.tile) {
        Shown::Visible
    } else {
        Shown::Hidden
    };
    let inset = (side - plate) / 2.0;
    rsx! {
        div {
            class: "sill-dock-tile",
            style: "left:{px(left)}px;top:{top}px;width:{side}px;height:{side}px",
            div {
                class: "sill-dock-plate",
                style: plate_style(plate),
                IconView {
                    source,
                    size: plate_size(plate),
                    plate: Some(PlateFamily::Neutral),
                    plate_tint: PlateTint::of(look.0.style, look.0.tint),
                }
                if let Some(count) = count {
                    span {
                        class: "sill-dock-badge",
                        style: badge_style(&dock, grow),
                        {count_text(count)}
                    }
                }
                if let (Some(progress), ProgressStyle::Ring) = (progress, dock.progress_style) {
                    span {
                        class: "sill-dock-ring",
                        style: ring_style(&dock, grow, progress),
                        span { class: "sill-dock-ring-hole" }
                    }
                }
            }
            div {
                class: "sill-dock-label",
                style: "left:{inset:.2}px;top:{inset:.2}px",
                Tooltip { kind: TooltipKind::Fly, text: name, shown,
                    span {
                        class: "sill-dock-label-box",
                        style: "width:{plate:.2}px;height:{plate:.2}px",
                    }
                }
            }
            if running {
                span {
                    class: "sill-dock-dot",
                    style: "top:{px(lift):.2}px;height:{side:.2}px",
                    RunningDot {}
                }
            }
        }
    }
}

/// `v` in logical pixels.
fn px(v: Cpx) -> f64 {
    f64::from(v.0) / 100.0
}

/// A count as the badge shows it (design/10 §10.3.2: `> 999` reads `999+`).
pub fn count_text(count: u64) -> String {
    if count > 999 {
        "999+".to_owned()
    } else {
        count.to_string()
    }
}

/// The plate's box, which the badge and the ring hang from.
fn plate_style(plate: f64) -> String {
    format!("width:{plate:.2}px;height:{plate:.2}px")
}

/// The quire plate's side in whole pixels (at least one).
fn plate_size(plate: f64) -> IconSize {
    // Rounded and clamped to u8's range, so the cast is exact.
    IconSize::Px(IconPx(plate.round().clamp(1.0, 255.0) as u8))
}

/// The badge: `dock.badge_size_px` tall, `dock.badge_radius_px` round, scaled with the tile.
fn badge_style(dock: &DockSettings, grow: f64) -> String {
    let h = f64::from(dock.badge_size_px.0);
    format!(
        "height:{h}px;min-width:{h}px;border-radius:{}px;transform:scale({grow:.4})",
        dock.badge_radius_px.0
    )
}

/// The ring: `dock.progress_ring_diameter_px` wide, an arc of `progress` from 12 o'clock.
fn ring_style(dock: &DockSettings, grow: f64, progress: Permille) -> String {
    let d = f64::from(dock.progress_ring_diameter_px.0);
    let turn = f64::from(progress.0.min(1000)) / 1000.0;
    format!(
        "width:{d}px;height:{d}px;padding:{}px;transform:scale({grow:.4});\
         background-image:conic-gradient(var(--accent-text) 0turn {turn:.3}turn, var(--line) {turn:.3}turn 1turn)",
        dock.progress_ring_stroke_px.0
    )
}

#[cfg(test)]
mod tests {
    use super::count_text;

    #[test]
    fn counts_over_999_read_999_plus() {
        const CASES: &[(u64, &str)] = &[(1, "1"), (7, "7"), (999, "999"), (1234, "999+")];
        for &(count, want) in CASES {
            assert_eq!(count_text(count), want, "{count}");
        }
    }
}
