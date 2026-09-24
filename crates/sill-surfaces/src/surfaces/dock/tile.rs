//! One dock tile: the plated icon at its magnified size and bounce lift, the running dot, the
//! count badge (`Count`, `999+`), the progress ring, the pressed scrim.
//!
//! The tile's box is its slot at the current size, standing on the baseline and lifted by the
//! bounce; the plate is 80.5 % of it (design/08 §2.2) with the icon inside at the design inset
//! (§4.1). The badge and the ring hang off the plate and scale with it (design/10 §10.3.2);
//! the running dot stays on the baseline while the tile bounces. The hover label is a ds
//! `Tooltip{Fly}` on the plate, whose own `:hover` rule and `--d-fly` delay show it (FINDINGS
//! "M3 dock-ui"). No pressed scrim: the frozen props carry no press.

use dioxus::prelude::*;
use ds::{IconView, Tooltip, TooltipKind};
use sill_services::{Badge, Cpx, Permille};
use sill_settings::{DockSettings, Indicators, ProgressStyle};

use crate::hooks::{use_apps, use_settings};
use crate::surfaces::dock::frame::Rows;
use crate::surfaces::dock::icon::{PLATE_PERMILLE, icon_scale, tile_icon};
use crate::surfaces::dock::items::{DockItem, ItemKind};
use crate::surfaces::dock::machine::TileApp;

/// A tile at `left` with side `size`, lifted `lift` by a bounce.
#[component]
pub fn Tile(item: DockItem, left: Cpx, size: Cpx, lift: Cpx, badge: Option<Badge>) -> Element {
    let apps = use_apps();
    let dock = use_settings()().dock;
    let index = apps();
    let app = item.kind.app_id().and_then(|id| index.get(id));
    let name = match (&item.kind, app) {
        (_, Some(app)) => app.name.clone(),
        (ItemKind::Pinned(id) | ItemKind::Running(id), None) => id.0.clone(),
        (ItemKind::Unmatched(raw), None) => raw.clone(),
        (ItemKind::Trash, None) => "Trash".to_owned(),
    };
    let source = tile_icon(&item.kind, app);
    let side = px(size);
    let plate = side * f64::from(PLATE_PERMILLE) / 1000.0;
    let scale = icon_scale(&source, plate);
    let rows = Rows::new(&dock);
    let top = px(rows.baseline) - side - px(lift);
    let running = item.app != TileApp::NotRunning && dock.indicators == Indicators::On;
    let grow = side / f64::from(dock.tile_size_px.0.max(1));
    let count = badge.and_then(|b| b.shown_count());
    let progress = badge.and_then(|b| b.shown_progress());
    rsx! {
        div {
            class: "sill-dock-tile",
            style: "left:{px(left)}px;top:{top}px;width:{side}px;height:{side}px",
            Tooltip { kind: TooltipKind::Fly, text: name,
                div {
                    class: "sill-dock-plate",
                    style: plate_style(plate),
                    div {
                        class: "sill-dock-icon",
                        style: "transform:scale({scale})",
                        IconView { source, size: crate::surfaces::dock::icon::DRAWN }
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
            }
            if running {
                span {
                    class: "sill-dock-dot",
                    style: dot_style(&dock, side, px(lift)),
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

/// The plate: its side and the squircle's corner (Apple's 22.37 % of the side).
fn plate_style(plate: f64) -> String {
    format!(
        "width:{plate:.2}px;height:{plate:.2}px;border-radius:{:.2}px",
        plate * 0.2237
    )
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
         background-image:conic-gradient(var(--accent) 0turn {turn:.3}turn, var(--line) {turn:.3}turn 1turn)",
        dock.progress_ring_stroke_px.0
    )
}

/// The running dot: centred under the tile, its centre 4 px below the baseline, whatever the
/// bounce.
fn dot_style(dock: &DockSettings, side: f64, lift: f64) -> String {
    let d = f64::from(dock.running_dot_diameter_px.0);
    format!(
        "width:{d}px;height:{d}px;left:{:.2}px;top:{:.2}px",
        (side - d) / 2.0,
        side + lift + 4.0 - d / 2.0
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
