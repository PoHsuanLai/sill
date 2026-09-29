//! Widgets (design/20 §1.14): each kind on quire's widget interface. quire owns the
//! card (`ds::WidgetCard`: footprint, corner, inset, material, Space tint, title row, lift); a
//! widget is a `ds::Widget` (quire's `BatteryWidget`, `WorldClockWidget` and `MonthWidget`, and
//! sill's own [`up_next::UpNext`] and [`now_playing::NowPlaying`]), and sill only provides each
//! one's entries from its services ([`Widget`] picks the provider for a kind). Drawn the same in
//! two hosts: the notification center's widget slot below the history
//! (`surfaces::notification_center::CenterWidgets`) and the desktop widget layer
//! (`surfaces::desktop_widgets`).
//!
//! Which kinds are placed stays a closed enum in the settings (`sill_settings::WidgetKind`,
//! `widgets.center`, `widgets.desktop_widgets`) until `layout.toml` (design/28 section 5.6):
//! [`kind_id`] names each one as quire's registry does, and [`layout`] maps the keys to a
//! `ds::WidgetLayout` and back.

mod card;
mod place;
mod registry;
mod wake;

pub mod battery;
pub mod calendar;
pub mod grid;
pub mod layout;
pub mod now_playing;
pub mod up_next;
pub mod world_clock;

#[cfg(test)]
mod tests;

pub use card::Widget;
pub use ds::widget::cells;
pub use ds::{CardPresence, Lift, WidgetHost, WidgetSize};
pub use place::{Place, desktop_size, size_of};
pub use registry::{
    ALL_KINDS, kind_id, kind_of, provide_registry, registry, widget_element_id, widgets_css,
};
pub use sill_settings::WidgetKind;
pub use wake::{provide_widget_wake, use_widget_wake};
