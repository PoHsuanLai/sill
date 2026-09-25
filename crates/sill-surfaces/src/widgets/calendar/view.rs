//! The Calendar widget's view (M10 lane a fills it): reads `use_agenda` (today, the events) and
//! the `calendar` settings, keeps the month shown in a signal (today's month on mount, stepped
//! by the previous and next buttons, back to today's month when the day turns), and draws
//! [`super::grid::month_grid`] with [`super::grid::busy_days`].

use dioxus::prelude::*;

use crate::widgets::WidgetHost;

/// The month, today marked.
#[component]
pub fn CalendarWidget(host: WidgetHost) -> Element {
    let _ = host;
    rsx! {}
}
