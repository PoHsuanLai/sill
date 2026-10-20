//! WidgetCard: any widget on quire's card (design/23-WIDGETS.md section 9.3). The card is the
//! only way a widget is drawn: it takes the widget's timeline, picks the entry that shows now
//! (`use_widget`), and draws it through the widget's own `view` inside a `WidgetFrame`, whose
//! corner, inset and title row are quire's alone.

use crate::widget::contract::{Widget, WidgetContext, fit};
use crate::widget::frame::WidgetFrame;
use crate::widget::kind::WidgetSize;
use crate::widget::timeline::{RefreshAsk, Timeline};
use crate::widget::use_widget::use_widget;
use dioxus::prelude::*;
use ds::motion::wake::WakeStamp;
use ds::root::common::Common;

/// `widget` at `size` (held to the sizes it draws), showing `timeline`'s entry for now. `wake`
/// replays the widget's appear motion when it changes (a host passes a new stamp as its widgets
/// come into view); `onrefresh` hears the timeline's refresh policy come due; `onintent` hears
/// the widget's controls; `common.id` names the card. The card writes `data-widget` with the
/// kind.
#[component]
pub fn WidgetCard<W: Widget>(
    widget: W,
    #[props(default)] timeline: Timeline<W::Entry>,
    #[props(default)] size: WidgetSize,
    #[props(default)] wake: WakeStamp,
    #[props(default)] common: Common,
    #[props(default)] onrefresh: Option<EventHandler<RefreshAsk>>,
    #[props(default)] onintent: Option<EventHandler<W::Intent>>,
) -> Element {
    let _ = widget;
    let size = fit::<W>(size);
    let entry = use_widget::<W>(&timeline, size, onrefresh);
    let cx = WidgetContext {
        size,
        wake,
        act: onintent,
    };
    rsx! {
        WidgetFrame { size, title: W::title(), common, kind: Some(W::kind()),
            {W::view(&entry, cx)}
        }
    }
}
