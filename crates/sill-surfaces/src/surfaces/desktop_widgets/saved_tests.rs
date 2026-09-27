use sill_settings::WidgetKind;

use super::{OutputKey, PlacementFile, SavedEntry, decode, encode, for_output, with_output};
use crate::surfaces::desktop_widgets::placement::{GridSlot, Placement};

fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
    Placement {
        kind,
        slot: GridSlot { column, row },
    }
}

fn on(output: &str, placement: Placement) -> SavedEntry {
    SavedEntry {
        output: OutputKey::of(Some(output)),
        placement,
    }
}

fn anywhere(placement: Placement) -> SavedEntry {
    SavedEntry {
        output: OutputKey::Any,
        placement,
    }
}

#[test]
fn entries_round_trip_through_the_file_format() {
    let entries = vec![
        on("DP-1", at(WidgetKind::Calendar, 9, 0)),
        on("DP-1", at(WidgetKind::UpNext, 7, 1)),
        on("HDMI-A-1", at(WidgetKind::NowPlaying, 0, 4)),
        anywhere(at(WidgetKind::WorldClock, 2, 2)),
        on("eDP-1", at(WidgetKind::Battery, 9, 1)),
    ];
    let text = encode(&entries);
    assert!(
        text.contains(r#"{"output": "DP-1", "kind": "up_next", "column": 7, "row": 1}"#),
        "{text}"
    );
    assert!(
        text.contains(r#"{"kind": "world_clock", "column": 2, "row": 2}"#),
        "{text}"
    );
    assert_eq!(decode(&text), entries);
    assert_eq!(encode(&[]), "[]\n");
    assert_eq!(decode("[]"), Vec::new());
}

#[test]
fn what_cannot_be_read_is_skipped() {
    let cases = [
        ("", Vec::new()),
        ("not json", Vec::new()),
        (
            r#"[{"kind":"weather","column":1,"row":1},{"row":2,"column":3,"kind":"battery"}]"#,
            vec![anywhere(at(WidgetKind::Battery, 3, 2))],
        ),
        (
            r#"[{"kind":"calendar","column":-1,"row":0},{"kind":"calendar","column":1}]"#,
            Vec::new(),
        ),
        (
            r#"[{"output":"DP-2","row":0,"column":4,"kind":"calendar"}]"#,
            vec![on("DP-2", at(WidgetKind::Calendar, 4, 0))],
        ),
    ];
    for (text, want) in cases {
        assert_eq!(decode(text), want, "{text}");
    }
}

#[test]
fn a_name_the_format_cannot_spell_is_any_output() {
    let cases = [
        (Some("DP-1"), OutputKey::Named("DP-1".to_owned())),
        (None, OutputKey::Any),
        (Some(""), OutputKey::Any),
        (Some("a\"b"), OutputKey::Any),
        (Some("a,b"), OutputKey::Any),
        (Some("a:b"), OutputKey::Any),
    ];
    for (name, want) in cases {
        assert_eq!(OutputKey::of(name), want, "{name:?}");
    }
}

#[test]
fn each_output_reads_its_own_places_or_the_unnamed_ones() {
    let entries = vec![
        anywhere(at(WidgetKind::Calendar, 5, 0)),
        on("DP-1", at(WidgetKind::Calendar, 1, 1)),
    ];
    let dp1 = OutputKey::of(Some("DP-1"));
    let hdmi = OutputKey::of(Some("HDMI-A-1"));
    assert_eq!(
        for_output(&entries, &dp1),
        vec![at(WidgetKind::Calendar, 1, 1)]
    );
    assert_eq!(
        for_output(&entries, &hdmi),
        vec![at(WidgetKind::Calendar, 5, 0)],
        "an output with none of its own reads the unnamed ones"
    );
    let saved = with_output(entries, &hdmi, &[at(WidgetKind::Battery, 0, 0)]);
    assert_eq!(
        for_output(&saved, &hdmi),
        vec![at(WidgetKind::Battery, 0, 0)]
    );
    assert_eq!(
        for_output(&saved, &dp1),
        vec![at(WidgetKind::Calendar, 1, 1)],
        "saving one output keeps the others"
    );
    let replaced = with_output(saved, &dp1, &[at(WidgetKind::Calendar, 2, 2)]);
    assert_eq!(replaced.len(), 3, "{replaced:?}");
}

#[test]
fn the_file_is_written_and_read_back() {
    let dir = std::env::temp_dir().join(format!("sill-dw-saved-{}", std::process::id()));
    let file = PlacementFile(dir.join("state").join("desktop-widgets.json"));
    let dp1 = OutputKey::of(Some("DP-1"));
    let dp2 = OutputKey::of(Some("DP-2"));
    assert_eq!(file.load(&dp1), Vec::new(), "missing is none");
    let one = vec![at(WidgetKind::Battery, 4, 0)];
    let two = vec![at(WidgetKind::Battery, 0, 3)];
    file.save(&dp1, &one)
        .unwrap_or_else(|e| panic!("saving: {e}"));
    file.save(&dp2, &two)
        .unwrap_or_else(|e| panic!("saving: {e}"));
    assert_eq!(file.load(&dp1), one);
    assert_eq!(file.load(&dp2), two);
    let _ = std::fs::remove_dir_all(dir);
}
