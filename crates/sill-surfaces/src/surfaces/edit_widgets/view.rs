//! Edit Widgets' root: one transparent `Ds` in the Sheet material, as large as the output below
//! the bar (`RootExtent::Viewport`), holding the catcher (a press on it closes) and quire's
//! `Panel` at the right edge, where the notification center it was opened from stood, wide
//! enough for the gallery's largest preview. In the panel: the title and Done, then quire's
//! `WidgetGallery` over the layout the keys make (`widgets::layout::layout_of`), each edit
//! applied with `widgets::layout::edited` against this output's desktop grid (measured from the
//! catcher, the same box the desktop widget layer measures) and written through the root's
//! [`WidgetsWriter`]. An edit sill cannot make says why under the gallery.

use std::rc::Rc;

use dioxus::prelude::*;
use ds::widget::{DesktopGrid, WidgetEdit};
use ds::{
    BlurState, Button, ButtonVariant, Ds, Material, Panel, PanelEdge, PanelScrim, Px, RootExtent,
    Shown, WidgetGallery, use_rect,
};
use shell_host::OutputInfo;
use sill_services::OutputName;

use super::drive::{places_to_save, use_phase};
use super::layer::EDIT_PANEL_ID;
use super::link::EditWidgetsLink;
use super::writer::{WidgetsWrite, WidgetsWriter};
use crate::hooks::{use_appearance, use_ds_scale, use_settings, use_workspaces};
use crate::surfaces::desktop_widgets::{OutputKey, PlacementFile, grid_size, place};
use crate::surfaces::launcher::panel::active_look;
use crate::surfaces::notification_center::phase::Phase;
use crate::widgets::layout::{WidgetKeys, edited, layout_of};
use crate::widgets::sizes::Sizes;
use crate::widgets::{WidgetHost, provide_registry, widgets_css};

/// Edit Widgets' stylesheet (lint-clean under `Profile::Strict`, `tests/edit_widgets_lint.rs`).
pub const EDIT_WIDGETS_CSS: &str = include_str!("../../style/edit_widgets.css");

/// The panel's width: the gallery's list (200), a Large preview (344) and their gaps and
/// padding, so every size's preview is drawn at its real size.
const EDIT_WIDTH: f32 = 720.0;

/// Edit Widgets on `output`.
pub(super) fn edit(output: OutputInfo) -> Element {
    provide_registry();
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
    rsx! {
        Ds {
            scale: ds_scale,
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            look,
            material: Material::Sheet,
            extent: RootExtent::Viewport,
            blur: BlurState::Available,
            tint_alpha: Some(environment.tint_alpha()),
            stack: Some(environment.material_stack()),
            style { {EDIT_WIDGETS_CSS} }
            style { {widgets_css()} }
            Stage { output }
        }
    }
}

/// The panel while it is drawn on `output`: each opening its own.
#[component]
fn Stage(output: OutputInfo) -> Element {
    let phase = use_phase(output.id);
    let now = phase.now();
    let shown = match now {
        Phase::Up(_) => Shown::Visible,
        Phase::Hidden(_) | Phase::Leaving(_) => Shown::Hidden,
    };
    let Some(opening) = now.drawn() else {
        return rsx! {};
    };
    let on_hidden = EventHandler::new(move |()| phase.settled());
    rsx! {
        Opened { key: "{opening.0}", output, shown, on_hidden }
    }
}

/// One opening: the catcher, the panel and the gallery.
#[component]
fn Opened(output: OutputInfo, shown: Shown, on_hidden: EventHandler<()>) -> Element {
    let link = use_context::<EditWidgetsLink>();
    let writer = try_use_context::<WidgetsWriter>();
    let file = try_use_context::<PlacementFile>();
    let probe = use_rect();
    let widgets = use_settings()().widgets;
    let mut refused = use_signal(|| None::<&'static str>);
    let grid = probe.rect().map(|rect| {
        let whole = |length: f32| length.round().clamp(0.0, 1.0e9) as u32;
        let size = grid_size(
            whole(rect.size.width.0),
            whole(rect.size.height.0),
            widgets.desktop_cell_px.0,
            widgets.desktop_gap_px.0,
        );
        DesktopGrid {
            columns: size.columns,
            rows: size.rows,
        }
    });
    let key = OutputKey::of(output.name.as_deref());
    let saved = file.as_ref().map(|f| f.load(&key)).unwrap_or_default();
    let sizes = Sizes::parse(&widgets.sizes);
    let sized: Vec<_> = widgets
        .desktop_widgets
        .iter()
        .map(|&kind| (kind, sizes.of(kind, WidgetHost::Desktop)))
        .collect();
    let placed = grid.map_or_else(Vec::new, |g| {
        let size = crate::surfaces::desktop_widgets::GridSize {
            columns: g.columns,
            rows: g.rows,
        };
        place(&sized, &saved, size)
    });
    let keys = WidgetKeys {
        center: widgets.center.clone(),
        desktop: widgets.desktop_widgets.clone(),
        sizes,
        placed,
    };
    let layout = layout_of(&keys);
    let onedit = move |edit: WidgetEdit| {
        let Some(grid) = grid else { return };
        match edited(&keys, edit, grid) {
            Ok(next) => {
                refused.set(None);
                let places = places_to_save(&keys.placed, &saved, &next.placed);
                let change = WidgetsWrite {
                    center: next.center,
                    desktop: next.desktop,
                    sizes: next.sizes.text(),
                    output: key.clone(),
                    places,
                };
                eprintln!("edit_widgets.write={change:?}");
                match &writer {
                    Some(writer) => writer.write(change),
                    None => eprintln!("sill: Edit Widgets has no writer; nothing was saved"),
                }
            }
            Err(why) => {
                eprintln!("edit_widgets.refused={why}");
                refused.set(Some(why.words()));
            }
        }
    };
    let (catch, escape, done) = (link.clone(), link.clone(), link);
    rsx! {
        div {
            class: "sill-ew-catcher",
            onmounted: move |event| probe.on_mounted(event),
            onpointerdown: move |_| catch.close(),
        }
        div { class: "sill-ew-scope",
            Panel {
                label: "Edit Widgets".to_owned(),
                shown,
                on_hidden,
                onclose: move |()| escape.close(),
                width: Px(EDIT_WIDTH),
                edge: PanelEdge::Right,
                material: Material::Sheet,
                scrim: PanelScrim::None,
                div {
                    id: EDIT_PANEL_ID,
                    class: "sill-ew-column",
                    "data-overscroll": "band",
                    tabindex: "-1",
                    onmounted: move |event: MountedEvent| take_keyboard(event.data()),
                    div { class: "sill-ew-head",
                        span { class: "sill-ew-title", "Edit Widgets" }
                        Button {
                            variant: ButtonVariant::Primary,
                            label: "Done",
                            onclick: move |_| done.close(),
                        }
                    }
                    WidgetGallery { layout, onedit }
                    if let Some(why) = refused() {
                        div { class: "sill-ew-refused", role: "status", {why} }
                    }
                }
            }
        }
    }
}

/// The column takes the keyboard as it mounts, so Escape reaches the panel.
fn take_keyboard(element: Rc<MountedData>) {
    ds::focus_soon(element);
}
