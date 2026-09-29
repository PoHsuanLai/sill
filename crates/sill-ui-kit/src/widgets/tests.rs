use super::registry::all_sized;
use super::{
    ALL_KINDS, WidgetHost, WidgetKind, cells, kind_id, kind_of, registry, size_of,
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
