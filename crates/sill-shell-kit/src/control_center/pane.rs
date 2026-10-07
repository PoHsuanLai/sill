//! PaneHeader and PaneFooter: the head and the foot of a control-center module's detail pane
//! (design/30 section 2.10, `PaneSwitcher`). The header is an optional back button, the pane's
//! title and a trailing element (the feature's switch); the footer holds one action under a
//! hairline. The body between them is the caller's.
//!
//! Markup: `div.ds-pane-header` holding `button.ds-button` (the back button, a large toolbar
//! bezel with a chevron), `span.ds-label` (the title) and the trailing element;
//! `div.ds-pane-footer` holding its children.

use dioxus::prelude::*;
use ds::base::press::Press;
use ds::components::content::label::{Label, LabelStyle};
use ds::components::content::text_runs::TextLine;
use ds::components::controls::button::Button;
use ds::components::controls::button_model::{Bezel, ImagePosition};
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::tokens::control_size::ControlSize;

/// A detail pane's header: `back` draws a back button that calls it (a module's own dropdown, which
/// has nothing to go back to, passes none), then `title`, then `trailing` at the far end.
#[component]
pub fn PaneHeader(
    #[props(into)] title: TextLine,
    #[props(default)] back: Option<EventHandler<Press>>,
    #[props(default)] trailing: Option<Element>,
    #[props(default)] common: Common,
) -> Element {
    let class = common.class("ds-pane-header");
    let data = common.data_attributes();
    rsx! {
        div {
            id: common.id.clone(),
            class,
            onmounted: move |event| common.mounted(event),
            ..data,
            if let Some(back) = back {
                Button {
                    bezel: Bezel::Toolbar,
                    size: ControlSize::Large,
                    image: ImagePosition::Only,
                    icon: Icon::ChevronLeft,
                    label: "Back",
                    onclick: move |press| back.call(press),
                }
            }
            Label {
                text: title,
                style: LabelStyle::Headline,
                common: Common::default(),
            }
            if let Some(trailing) = trailing {
                {trailing}
            }
        }
    }
}

/// A detail pane's footer: `children` (an action such as "Wi-Fi Settings…") under a hairline.
#[component]
pub fn PaneFooter(#[props(default)] common: Common, children: Element) -> Element {
    let class = common.class("ds-pane-footer");
    let data = common.data_attributes();
    rsx! {
        div {
            id: common.id.clone(),
            class,
            onmounted: move |event| common.mounted(event),
            ..data,
            {children}
        }
    }
}
