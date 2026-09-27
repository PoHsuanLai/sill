//! Widgets (design/20 §1.14; sill FINDINGS "M10 freeze", "Widget interface port"): each kind on
//! quire's widget interface (quire CONSUMING "Widget interface (2026-09-27)"). quire owns the
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

pub mod battery;
pub mod calendar;
pub mod layout;
pub mod now_playing;
pub mod sizes;
pub mod up_next;
mod up_next_view;
mod wake;
pub mod world_clock;

pub use ds::widget::cells;
pub use ds::{Lift, WidgetHost, WidgetSize};
pub use sill_settings::WidgetKind;
pub use wake::{provide_widget_wake, use_widget_wake};

use dioxus::prelude::*;

/// Every kind, in the settings' order.
pub const ALL_KINDS: [WidgetKind; 5] = [
    WidgetKind::Calendar,
    WidgetKind::UpNext,
    WidgetKind::Battery,
    WidgetKind::NowPlaying,
    WidgetKind::WorldClock,
];

/// The size `kind` takes in `host` unless the person picked another (`widgets.sizes`,
/// [`sizes`]). In the notification center every tile spans the column (a medium tile's 344 fits
/// its 384 less the padding), the month two cells tall; on the desktop the compact month and
/// the batteries are small, the lists and the clocks' dials medium.
pub fn size_of(kind: WidgetKind, host: WidgetHost) -> WidgetSize {
    match (host, kind) {
        (WidgetHost::Tile, WidgetKind::Calendar) => WidgetSize::Large,
        (WidgetHost::Tile, _) => WidgetSize::Medium,
        (WidgetHost::Desktop, WidgetKind::Calendar | WidgetKind::Battery) => WidgetSize::Small,
        (WidgetHost::Desktop, WidgetKind::UpNext | WidgetKind::NowPlaying) => WidgetSize::Medium,
        (WidgetHost::Desktop, WidgetKind::WorldClock) => WidgetSize::Medium,
    }
}

/// `kind`'s name in quire's registry.
pub fn kind_id(kind: WidgetKind) -> ds::WidgetKind {
    use ds::Widget as _;
    match kind {
        WidgetKind::Calendar => ds::MonthWidget::kind(),
        WidgetKind::UpNext => up_next::UpNext::kind(),
        WidgetKind::Battery => ds::BatteryWidget::kind(),
        WidgetKind::NowPlaying => now_playing::NowPlaying::kind(),
        WidgetKind::WorldClock => ds::WorldClockWidget::kind(),
    }
}

/// The settings' kind named `id`; none for a widget sill does not place.
pub fn kind_of(id: &ds::WidgetKind) -> Option<WidgetKind> {
    ALL_KINDS.into_iter().find(|kind| kind_id(*kind) == *id)
}

/// The sizes `kind` draws, its first the one it is added at.
pub fn offered(kind: WidgetKind) -> &'static [WidgetSize] {
    use ds::Widget as _;
    match kind {
        WidgetKind::Calendar => ds::MonthWidget::sizes(),
        WidgetKind::UpNext => up_next::UpNext::sizes(),
        WidgetKind::Battery => ds::BatteryWidget::sizes(),
        WidgetKind::NowPlaying => now_playing::NowPlaying::sizes(),
        WidgetKind::WorldClock => ds::WorldClockWidget::sizes(),
    }
}

/// Every widget sill can place: quire's three and sill's two.
pub fn registry() -> ds::WidgetRegistry {
    let quire = ds::WidgetRegistry::quire();
    match quire
        .clone()
        .with::<up_next::UpNext>()
        .and_then(ds::WidgetRegistry::with::<now_playing::NowPlaying>)
    {
        Ok(all) => all,
        Err(taken) => {
            eprintln!("sill: widget registry: {taken}");
            quire
        }
    }
}

/// Provide [`registry`] to a host root's subtree (the desktop layer, the notification center,
/// Edit Widgets).
pub fn provide_registry() {
    ds::provide_widget_registry(registry());
}

