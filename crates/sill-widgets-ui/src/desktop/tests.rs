use shell_host::{BlurSpec, RegionSpec};
use sill_settings::WidgetKind;

use super::model::{merged, recorded, widget_regions};
use sill_ui_kit::widgets::grid::{GridSlot, Placement};

fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
    Placement {
        kind,
        slot: GridSlot { column, row },
    }
}

#[test]
fn a_widget_taken_off_keeps_its_saved_place() {
    let placed = [at(WidgetKind::Battery, 9, 0)];
    let saved = [at(WidgetKind::Battery, 8, 0), at(WidgetKind::UpNext, 1, 1)];
    assert_eq!(
        merged(&placed, &saved),
        vec![at(WidgetKind::Battery, 9, 0), at(WidgetKind::UpNext, 1, 1)]
    );
    assert_eq!(merged(&[], &[]), Vec::new());
}

#[test]
fn a_layout_records_new_places_but_never_writes_over_saved_ones() {
    let cases = [
        (
            "a clamped place is drawn at the edge but kept as saved",
            vec![at(WidgetKind::Battery, 4, 0)],
            vec![at(WidgetKind::Battery, 9, 0)],
            vec![at(WidgetKind::Battery, 9, 0)],
        ),
        (
            "a widget seen for the first time is recorded after the saved ones",
            vec![
                at(WidgetKind::Calendar, 6, 0),
                at(WidgetKind::Battery, 0, 0),
            ],
            vec![at(WidgetKind::Battery, 0, 0), at(WidgetKind::UpNext, 3, 2)],
            vec![
                at(WidgetKind::Battery, 0, 0),
                at(WidgetKind::UpNext, 3, 2),
                at(WidgetKind::Calendar, 6, 0),
            ],
        ),
        ("nothing", Vec::new(), Vec::new(), Vec::new()),
    ];
    for (name, placed, saved, want) in cases {
        assert_eq!(recorded(&placed, &saved), want, "{name}");
    }
}

#[test]
fn the_regions_are_the_frames_or_nothing() {
    assert_eq!(widget_regions(&[]), (RegionSpec::Empty, BlurSpec::None));
    let (input, blur) = widget_regions(&[
        at(WidgetKind::Calendar, 0, 0),
        at(WidgetKind::Battery, 1, 0),
    ]);
    let RegionSpec::Elements(regions) = &input else {
        panic!("element regions: {input:?}");
    };
    let ids: Vec<_> = regions.iter().map(|r| r.id.clone()).collect();
    assert_eq!(
        ids,
        vec![
            shell_host::ElementId::new("widget-calendar"),
            shell_host::ElementId::new("widget-battery"),
        ]
    );
    assert_eq!(blur, BlurSpec::Region(input.clone()));
}
