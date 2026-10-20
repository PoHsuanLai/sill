//! The notification center's widgets (design/20 §1.14): Calendar, then Up Next, under the
//! history, each on quire's widget interface. quire owns the card
//! (`sill_shell_kit::prelude::WidgetCard`); a widget is a `sill_shell_kit::prelude::Widget`
//! (quire's `MonthWidget` and sill's own [`up_next::UpNext`]), and sill only provides each one's
//! entries from its services ([`Widget`] picks the provider for a kind, [`with_provider`]).

mod card;
mod wake;

pub mod calendar;
pub mod up_next;

pub use card::{Card, Feed, OnProvider, Widget, WidgetProvider, card, with_provider};
pub use sill_settings::WidgetKind;
pub use sill_shell_kit::prelude::{WidgetHost, WidgetSize};
pub use wake::{provide_widget_wake, use_widget_wake};

/// Where the widgets are drawn: the notification center's column, each card spanning it at
/// Medium (336 fits its 360 less the padding).
pub const SIZE: WidgetSize = WidgetSize::Medium;

/// sill's own widgets' stylesheets (quire's widgets style themselves), for the notification
/// center to inject beside its own.
pub fn widgets_css() -> &'static str {
    up_next::CSS
}
