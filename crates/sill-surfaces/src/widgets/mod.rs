//! Widgets (design/20 §1.14; sill FINDINGS "M10 freeze"): one component per
//! [`WidgetKind`], drawn the same in two hosts: the notification center's widget slot below the
//! history (`surfaces::notification_center::CenterWidgets`, macOS 11's Notification Center
//! widgets) and the desktop widget layer (`surfaces::desktop_widgets`, macOS Sonoma's desktop
//! widgets). [`Widget`] picks the kind's component and wraps it in quire's `WidgetFrame` (Q182:
//! the card, its corner, padding, footprint and title row); a kind's component draws its content
//! only.
//!
//! An enum and not a trait: the kinds are a closed set chosen in the settings
//! (`widgets.center`, `widgets.desktop_widgets`), so adding one is a compile error at every
//! place that must learn it (CONVENTIONS §0 "Prefer an enum to a trait for closed vocabulary").

pub mod battery;
pub mod calendar;
pub mod frame;
pub mod now_playing;
pub mod up_next;
pub mod world_clock;

pub use ds::{WidgetHost, WidgetSize};
pub use sill_settings::WidgetKind;

use dioxus::prelude::*;

/// A size's footprint on the desktop grid, in cells: (columns, rows). quire's `WidgetFrame`
/// sizes its card from the same unit (`--widget-cell`, `--widget-gap`).
pub fn cells(size: WidgetSize) -> (u16, u16) {
    match size {
        WidgetSize::Small => (1, 1),
        WidgetSize::Medium => (2, 1),
        WidgetSize::Large => (2, 2),
    }
}

/// The size `kind` takes in `host`. In the notification center every tile spans the column (a
/// medium tile's 344 fits its 384 less the padding), the month two cells tall since its weeks do
/// not fit in one; on the desktop the date face and the batteries are small, the lists and the
/// clocks' dials medium.
pub fn size_of(kind: WidgetKind, host: WidgetHost) -> WidgetSize {
    match (host, kind) {
        (WidgetHost::Tile, WidgetKind::Calendar) => WidgetSize::Large,
        (WidgetHost::Tile, _) => WidgetSize::Medium,
        (WidgetHost::Desktop, WidgetKind::Calendar | WidgetKind::Battery) => WidgetSize::Small,
        (WidgetHost::Desktop, WidgetKind::UpNext | WidgetKind::NowPlaying) => WidgetSize::Medium,
        (WidgetHost::Desktop, WidgetKind::WorldClock) => WidgetSize::Medium,
    }
}

/// The element id of a widget's frame on the desktop layer: its input and blur region.
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

/// Every widget's stylesheet and the frame's, for a host root to inject beside its own (the
/// notification center's panel, the desktop layer): one widget's rules per file.
pub fn widgets_css() -> String {
    [
        frame::CSS,
        calendar::CSS,
        up_next::CSS,
        battery::CSS,
        now_playing::CSS,
        world_clock::CSS,
    ]
    .join("\n")
}

/// One widget of `kind` in `host`, on quire's frame, named by [`widget_element_id`].
#[component]
pub fn Widget(kind: WidgetKind, host: WidgetHost) -> Element {
    rsx! {
        ds::WidgetFrame {
            size: size_of(kind, host),
            host,
            title: frame::frame_title(kind),
            id: widget_element_id(kind),
            match kind {
                WidgetKind::Calendar => rsx! { calendar::CalendarWidget { host } },
                WidgetKind::UpNext => rsx! { up_next::UpNextWidget { host } },
                WidgetKind::Battery => rsx! { battery::BatteryWidget { host } },
                WidgetKind::NowPlaying => rsx! { now_playing::NowPlayingWidget { host } },
                WidgetKind::WorldClock => rsx! { world_clock::WorldClockWidget { host } },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WidgetHost, WidgetKind, cells, size_of, widget_element_id};

    const ALL: [WidgetKind; 5] = [
        WidgetKind::Calendar,
        WidgetKind::UpNext,
        WidgetKind::Battery,
        WidgetKind::NowPlaying,
        WidgetKind::WorldClock,
    ];

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
    fn element_ids_are_distinct() {
        let mut ids: Vec<_> = ALL.into_iter().map(widget_element_id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), ALL.len());
        assert_eq!(widget_element_id(WidgetKind::UpNext), "widget-up-next");
    }
}
