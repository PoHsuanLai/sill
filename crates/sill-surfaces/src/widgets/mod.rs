//! Widgets (design/20 §1.14; sill FINDINGS "M10 freeze"): one component per
//! [`WidgetKind`], drawn the same in two hosts: the notification center's widget slot below the
//! history (`surfaces::notification_center::CenterWidgets`, macOS 11's Notification Center
//! widgets) and the desktop widget layer (`surfaces::desktop_widgets`, macOS Sonoma's desktop
//! widgets). [`Widget`] picks the kind's component and wraps it in the [`frame::WidgetFrame`];
//! a kind's component draws its content only.
//!
//! An enum and not a trait: the kinds are a closed set chosen in the settings
//! (`widgets.center`, `widgets.desktop_widgets`), so adding one is a compile error at every
//! place that must learn it (CONVENTIONS §0 "Prefer an enum to a trait for closed vocabulary").
//!
//! Every component here draws nothing until its lane fills it (the freeze renders no `todo!()`,
//! since the notification center's render tests mount the slot).

pub mod battery;
pub mod calendar;
pub mod frame;
pub mod now_playing;
pub mod up_next;
pub mod world_clock;

pub use sill_settings::WidgetKind;

use dioxus::prelude::*;

/// Where a widget is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetHost {
    /// In the notification center's column, full width, on its Popover.
    Center,
    /// On the desktop layer, on its own `Material::Widget` card, on the grid.
    Desktop,
}

/// A widget's size class, macOS's: a small one is one desktop grid cell, a medium one two cells
/// wide, a large one two by two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetSize {
    Small,
    Medium,
    Large,
}

impl WidgetSize {
    /// Its footprint on the desktop grid, in cells: (columns, rows).
    pub fn cells(self) -> (u16, u16) {
        match self {
            WidgetSize::Small => (1, 1),
            WidgetSize::Medium => (2, 1),
            WidgetSize::Large => (2, 2),
        }
    }
}

/// The size `kind` takes in `host`: every widget spans the notification center's column
/// (medium); on the desktop the month and the clocks are small, the lists medium.
pub fn size_of(kind: WidgetKind, host: WidgetHost) -> WidgetSize {
    match (host, kind) {
        (WidgetHost::Center, _) => WidgetSize::Medium,
        (WidgetHost::Desktop, WidgetKind::Calendar | WidgetKind::Battery) => WidgetSize::Small,
        (WidgetHost::Desktop, WidgetKind::WorldClock) => WidgetSize::Small,
        (WidgetHost::Desktop, WidgetKind::UpNext | WidgetKind::NowPlaying) => WidgetSize::Medium,
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
/// notification center's panel, the desktop layer): one lane's rules per file.
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

/// One widget of `kind` in `host`, in its frame.
#[component]
pub fn Widget(kind: WidgetKind, host: WidgetHost) -> Element {
    let size = size_of(kind, host);
    rsx! {
        frame::WidgetFrame { kind, host, size,
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
    use super::{WidgetHost, WidgetKind, WidgetSize, size_of, widget_element_id};

    const ALL: [WidgetKind; 5] = [
        WidgetKind::Calendar,
        WidgetKind::UpNext,
        WidgetKind::Battery,
        WidgetKind::NowPlaying,
        WidgetKind::WorldClock,
    ];

    #[test]
    fn every_widget_spans_the_center_and_has_its_desktop_size() {
        for kind in ALL {
            assert_eq!(size_of(kind, WidgetHost::Center), WidgetSize::Medium);
        }
        let desktop = |kind| size_of(kind, WidgetHost::Desktop).cells();
        assert_eq!(desktop(WidgetKind::Calendar), (1, 1));
        assert_eq!(desktop(WidgetKind::UpNext), (2, 1));
        assert_eq!(desktop(WidgetKind::NowPlaying), (2, 1));
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
