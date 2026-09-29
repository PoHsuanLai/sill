//! Edit Widgets' root: one transparent `Ds` in the Sheet material, as large as the output below
//! the bar (`RootExtent::Viewport`), holding the catcher (a press on it closes) and quire's
//! `Panel` as a sheet at the bottom edge (the reference's gallery), under half the output so the
//! desktop's widgets stay in view above it and a widget added there is seen arriving. In the
//! sheet: the title and Done, then quire's `WidgetGallery` over the layout the keys make
//! (`widgets::layout::layout_of`), one preview per widget at its one size and no size control,
//! each edit applied with `widgets::layout::edited` against this output's desktop grid (measured
//! from the catcher, the same box the desktop widget layer measures) and written through the
//! [`EditWidgetsLink`]'s writer. An add is shown by the gallery itself, its button settling to
//! "Added" with a check; a removal, or why sill could not make an edit, is said under the gallery
//! ([`EditStatus`]). An add to the desktop also turns `widgets.desktop` on, so the widget is
//! seen.

use std::rc::Rc;

use dioxus::prelude::*;
use ds::widget::{DesktopGrid, WidgetEdit};
use ds::{
    Button, ButtonVariant, Material, Panel, PanelEdge, PanelScrim, Px, RootExtent, Shown,
    WidgetGallery, use_rect,
};
use shell_host::OutputInfo;
use sill_settings::{DesktopWidgets, WidgetsSettings};

use super::io::{places_to_save, use_phase};
use super::layer::EDIT_PANEL_ID;
use super::status::{EditCount, EditStatus, Outcome, outcome_of};
use crate::hooks::use_section;
use crate::links::EditWidgetsLink;
use crate::links::WidgetsWrite;
use crate::root::SurfaceRoot;
use crate::surfaces::desktop_widgets::{grid_size, place};
use crate::surfaces::notification_center::phase::Phase;
use crate::widgets::grid::OutputKey;
use crate::widgets::layout::{WidgetKeys, edited, layout_of};
use crate::widgets::{WidgetHost, desktop_size, provide_registry, widgets_css};

/// Edit Widgets' stylesheet (lint-clean under `Profile::Strict`, `tests/edit_widgets_lint.rs`).
pub const EDIT_WIDGETS_CSS: &str = include_str!("../../style/edit_widgets.css");

/// The sheet's width: the gallery's list (200), a Medium preview on its plate (344 and the
/// plate's padding), the placed column (280), and their gaps and the panel's padding.
const EDIT_WIDTH: f32 = 1040.0;

/// The sheet's height: the Medium preview and its buttons with the title row and the status
/// line. quire holds it under half the output, so the desktop's top-right columns, where new
/// widgets land, stay in view above it.
const EDIT_HEIGHT: f32 = 400.0;

