//! WidgetSlotGuide: where a dragged widget will land (design/23-WIDGETS.md section
//! 9.8). While a host drags a desktop widget, it draws this at the snap cell under the pointer:
//! the card's footprint for `size`, a quiet translucent plate in the card's own corner, faded in
//! as the drag reaches the cell. The host places it; quire draws it, so a shell draws nothing of
//! its own for the guide.

use crate::widget::kind::{WidgetHost, WidgetSize};
use dioxus::prelude::*;
use ds::base::word::Word;
use ds::motion::anim::Anim;
use ds::root::common::Common;

/// `div.ds-widget-slot[data-size][data-host]`: the footprint of a `size` widget in `host`.
/// Decorative (`aria-hidden`): the drag's own announcement says where it goes.
#[component]
pub fn WidgetSlotGuide(
    #[props(default)] size: WidgetSize,
    #[props(default)] host: WidgetHost,
    #[props(default)] common: Common,
) -> Element {
    let class = common.class(&format!("ds-widget-slot {}", Anim::Fade.class()));
    let data = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "data-size": size.slug(),
            "data-host": host.slug(),
            "aria-hidden": "true",
            onmounted: move |event| common.mounted(event),
            ..data,
        }
    }
}
