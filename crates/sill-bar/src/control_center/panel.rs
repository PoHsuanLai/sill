//! The control center's render: the page state ([`super::step_page`]), the modules in
//! `control_center.modules` order on quire's `ModuleGrid` (each from
//! `crate::control_center::modules`), the detail panes behind quire's `PaneSwitcher`,
//! and Escape.

use ds::focus::soon::focus_soon;
use std::rc::Rc;

use dioxus::prelude::*;
use ds::components::lists::preview::switcher::PaneSwitcher;
use ds::prelude::*;
use ds_motion::pane_slide::Pane;
use ds_shell::control_center::module_grid::GridColumns;
use ds_shell::prelude::*;
use sill_model::ids::OutputName;
use sill_settings::ControlCenterModule;

use super::modules::bluetooth_ask::{TurnOffAlert, provide_power_ask};
use super::modules::{CcCx, KINDS, Module, module};
use super::page::{CcPage, Detail, PageInput, PageOut, step_page};
use super::pages::foot::ClosePanel;
use sill_ui_kit::hooks::use_settings;

/// The panel's greatest height on its output ([`super::height_cap`]), which the bar's popup
/// root provides; past it the panel scrolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelCap(pub i32);

/// The panel.
pub(crate) fn control_center(
    output: Option<OutputName>,
    on_close: EventHandler<()>,
    on_page: Option<EventHandler<CcPage>>,
) -> Element {
    let cc = use_settings()().control_center;
    let cap = try_use_context::<PanelCap>();
    let mut page = use_signal(CcPage::default);
    // The detail pane last opened: still drawn while it slides out.
    let mut opened = use_signal(|| Detail::Wifi);
    let home = use_keyboard_home();
    use_context_provider(|| ClosePanel(Callback::new(move |()| on_close.call(()))));
    provide_power_ask();
    let go = use_callback(move |input: PageInput| {
        // The slide is quire's `PaneSwitcher`'s, played on `shown`.
        let (next, _slide, out) = step_page(*page.peek(), input);
        match out {
            PageOut::Close => on_close.call(()),
            PageOut::Stay if next != *page.peek() => {
                if let CcPage::Detail(detail) = next {
                    opened.set(detail);
                }
                page.set(next);
                // The pressed chevron or back button is gone with its pane.
                home.take();
                if let Some(on_page) = on_page {
                    on_page.call(next);
                }
            }
            PageOut::Stay => {}
        }
    });
    let current = page();
    let root = rsx! {
        Grid {
            modules: cc.modules.clone(),
            columns: GridColumns(cc.grid_columns.0),
            gap: Px(f32::from(cc.grid_gap_px.0)),
            output: output.clone(),
            on_detail: move |detail: Detail| go.call(PageInput::Open(detail)),
        }
    };
    let cx = CcCx {
        output: output.clone(),
        on_detail: EventHandler::new(|_| {}),
        on_back: Some(EventHandler::new(move |()| go.call(PageInput::Back))),
    };
    let detail = KINDS
        .into_iter()
        .find_map(|kind| module(kind).detail(opened(), &cx))
        .unwrap_or_else(|| rsx! {});
    let style = panel_style(cc.width_px.0, cc.grid_padding_px.0, cap);
    rsx! {
        div {
            class: "sill-cc",
            tabindex: "-1",
            style,
            onmounted: move |event: MountedEvent| home.mounted(event),
            onkeydown: move |event: KeyboardEvent| {
                // Escape on a detail page goes back; on the grid it reaches the popover, which
                // closes.
                if event.key() == Key::Escape && *page.peek() != CcPage::Modules {
                    event.stop_propagation();
                    event.prevent_default();
                    go.call(PageInput::Escape);
                }
            },
            PaneSwitcher { shown: pane_of(current), root, detail }
            // The alert slot: last in the positioned body, which it covers (quire's `Alert`,
            // `Flow::Inline`).
            TurnOffAlert { on_answered: move |()| home.take() }
        }
    }
}

/// quire's pane for a page.
pub(crate) fn pane_of(page: CcPage) -> Pane {
    match page {
        CcPage::Modules => Pane::Root,
        CcPage::Detail(_) => Pane::Detail,
    }
}

/// The panel's own element, which holds the keyboard so Escape reaches the panel (and the
/// popover around it) with nothing inside focused: as it mounts, and after a page change takes
/// the focused button away.
#[derive(Clone, Copy)]
pub(crate) struct KeyboardHome(Signal<Option<Rc<MountedData>>>);

impl KeyboardHome {
    /// The panel mounted: keep it and give it the keyboard.
    pub(crate) fn mounted(mut self, event: MountedEvent) {
        let element = event.data();
        self.0.set(Some(element.clone()));
        focus_soon(element);
    }

    /// Give the panel the keyboard again.
    pub(crate) fn take(self) {
        if let Some(element) = self.0.peek().clone() {
            focus_soon(element);
        }
    }
}

pub(crate) fn use_keyboard_home() -> KeyboardHome {
    KeyboardHome(use_signal(|| None))
}

/// The panel's inline box: `control_center.width_px` wide, `grid_padding_px` inside, scrolling
/// past the cap.
pub(crate) fn panel_style(width: u16, padding: u16, cap: Option<PanelCap>) -> String {
    let cap = cap.map_or(String::new(), |PanelCap(px)| {
        format!(";max-height:{px}px;overflow-y:auto")
    });
    format!("width:{width}px;padding:{padding}px;box-sizing:border-box{cap}")
}

/// The modules on quire's grid (`control_center.grid_columns` and `grid_gap_px`), in order: a
/// tile takes a column, a full-width module the row (as [`super::place`] lays them out). The
/// panel pads the grid and the detail panes alike with `grid_padding_px`,
/// so the grid pads nothing of its own.
#[component]
fn Grid(
    modules: Vec<ControlCenterModule>,
    columns: GridColumns,
    gap: Px,
    output: Option<OutputName>,
    on_detail: EventHandler<Detail>,
) -> Element {
    rsx! {
        ModuleGrid { columns, gap, padding: Px(0.0),
            for kind in modules {
                Module { key: "{kind:?}", kind, output: output.clone(), on_detail }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PanelCap, pane_of, panel_style};
    use crate::control_center::{CcPage, Detail};
    use ds_motion::pane_slide::Pane;

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

    #[test]
    fn a_detail_page_is_quires_detail_pane() {
        assert_eq!(pane_of(CcPage::Modules), Pane::Root);
        assert_eq!(pane_of(CcPage::Detail(Detail::Bluetooth)), Pane::Detail);
    }
}