/// Edit Widgets on `output`.
pub(super) fn edit_widgets(output: OutputInfo) -> Element {
    provide_registry();
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Sheet,
            extent: RootExtent::Viewport,
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
    let mut column = use_signal(|| None::<Rc<MountedData>>);
    let writer = link.writer().cloned();
    let file = link.places().cloned();
    let probe = use_rect();
    let widgets = use_section::<WidgetsSettings>()();
    let mut outcome = use_signal(Outcome::default);
    let mut count = use_hook(|| CopyValue::new(EditCount::default()));
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
    let sized: Vec<_> = widgets
        .desktop_widgets
        .iter()
        .map(|&kind| (kind, desktop_size(kind)))
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
        placed,
    };
    let desktop_shown = widgets.desktop;
    let layout = layout_of(&keys);
    let before = layout.clone();
    let onedit = move |edit: WidgetEdit| {
        let Some(grid) = grid else { return };
        let now = count.peek().next();
        count.set(now);
        let made = edited(&keys, edit.clone(), grid);
        if let Some(said) = outcome_of(&edit, &before, made.as_ref().map(|_| ()), now) {
            outcome.set(said);
        }
        match made {
            Ok(next) => {
                let places = places_to_save(&keys.placed, &saved, &next.placed);
                let change = WidgetsWrite {
                    center: next.center,
                    desktop: next.desktop,
                    shown: shown_after(&edit, desktop_shown),
                    output: key.clone(),
                    places,
                };
                eprintln!("edit_widgets.write={change:?}");
                match &writer {
                    Some(writer) => writer.write(change),
                    None => eprintln!("sill: Edit Widgets has no writer; nothing was saved"),
                }
            }
            Err(why) => eprintln!("edit_widgets.refused={why}"),
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
                height: Px(EDIT_HEIGHT),
                edge: PanelEdge::Bottom,
                material: Material::Sheet,
                scrim: PanelScrim::None,
                div {
                    id: EDIT_PANEL_ID,
                    class: "sill-ew-column",
                    "data-overscroll": "band",
                    tabindex: "-1",
                    onmounted: move |event: MountedEvent| {
                        let element = event.data();
                        column.set(Some(element.clone()));
                        take_keyboard(element);
                    },
                    // A pointer press anywhere in the panel leaves the keyboard with the panel:
                    // a press on a gallery button focuses nothing, and a Remove takes its
                    // button away with its row, so Escape reached nothing after one.
                    // Pointer only: a key's activation keeps its focus where it is.
                    onmouseup: move |_| {
                        if let Some(element) = column.peek().clone() {
                            take_keyboard(element);
                        }
                    },
                    div { class: "sill-ew-head",
                        span { class: "sill-ew-title", "Edit Widgets" }
                        Button {
                            variant: ButtonVariant::Primary,
                            label: "Done",
                            onclick: move |_| done.close(),
                        }
                    }
                    WidgetGallery { layout, onedit }
                    EditStatus { outcome: outcome() }
                }
            }
        }
    }
}

/// `widgets.desktop` after `edit`: an add to the desktop shows the desktop's widgets (it was
/// added to be seen); anything else leaves it as it was.
fn shown_after(edit: &WidgetEdit, now: DesktopWidgets) -> DesktopWidgets {
    match edit {
        WidgetEdit::Add {
            host: WidgetHost::Desktop,
            ..
        } => DesktopWidgets::On,
        WidgetEdit::Add {
            host: WidgetHost::Tile,
            ..
        }
        | WidgetEdit::Remove(_)
        | WidgetEdit::Resize(..)
        | WidgetEdit::Move(..) => now,
    }
}

/// The column takes the keyboard as it mounts, so Escape reaches the panel.
fn take_keyboard(element: Rc<MountedData>) {
    ds::focus_soon(element);
}

#[cfg(test)]
mod tests {
    use ds::catalog::PlacementId;
    use ds::widget::WidgetEdit;
    use ds::{WidgetHost, WidgetSize};
    use sill_settings::{DesktopWidgets, WidgetKind};

    use super::shown_after;
    use crate::widgets::kind_id;

    #[test]
    fn only_an_add_to_the_desktop_turns_the_desktops_widgets_on() {
        let add = |host| WidgetEdit::Add {
            kind: kind_id(WidgetKind::WorldClock),
            size: WidgetSize::Medium,
            host,
        };
        let cases = [
            (
                add(WidgetHost::Desktop),
                DesktopWidgets::Off,
                DesktopWidgets::On,
            ),
            (
                add(WidgetHost::Desktop),
                DesktopWidgets::On,
                DesktopWidgets::On,
            ),
            (
                add(WidgetHost::Tile),
                DesktopWidgets::Off,
                DesktopWidgets::Off,
            ),
            (
                WidgetEdit::Remove(PlacementId(1)),
                DesktopWidgets::Off,
                DesktopWidgets::Off,
            ),
            (
                WidgetEdit::Remove(PlacementId(1)),
                DesktopWidgets::On,
                DesktopWidgets::On,
            ),
        ];
        for (edit, now, want) in cases {
            assert_eq!(shown_after(&edit, now), want, "{edit:?} {now:?}");
        }
    }
}
