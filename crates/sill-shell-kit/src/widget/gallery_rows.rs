//! Edit Widgets' placed lists: what is on the desktop and in the notification
//! center, a row each with the widget's name and Remove. A row that arrives while the gallery is
//! open is brought into view in the placed column, moving the column the least that shows it.
//! Rows already placed when the gallery opened sit still.

use crate::catalog::placement::PlacementId;
use crate::widget::gallery::GalleryWords;
use crate::widget::kind::WidgetHost;
use crate::widget::layout::{WidgetAt, WidgetEdit, WidgetLayout, WidgetPlacement};
use crate::widget::registry::WidgetRegistry;
use dioxus::core::current_scope_id;
use dioxus::prelude::*;
use ds::base::word::Word;
use ds::components::content::text_runs::{TextLine, text};
use ds::components::controls::button::Button;
use ds::components::controls::button_model::ButtonRole;
use ds::host::measure::MountedRef;
use ds::host::reveal::reveal;
use ds::prelude::*;
use ds::style::task::spawn_in;
use ds::style::tokens::control_size::ControlSize;
use std::collections::BTreeSet;

/// The placements on `host`, in their order there.
pub(crate) fn on_host(layout: &WidgetLayout, host: WidgetHost) -> Vec<WidgetPlacement> {
    let mut items: Vec<WidgetPlacement> = layout
        .items()
        .iter()
        .filter(|item| item.at.host() == host)
        .cloned()
        .collect();
    items.sort_by_key(at_key);
    items
}

/// A placement's sort key on its surface: the column's order, or the desktop's cell by row.
fn at_key(item: &WidgetPlacement) -> (u16, u16) {
    match item.at {
        WidgetAt::Desktop(cell) => (cell.row, cell.column),
        WidgetAt::Center(order) => (order.0, 0),
    }
}

/// What the placed column needs from the gallery: the registry for names, the layout, the rows
/// that arrived with this layout, and the edit handler.
pub(crate) struct Placed<'a> {
    pub(crate) registry: &'a WidgetRegistry,
    pub(crate) layout: &'a WidgetLayout,
    pub(crate) arrived: &'a BTreeSet<PlacementId>,
    pub(crate) onedit: EventHandler<WidgetEdit>,
    pub(crate) words: &'a GalleryWords,
}

/// What is placed on each surface, in a column that scrolls on its own.
pub(crate) fn placed(placed: Placed<'_>, mut list: CopyValue<Option<MountedRef>>) -> Element {
    let Placed {
        registry,
        layout,
        arrived,
        onedit,
        words,
    } = placed;
    let surfaces = [
        (WidgetHost::Desktop, words.desktop.clone()),
        (WidgetHost::Tile, words.center.clone()),
    ];
    rsx! {
        div { class: "ds-widget-gallery-placed",
            onmounted: move |event: MountedEvent| list.set(Some(MountedRef(event.data()))),
            for (host, heading) in surfaces {
                div { key: "{host.slug()}", class: "ds-widget-gallery-surface", "data-host": host.slug(),
                    SectionHeader { title: heading.plain_text() }
                    if on_host(layout, host).is_empty() {
                        span { class: "ds-widget-gallery-none", {text(&words.none)} }
                    }
                    for item in on_host(layout, host) {
                        PlacedRow {
                            key: "{item.id.0}",
                            id: item.id,
                            name: name_of(registry, &item),
                            remove: words.remove.clone(),
                            arrived: arrived.contains(&item.id),
                            list,
                            onedit,
                        }
                    }
                }
            }
        }
    }
}

/// The widget's name in the registry, or its kind when the registry does not hold it.
fn name_of(registry: &WidgetRegistry, item: &WidgetPlacement) -> String {
    registry.get(&item.kind).map_or_else(
        || item.kind.as_str().to_owned(),
        |info| info.name.plain_text(),
    )
}

/// One placed widget's row: its name and Remove. `arrived` is read once, as the row mounts: a
/// row that arrived is brought into view in `list`; later renders never replay it.
#[component]
fn PlacedRow(
    id: PlacementId,
    name: String,
    remove: TextLine,
    arrived: bool,
    list: CopyValue<Option<MountedRef>>,
    onedit: EventHandler<WidgetEdit>,
) -> Element {
    let arrived = use_hook(|| arrived);
    let scope = use_hook(current_scope_id);
    rsx! {
        div {
            class: "ds-widget-gallery-row",
            onmounted: move |event: MountedEvent| {
                let (true, Some(scroller)) = (arrived, list.peek().clone()) else {
                    return;
                };
                let row = MountedRef(event.data());
                spawn_in(scope, async move {
                    let _ = reveal(&scroller.0, &row.0).await;
                });
            },
            span { class: "ds-widget-gallery-row-name", "{name}" }
            Button { role: ButtonRole::Destructive, size: ControlSize::Mini, label: remove,
                onclick: move |_| onedit.call(WidgetEdit::Remove(id)) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{at_key, on_host};
    use crate::widget::battery::BatteryWidget;
    use crate::widget::calendar::MonthWidget;
    use crate::widget::clock::WorldClockWidget;
    use crate::widget::contract::Widget;
    use crate::widget::kind::{WidgetHost, WidgetSize};
    use crate::widget::layout::{DesktopGrid, WidgetEdit, WidgetLayout, apply};

    #[test]
    fn each_surface_lists_only_its_own_in_their_order() {
        const GRID: DesktopGrid = DesktopGrid {
            columns: 4,
            rows: 3,
        };
        let adds = [
            (BatteryWidget::kind(), WidgetHost::Desktop),
            (MonthWidget::kind(), WidgetHost::Tile),
            (WorldClockWidget::kind(), WidgetHost::Desktop),
        ];
        let layout = adds
            .into_iter()
            .fold(WidgetLayout::default(), |layout, (kind, host)| {
                let edit = WidgetEdit::Add {
                    kind,
                    size: WidgetSize::Small,
                    host,
                };
                apply(layout.clone(), edit, GRID).unwrap_or(layout)
            });
        let desktop = on_host(&layout, WidgetHost::Desktop);
        assert_eq!(desktop.len(), 2);
        assert!(
            desktop
                .windows(2)
                .all(|pair| at_key(&pair[0]) <= at_key(&pair[1]))
        );
        assert_eq!(on_host(&layout, WidgetHost::Tile).len(), 1);
    }
}
