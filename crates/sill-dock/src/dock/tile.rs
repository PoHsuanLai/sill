//! One dock tile: quire's `DockTile` at the tile's magnified size, in the slot the frame gives
//! it. `DockTile` draws the app's icon as it is, on no plate (under Muted or
//! Monochrome the icon cache re-colours its raster), its `Badge` (a count, `999+` past 999), the `ProgressIndicator`
//! bar while the app works, the running dot that says the app is open and the hover label
//! (`DockLabel`); an opening app does not bounce (the owner, 2026-10-20). The label is shown and hidden by the
//! dock machine alone (`shown`): up from its `ShowLabel` (the fly delay, the warm skip) to its
//! `HideLabel` (the pointer left, a button went down, a context menu opened). The dock provides
//! the machine's shown label as context; the frozen `Tile` props are unchanged. The tile takes
//! no press of its own: the dock's input layer reads every pointer event.

use dioxus::prelude::*;
use ds::base::vocab::Activity;
use ds::components::controls::badge::BadgeContent;
use ds::prelude::*;
use ds::style::scale::use_scale;
use sill_model::badges::Badge;
use sill_model::units::{Cpx, Permille};
use sill_settings::OnOff;
use sill_shell_kit::dock::tile::plate_side;
use sill_shell_kit::prelude::*;

use crate::dock::frame::Rows;
use crate::dock::icon::{IconContext, TileSizes, drawn, tile_art};
use crate::dock::items::{DockItem, ItemKind};
use crate::dock::machine::{TileApp, TileId};
use crate::dock::tokens::DockTokens;
use sill_services::AppsService;
use sill_ui_kit::app_icons::{IconLook, use_icon_resolver};
use sill_ui_kit::hooks::{use_section, use_service};

/// The tile whose hover label the dock machine shows, if any: the context the dock provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelShown(pub Option<TileId>);

/// The icon style the dock draws its tiles in: the context the dock provides (Colour when a
/// tile renders without one).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TileLook(pub IconLook);

/// The gap between the label and the top of the plate (design/10 §10.3.4).
const LABEL_GAP: u32 = 7;

/// A tile at `left` with side `size`.
#[component]
pub fn Tile(item: DockItem, left: Cpx, size: Cpx, badge: Option<Badge>) -> Element {
    let apps = use_service::<AppsService>();
    let dock = use_section(|file| &file.dock)();
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
    let look = try_use_context::<Signal<TileLook>>().map_or_else(TileLook::default, |look| look());
    let cx = IconContext {
        look: look.0,
        scale: use_scale(),
        resolver: use_icon_resolver(),
    };
    let icon = drawn(tile_art(&item.kind, app), sizes, &cx);
    let tokens =
        try_use_context::<Signal<DockTokens>>().map_or_else(DockTokens::default, |tokens| tokens());
    let rows = Rows::new(&dock, &tokens);
    let top = px(rows.baseline) - side;
    // How far the plate's top is below the tile's: the label stands above it.
    let rise = (side - f64::from(plate_side(Px(side as f32)).0)) / 2.0;
    let running = if item.app != TileApp::NotRunning && dock.indicators == OnOff::On {
        Activity::Active
    } else {
        Activity::Inactive
    };
    let label = try_use_context::<Signal<LabelShown>>().map_or(LabelShown(None), |shown| shown());
    let shown = if label.0 == Some(item.tile) {
        Shown::Visible
    } else {
        Shown::Hidden
    };
    rsx! {
        div {
            class: "sill-dock-tile",
            style: "left:{px(left)}px;top:{top}px;width:{side}px;height:{side}px",
            DockTile {
                icon,
                // The app's own icon as it is, on no plate (the owner, 2026-10-20).
                plate: None,
                badge: badge.and_then(|b| b.shown_count()).map(count),
                progress: badge.and_then(|b| b.shown_progress()).map(fraction),
                running,
                side: Px(side as f32),
                label: name.clone(),
                // Never quire's own: its card is placed in the pill-sized root's coordinates and
                // lands the pill's origin away from its tile. The label is drawn below, in the
                // tile's own box, so it follows the tile every frame.
                label_shown: Some(Shown::Hidden),
                onclick: |_| {},
            }
            if shown == Shown::Visible {
                span {
                    class: "sill-dock-label",
                    style: "bottom:calc(100% - {rise}px + {LABEL_GAP}px)",
                    span { class: "sill-dock-label-text", "{name}" }
                }
            }
        }
    }
}

/// `v` in logical pixels.
pub(super) fn px(v: Cpx) -> f64 {
    f64::from(v.0) / 100.0
}

/// A count as quire's `Badge` takes it (it says `999+` past 999 itself).
fn count(count: u64) -> BadgeContent {
    BadgeContent::Number(u32::try_from(count).unwrap_or(u32::MAX))
}

/// A progress as quire's `ProgressIndicator` takes it: a share in thousandths, at most all.
fn fraction(progress: Permille) -> Fraction {
    Fraction(u16::try_from(progress.0.min(1000)).unwrap_or(1000))
}
