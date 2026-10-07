//! The layer's root: a transparent root in the Widget material as large as the layer
//! (`RootExtent::Viewport`), holding the grid. The grid measures itself once laid out, places
//! this output's widgets (its own list in the [`PlacementFile`] the surface is given, else
//! `widgets.desktop_widgets`; each at its one desktop size) with `place` over `grid_size`
//! (this output's saved places first), both read again when Edit Widgets writes, draws each
//! widget at its box, lets a widget be dragged to another cell (`drag`, `placed`; quire's
//! `WidgetSlotGuide` at the cell it would land in), opens the desktop's menu at a press of the
//! secondary button (`menu`), and keeps the surface's blur region on the frames
//! (`RegionSpec::Elements`, resolved after every layout, so it follows a dragged frame). A widget
//! taken away keeps its card at its last box while it plays its exit, outside the region
//! (`leaving`).
//! Nothing here runs a timer: the widgets redraw on their services' changes, a drag's motion runs
//! only while it moves, and an exit's end is quire's card's to call.

use dioxus::prelude::*;
use ds::host::measure::use_rect;
use ds::motion::wake::WakeStamp;
use ds::prelude::*;
use shell_host::{OutputInfo, use_watch};
use sill_settings::WidgetKind;
use sill_shell_kit::prelude::*;
use sill_shell_kit::tokens::widgets::WidgetMetrics;

use super::drag::{DragInput, Hold, shown};
use super::geometry::GridMetrics;
use super::glide::PointMeter;
use super::io::{Feeder, save, secondary_press, use_regions, use_reload};
use super::leaving::use_leaving;
use super::menu::{MenuWorld, use_desktop_menu};
use super::model::{Draw, Dropped, Laid, boxes, draw_order, logical, recorded, slot_guide};
use super::placed::Placed;
use super::placement::{grid_size, place};
use sill_ui_kit::hooks::{use_services, use_settings};
use sill_ui_kit::links::{DesktopView, EditWidgetsLink, PlacementFile, Written};
use sill_ui_kit::menu::use_popup_look;
use sill_ui_kit::root::SurfaceRoot;
use sill_ui_kit::widgets::grid::OutputKey;
use sill_ui_kit::widgets::{
    WidgetHost, desktop_size, provide_registry, provide_widget_wake, widgets_css,
};

/// The layer's own stylesheet (the grid), injected beside `widgets_css`.
pub const DESKTOP_WIDGETS_CSS: &str = include_str!("style.css");

/// The layer on `output`: its own widget wake (`sill_ui_kit::widgets::WidgetWake`), bumped once when
/// this layer is mapped (`widgets.desktop` turning `On` spawns a fresh one), so the batteries' rings and figures count up as they come into
/// view.
pub(super) fn desktop(output: OutputInfo) -> Element {
    provide_widget_wake(WakeStamp::default());
    provide_registry();
    let file = use_settings()();
    let widgets = file.widgets;
    let written = use_written();
    let key = OutputKey::of(output.name.as_deref());
    let kinds = try_use_context::<PlacementFile>()
        .and_then(|places| places.kinds(&key))
        .unwrap_or(widgets.desktop_widgets);
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Widget,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            AppStyle { css: DESKTOP_WIDGETS_CSS }
            AppStyle { css: widgets_css() }
            Grid {
                kinds,
                written,
                cell: widgets.desktop_cell_px.0,
                gap: widgets.desktop_gap_px.0,
                threshold: file.window.move_threshold_px.0,
                output,
            }
        }
    }
}

