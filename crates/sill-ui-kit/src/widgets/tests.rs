use super::registry::all_sized;
use super::{
    WidgetHost, WidgetKind, WidgetSize, cells, kind_id, kind_of, registry, size_of,
    widget_element_id,
};

#[test]
fn each_widget_has_its_footprint_in_each_host() {
    let cases = [
        (WidgetKind::Calendar, (2, 1), (1, 1)),
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
fn each_kind_is_named_in_the_registry_at_its_one_size() {
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
            // The gallery previews and adds this one size.
            assert_eq!(
                registry.get(&kind_id(kind)).map(|info| info.size_in(host)),
                Some(size_of(kind, host)),
                "{kind:?} {host:?}"
            );
        }
    }
    assert!(all_sized(ds::WidgetRegistry::quire()).is_ok());
    assert_eq!(registry.all().len(), WidgetKind::ALL.len());
    assert_eq!(kind_of(&ds::WidgetKind::fixed("org.example.weather")), None);
}

#[test]
fn element_ids_are_distinct() {
    let mut ids: Vec<_> = WidgetKind::ALL
        .iter()
        .copied()
        .map(widget_element_id)
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), WidgetKind::ALL.len());
    assert_eq!(widget_element_id(WidgetKind::UpNext), "widget-up-next");
}

#[test]
fn every_slug_and_desktop_size_is_pinned() {
    let table = [
        (
            WidgetKind::Calendar,
            "calendar",
            "widget-calendar",
            WidgetSize::Small,
        ),
        (
            WidgetKind::UpNext,
            "up-next",
            "widget-up-next",
            WidgetSize::Medium,
        ),
        (
            WidgetKind::Battery,
            "battery",
            "widget-battery",
            WidgetSize::Small,
        ),
        (
            WidgetKind::NowPlaying,
            "now-playing",
            "widget-now-playing",
            WidgetSize::Medium,
        ),
        (
            WidgetKind::WorldClock,
            "world-clock",
            "widget-world-clock",
            WidgetSize::Medium,
        ),
    ];
    let kinds: Vec<_> = table.iter().map(|row| row.0).collect();
    assert_eq!(kinds, WidgetKind::ALL);
    for (kind, slug, element, desktop) in table {
        assert_eq!(kind.slug(), slug);
        assert_eq!(widget_element_id(kind), element);
        assert_eq!(size_of(kind, WidgetHost::Desktop), desktop);
        assert_eq!(size_of(kind, WidgetHost::Tile), WidgetSize::Medium);
    }
}
