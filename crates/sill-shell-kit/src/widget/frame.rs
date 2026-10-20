//! WidgetFrame: the card a widget is drawn on (design/04-COMPONENTS.md "Widgets"; design/20
//! section 1.14). The frame owns the corner, the padding, the footprint and the title row, so a
//! widget draws only its content and writes no CSS for any of them.
//!
//! The card is a tile on the notification center's own Popover, as a `ModuleTile` is, its
//! footprint from `--widget-cell` and `--widget-gap`.

use crate::widget::contract::WidgetKind;
use crate::widget::kind::{WidgetSize, WidgetTitle};
use crate::widget::scope::use_frame_provider;
use dioxus::prelude::*;
use ds::base::word::Word;
use ds::components::content::text_runs::text;
use ds::root::common::Common;
use ds::style::icon::render::{Glyph, IconSize};

/// `children` on a widget's card, `size` on the grid unit. `kind` writes `data-widget` (a
/// `WidgetCard` passes its widget's kind). `title` draws a glyph and a name above the content.
/// `common.id` names the card.
///
/// The frame provides its `size` to its content (`widget_scope`), so content that must fit
/// the frame, a `MonthGrid` at `MonthDensity::Auto`, fits itself without being told.
#[component]
pub fn WidgetFrame(
    #[props(default)] size: WidgetSize,
    #[props(default)] title: Option<WidgetTitle>,
    #[props(default)] kind: Option<WidgetKind>,
    #[props(default)] common: Common,
    children: Element,
) -> Element {
    use_frame_provider(size);
    let kind = kind.map(|kind| kind.as_str().to_owned());
    let class = common.class("ds-widget");
    let data = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "data-size": size.slug(),
            "data-widget": kind,
            "aria-label": common.aria_label.clone(),
            onmounted: move |event| common.mounted(event),
            ..data,
            if let Some(title) = title {
                div { class: "ds-widget-title",
                    Glyph { icon: title.glyph, size: IconSize::Tiny }
                    span { class: "ds-widget-title-text", {text(&title.text)} }
                }
            }
            div { class: "ds-widget-body", {children} }
        }
    }
}
