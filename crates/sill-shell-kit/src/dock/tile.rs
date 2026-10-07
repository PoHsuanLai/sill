//! DockTile: one Dock tile (design/30 section 2.10, design/08-ICONS.md section 2.2,
//! design/10-BEHAVIOUR-dock.md section 10.3.2): the app's plated icon, its `Badge`, a
//! `ProgressIndicator { Bar }` while it works, the running dot under it, and the `DockLabel` with
//! its name. Magnification and the bounce are the shell's: it hands the tile its current `side`
//! and sets its place; the tile draws what it is at that size.
//!
//! Markup: `button.ds-dock-tile[data-running]` holding `span.ds-dock-plate` (the `IconView` plate,
//! the badge and the bar) and the running dot. The plate is 80.5 % of the tile's side
//! (design/08 section 2.2); the badge hangs off its top right corner and the bar sits inside its
//! foot, both on the plate so they scale with it. The running dot sits in the tile's own box, so
//! it stays on the baseline while the plate bounces. `label` names the tile for assistive
//! technology and draws over the plate as a `DockLabel` while `label_shown` says so (`None`
//! follows the pointer). `lift` raises the plate (and its badge, bar and label) by a bounce,
//! whole in layout so the label follows; the dot stays where it is.

use crate::dock::parts::{DockLabel, RunningDot};
use dioxus::prelude::*;
use ds::base::geometry::units::Px;
use ds::base::press::Press;
use ds::base::vocab::{Activity, Fraction, Shown};
use ds::base::word::Word;
use ds::components::content::icon_source::IconSource;
use ds::components::content::icon_view::IconView;
use ds::components::controls::badge::{Badge, BadgeContent, BadgeTone};
use ds::components::controls::press::{ActivationKeys, PressListeners};
use ds::components::controls::progress::model::{Progress, ProgressStyle};
use ds::components::controls::progress::view::ProgressIndicator;
use ds::root::common::Common;
use ds::style::icon::family::PlateFamily;
use ds::style::icon::plate_tint::PlateTint;
use ds::style::icon::render::{IconPx, IconSize};
use ds::style::tokens::control_size::ControlSize;

/// The plate's share of the tile's side (design/08 section 2.2).
const PLATE_SHARE: f32 = 0.805;

/// The tile at rest, `dock.tile_size_px`'s default.
const REST_SIDE: Px = Px(48.0);

/// The plate's side for a tile of `side`: whole pixels, at least one.
pub fn plate_side(side: Px) -> IconPx {
    IconPx((side.0 * PLATE_SHARE).round().clamp(1.0, 255.0) as u8)
}

/// A dock tile. `icon` is drawn on `plate` (`None` for an app whose own picture is its plate),
/// re-coloured by `plate_tint` under a Muted or Monochrome dock. `badge` is the app's count or
/// dot; `progress` its work under way, as a share; `running` is `Active` while the app has a
/// window, which draws the dot. `side` is the tile's current side, `lift` how far a bounce has
/// raised the plate.
#[component]
pub fn DockTile(
    icon: IconSource,
    #[props(default)] plate: Option<PlateFamily>,
    #[props(default)] plate_tint: Option<PlateTint>,
    #[props(default)] badge: Option<BadgeContent>,
    #[props(default)] progress: Option<Fraction>,
    #[props(default)] running: Activity,
    #[props(default = REST_SIDE)] side: Px,
    #[props(default)] lift: Px,
    #[props(into)] label: String,
    #[props(default)] label_shown: Option<Shown>,
    onclick: EventHandler<Press>,
    #[props(default)] common: Common,
) -> Element {
    let class = common.class("ds-dock-tile");
    let data = common.data_attributes();
    let listen = PressListeners::new(onclick);
    let plate_px = plate_side(side);
    let inset = (side.0 - f32::from(plate_px.0)) / 2.0;
    let name = common.aria_label.clone().unwrap_or_else(|| label.clone());
    rsx! {
        button {
            r#type: "button",
            class,
            id: common.id.clone(),
            "data-running": running.slug(),
            "aria-label": "{name}",
            style: "width:{side.0}px;height:{side.0}px",
            onclick: move |event| listen.click(&event),
            oncontextmenu: move |event| listen.context_menu(&event),
            onkeydown: move |event| listen.key_down(&event, ActivationKeys::ReturnAndSpace),
            onmounted: move |event| common.mounted(event),
            ..data,
            span {
                class: "ds-dock-plate",
                style: "left:{inset}px;top:{inset - lift.0}px;width:{plate_px.0}px;height:{plate_px.0}px",
                DockLabel { text: label, shown: label_shown,
                    IconView {
                        source: icon,
                        size: IconSize::Px(plate_px),
                        plate,
                        plate_tint,
                    }
                }
                if let Some(content) = badge {
                    span { class: "ds-dock-badge",
                        Badge { content, tone: BadgeTone::Alert, size: ControlSize::Small }
                    }
                }
                if let Some(share) = progress {
                    span { class: "ds-dock-progress",
                        ProgressIndicator {
                            style: ProgressStyle::Bar,
                            progress: Progress::Known(share),
                            size: ControlSize::Mini,
                        }
                    }
                }
            }
            if running == Activity::Active {
                RunningDot {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::plate_side;
    use ds::base::geometry::units::Px;
    use ds::style::icon::render::IconPx;

    #[test]
    fn the_plate_is_eighty_and_a_half_percent_of_the_tile_in_whole_pixels() {
        const CASES: &[(f32, u8)] = &[(48.0, 39), (96.0, 77), (64.0, 52), (0.0, 1)];
        for &(side, plate) in CASES {
            assert_eq!(plate_side(Px(side)), IconPx(plate), "{side}");
        }
    }
}