/// The widgets on their grid, once the layer's size is known.
#[component]
fn Grid(
    kinds: Vec<WidgetKind>,
    written: Written,
    cell: u16,
    gap: u16,
    threshold: u16,
    output: OutputInfo,
) -> Element {
    let probe = use_rect();
    let file = try_use_context::<PlacementFile>();
    let key = OutputKey::of(output.name.as_deref());
    let mut kept =
        use_hook(|| CopyValue::new(file.as_ref().map(|f| f.load(&key)).unwrap_or_default()));
    use_reload((&kinds, written), file.as_ref(), &key, kept);
    let apps = use_watch(use_services().apps.state.clone());
    let world = MenuWorld::of(
        &apps.read(),
        DesktopView {
            cell: sill_settings::Px(cell),
            gap: sill_settings::Px(gap),
        },
    );
    let menu = use_desktop_menu(&output, world, use_popup_look());
    let hold = use_signal(Hold::default);
    let dropped = use_signal(|| None::<Dropped>);
    let meter = use_hook(|| CopyValue::new(PointMeter::default()));
    let mut laid = use_hook(|| CopyValue::new(None::<Laid>));
    let cards = use_leaving(&kinds);
    let now = probe.rect().map(|rect| {
        let width = logical(rect.size.width.0);
        let grid = grid_size(width, logical(rect.size.height.0), cell, gap);
        let sized: Vec<_> = kinds
            .iter()
            .map(|kind| (*kind, desktop_size(*kind)))
            .collect();
        let metrics = GridMetrics {
            grid,
            layer_width: width,
            cell,
            gap,
        };
        let rest = place(&sized, &kept.peek(), grid);
        Laid {
            boxes: boxes(&rest, metrics),
            rest,
            metrics,
            threshold: u32::from(threshold),
        }
    });
    if let Some(now) = &now {
        let recorded = recorded(&now.rest, &kept.peek());
        if recorded != *kept.peek() {
            save(file.as_ref(), &key, &recorded);
            kept.set(recorded);
        }
    }
    // Where each card stood before this layout: a kind just taken away leaves from there.
    let last_boxes = laid.peek().as_ref().map(|l| l.boxes.clone());
    let leaving = cards.now(&kinds, &last_boxes.unwrap_or_default());
    laid.set(now.clone());
    let feed = Feeder {
        hold,
        dropped,
        meter,
        laid,
        kept,
        file: file.clone(),
        key: key.clone(),
    };
    let held = hold();
    let on_screen = now
        .as_ref()
        .map(|l| shown(&held, &l.rest, l.metrics))
        .unwrap_or_default();
    use_regions(&on_screen);
    let guide = now.as_ref().and_then(|l| slot_guide(&held, l));
    let places = now
        .map(|l| draw_order(&leaving, &on_screen, &held, dropped(), l.metrics))
        .unwrap_or_default();
    let (down, moved, up) = (feed.clone(), feed.clone(), feed);
    rsx! {
        div {
            class: "sill-dw-grid",
            style: grid_style(cell, gap),
            onmounted: move |event| probe.on_mounted(event),
            onmousedown: move |event| {
                secondary_press(&event, menu);
                down.pointer(&event, DragInput::Down);
            },
            onmousemove: move |event| moved.pointer(&event, DragInput::Move),
            onmouseup: move |event| up.pointer(&event, DragInput::Up),
            if let Some((size, spot)) = guide {
                div { class: "sill-dw-guide", style: spot.style_attr(),
                    WidgetSlotGuide { size, host: WidgetHost::Desktop }
                }
            }
            for Draw { kind, target, pose, glide, shown } in places {
                Placed {
                    key: "{kind:?}",
                    kind,
                    target,
                    pose,
                    glide,
                    shown,
                    on_hidden: match shown {
                        Shown::Hidden => Some(cards.on_gone(kind)),
                        Shown::Visible => None,
                    },
                }
            }
        }
    }
}

/// The grid unit as quire's widget tokens (`--widget-cell`, `--widget-gap`), so each
/// `WidgetFrame` is as large as the place the layer gives it.
fn grid_style(cell: u16, gap: u16) -> String {
    WidgetMetrics {
        cell: Px(f32::from(cell)),
        gap: Px(f32::from(gap)),
    }
    .style_attr()
}

/// How many writes the Edit Widgets link has carried out, live: the layer reads its output's
/// widgets from the state file again after each. Without a link (a test render) it never moves.
fn use_written() -> Written {
    let link = try_use_context::<EditWidgetsLink>().unwrap_or_default();
    *use_watch(link.written()).read()
}
