//! The control center's render: the page state ([`super::step_page`]), the module grid laid out
//! by [`super::place`] (each module from `crate::surfaces::control_center::modules`), the detail
//! pages, the slide between them, and Escape.

use dioxus::prelude::*;
use sill_services::OutputName;

use super::geometry::{Placed, Span, place};
use super::modules::Module;
use super::page::{CcPage, Detail, PageInput, PageOut, Slide, step_page};
use super::pages::foot::ClosePanel;
use super::pages::{BluetoothDevices, WifiNetworks};
use super::parts::PaneSwitcher;
use crate::hooks::use_settings;

/// The panel's greatest height on its output ([`super::height_cap`]), which the bar's popup
/// root provides; past it the panel scrolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelCap(pub i32);

/// The panel.
pub(crate) fn control_center(
    output: Option<OutputName>,
    on_close: EventHandler<()>,
    on_page: EventHandler<CcPage>,
) -> Element {
    let cc = use_settings()().control_center;
    let cap = try_use_context::<PanelCap>();
    let mut page = use_signal(CcPage::default);
    let mut slide = use_signal(|| Slide::None);
    use_context_provider(|| ClosePanel(Callback::new(move |()| on_close.call(()))));
    let go = use_callback(move |input: PageInput| {
        let (next, moved, out) = step_page(*page.peek(), input);
        match out {
            PageOut::Close => on_close.call(()),
            PageOut::Stay if next != *page.peek() => {
                page.set(next);
                slide.set(moved);
                on_page.call(next);
            }
            PageOut::Stay => {}
        }
    });
    let current = page();
    let pane = match current {
        CcPage::Modules => rsx! {
            Grid {
                placed: place(&cc.modules, cc.grid_columns.0),
                gap: cc.grid_gap_px.0,
                output,
                on_detail: move |detail: Detail| go.call(PageInput::Open(detail)),
            }
        },
        CcPage::Detail(Detail::Wifi) => rsx! {
            WifiNetworks { on_back: move |()| go.call(PageInput::Back) }
        },
        CcPage::Detail(Detail::Bluetooth) => rsx! {
            BluetoothDevices { on_back: move |()| go.call(PageInput::Back) }
        },
    };
    let style = panel_style(cc.width_px.0, cc.grid_padding_px.0, cap);
    rsx! {
        div {
            class: "sill-cc",
            style,
            onkeydown: move |event: KeyboardEvent| {
                // Escape on a detail page goes back; on the grid it reaches the popover, which
                // closes.
                if event.key() == Key::Escape && *page.peek() != CcPage::Modules {
                    event.stop_propagation();
                    event.prevent_default();
                    go.call(PageInput::Escape);
                }
            },
            PaneSwitcher { pane: current, slide: slide(), {pane} }
        }
    }
}

/// The panel's inline box: `control_center.width_px` wide, `grid_padding_px` inside, scrolling
/// past the cap.
pub(crate) fn panel_style(width: u16, padding: u16, cap: Option<PanelCap>) -> String {
    let cap = cap.map_or(String::new(), |PanelCap(px)| {
        format!(";max-height:{px}px;overflow-y:auto")
    });
    format!("width:{width}px;padding:{padding}px;box-sizing:border-box{cap}")
}

/// The module grid, row by row.
#[component]
fn Grid(
    placed: Vec<Placed>,
    gap: u16,
    output: Option<OutputName>,
    on_detail: EventHandler<Detail>,
) -> Element {
    let gap = format!("gap:{gap}px");
    rsx! {
        div { class: "sill-cc-grid", style: gap.clone(),
            for row in rows_of(&placed) {
                div { key: "{row.first().map_or(0, |cell| cell.row)}", class: "sill-cc-row", style: gap.clone(),
                    for cell in row {
                        div {
                            key: "{cell.module:?}",
                            class: "sill-cc-cell",
                            "data-span": span_slug(cell.span),
                            Module { module: cell.module, output: output.clone(), on_detail }
                        }
                    }
                }
            }
        }
    }
}

/// `placed` grouped into its rows, in order.
pub(crate) fn rows_of(placed: &[Placed]) -> Vec<Vec<Placed>> {
    placed
        .iter()
        .fold(Vec::<Vec<Placed>>::new(), |mut rows, &cell| {
            match rows.last_mut() {
                Some(row) if row.first().is_some_and(|first| first.row == cell.row) => {
                    row.push(cell)
                }
                _ => rows.push(vec![cell]),
            }
            rows
        })
}

fn span_slug(span: Span) -> &'static str {
    match span {
        Span::One => "one",
        Span::Row => "row",
    }
}

#[cfg(test)]
mod tests {
    use super::{PanelCap, panel_style, rows_of};
    use crate::surfaces::control_center::place;
    use sill_settings::ControlCenterSettings;

    #[test]
    fn the_grid_draws_row_by_row() {
        let placed = place(&ControlCenterSettings::default().modules, 2);
        let widths: Vec<usize> = rows_of(&placed).iter().map(Vec::len).collect();
        assert_eq!(widths, [2, 1, 1, 1, 1, 1, 1]);
        assert!(rows_of(&[]).is_empty());
    }

    #[test]
    fn the_panel_scrolls_past_its_cap() {
        assert_eq!(
            panel_style(320, 12, None),
            "width:320px;padding:12px;box-sizing:border-box"
        );
        assert_eq!(
            panel_style(320, 12, Some(PanelCap(600))),
            "width:320px;padding:12px;box-sizing:border-box;max-height:600px;overflow-y:auto"
        );
    }
}