/// The element id of a widget's card on the desktop layer: its input and blur region.
pub fn widget_element_id(kind: WidgetKind) -> String {
    let word = match kind {
        WidgetKind::Calendar => "calendar",
        WidgetKind::UpNext => "up-next",
        WidgetKind::Battery => "battery",
        WidgetKind::NowPlaying => "now-playing",
        WidgetKind::WorldClock => "world-clock",
    };
    format!("widget-{word}")
}

/// sill's own widgets' stylesheets (quire's widgets style themselves), for a host root to inject
/// beside its own: one widget's rules per file.
pub fn widgets_css() -> String {
    [up_next::CSS, now_playing::CSS].join("\n")
}

/// One widget of `kind` in `host` at `size`, on quire's card, named by [`widget_element_id`],
/// `lift`ed while the desktop drags it: the kind's provider feeds the card.
#[component]
pub fn Widget(
    kind: WidgetKind,
    host: WidgetHost,
    size: WidgetSize,
    #[props(default)] lift: Lift,
) -> Element {
    let place = Place { host, size, lift };
    match kind {
        WidgetKind::Calendar => rsx! { calendar::CalendarCard { place } },
        WidgetKind::UpNext => rsx! { up_next::UpNextCard { place } },
        WidgetKind::Battery => rsx! { battery::BatteryCard { place } },
        WidgetKind::NowPlaying => rsx! { now_playing::NowPlayingCard { place } },
        WidgetKind::WorldClock => rsx! { world_clock::WorldClockCard { place } },
    }
}

/// Where a provider's card is drawn: the host, the size and the lift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Place {
    pub host: WidgetHost,
    pub size: WidgetSize,
    pub lift: Lift,
}

#[cfg(test)]
mod tests {
    use super::{ALL_KINDS, WidgetHost, WidgetKind, cells, kind_id, kind_of, offered, size_of};
    use super::{registry, widget_element_id};

    #[test]
    fn each_widget_has_its_footprint_in_each_host() {
        let cases = [
            (WidgetKind::Calendar, (2, 2), (1, 1)),
            (WidgetKind::UpNext, (2, 1), (2, 1)),
            (WidgetKind::Battery, (2, 1), (1, 1)),
            (WidgetKind::NowPlaying, (2, 1), (2, 1)),
            (WidgetKind::WorldClock, (2, 1), (2, 1)),
        ];
        for (kind, tile, desktop) in cases {
            assert_eq!(cells(size_of(kind, WidgetHost::Tile)), tile, "{kind:?}");
            assert_eq!(
                cells(size_of(kind, WidgetHost::Desktop)),
                desktop,
                "{kind:?}"
            );
        }
    }

    #[test]
    fn each_kind_is_named_in_the_registry_and_offers_its_usual_sizes() {
        let cases = [
            (WidgetKind::Calendar, "quire.month"),
            (WidgetKind::UpNext, "sill.up-next"),
            (WidgetKind::Battery, "quire.battery"),
            (WidgetKind::NowPlaying, "sill.now-playing"),
            (WidgetKind::WorldClock, "quire.world-clock"),
        ];
        let registry = registry();
        for (kind, name) in cases {
            assert_eq!(kind_id(kind).as_str(), name);
            assert_eq!(kind_of(&kind_id(kind)), Some(kind));
            assert!(registry.get(&kind_id(kind)).is_some(), "{name}");
            for host in [WidgetHost::Tile, WidgetHost::Desktop] {
                assert!(
                    offered(kind).contains(&size_of(kind, host)),
                    "{kind:?} {host:?}"
                );
            }
        }
        assert_eq!(registry.all().len(), ALL_KINDS.len());
        assert_eq!(kind_of(&ds::WidgetKind::fixed("org.example.weather")), None);
    }

    #[test]
    fn element_ids_are_distinct() {
        let mut ids: Vec<_> = ALL_KINDS.into_iter().map(widget_element_id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), ALL_KINDS.len());
        assert_eq!(widget_element_id(WidgetKind::UpNext), "widget-up-next");
    }
}
