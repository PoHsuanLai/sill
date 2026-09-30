//! One dock tile: quire's `DockTile` at the tile's magnified size and bounce lift, in the slot the
//! frame gives it. `DockTile` draws the plated icon (a quire `IconView` plate; under Muted or
//! Monochrome its two stops and its glyph ink are re-coloured through `PlateTint::of(style,
//! tint)`, `None` under Colour), its `Badge` (a count, `999+` past 999), the `ProgressIndicator`
//! bar while the app works, the running dot and the hover label (`DockLabel`), and lifts the
//! plate by a bounce while the dot stays on the baseline. The label is shown and hidden by the
//! dock machine alone (`shown`): up from its `ShowLabel` (the fly delay, the warm skip) to its
//! `HideLabel` (the pointer left, a button went down, a context menu opened). The dock provides
//! the machine's shown label as context; the frozen `Tile` props are unchanged. The tile takes
//! no press of its own: the dock's input layer reads every pointer event.

use dioxus::prelude::*;
use ds::components::controls::badge::BadgeContent;
use ds::prelude::*;
use ds_core::vocab::Activity;
use ds_shell::prelude::*;
use ds_style::icon::family::PlateFamily;
use ds_style::icon::plate_tint::PlateTint;
use ds_style::scale::use_scale;
use sill_model::badges::Badge;
use sill_model::units::{Cpx, Permille};
use sill_settings::{DockSettings, Indicators};

use crate::dock::frame::Rows;
use crate::dock::icon::{IconContext, TileSizes, drawn, tile_art};
use crate::dock::items::{DockItem, ItemKind};
use crate::dock::machine::{TileApp, TileId};
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
    let look = try_use_context::<Signal<TileLook>>().map_or(TileLook::default(), |look| look());
    let cx = IconContext {
        look: look.0,
        scale: use_scale(),
        resolver: use_icon_resolver(),
    };
    let icon = drawn(tile_art(&item.kind, app), sizes, &cx);
    let rows = Rows::new(&dock);
    let top = px(rows.baseline) - side;
    let running = if item.app != TileApp::NotRunning && dock.indicators == Indicators::On {
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
                plate: Some(PlateFamily::Neutral),
                plate_tint: PlateTint::of(look.0.style, look.0.tint),
                badge: badge.and_then(|b| b.shown_count()).map(count),
                progress: badge.and_then(|b| b.shown_progress()).map(fraction),
                running,
                side: Px(side as f32),
                lift: Px(px(lift) as f32),
                label: name,
                label_shown: Some(shown),
                onclick: |_| {},
            }
        }
    }
}

/// `v` in logical pixels.
fn px(v: Cpx) -> f64 {
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
