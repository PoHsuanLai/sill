use sill_settings::WidgetKind;

use super::{PlacementFile, SavedOutput, decode, encode, kinds_of, places_of, with_output};
use crate::widgets::grid::OutputKey;
use crate::widgets::grid::{GridSlot, Placement};

fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
    Placement {
        kind,
        slot: GridSlot { column, row },
    }
}

fn named(output: &str, kinds: Option<Vec<WidgetKind>>, places: Vec<Placement>) -> SavedOutput {
    SavedOutput {
        output: OutputKey::of(Some(output)),
        kinds,
        places,
    }
}

#[test]
fn outputs_round_trip_through_the_file_format() {
    let outputs = vec![
        named(
            "DP-1",
            Some(vec![WidgetKind::Calendar, WidgetKind::UpNext]),
            vec![at(WidgetKind::Calendar, 9, 0), at(WidgetKind::UpNext, 7, 1)],
        ),
        named("HDMI-A-1", None, vec![at(WidgetKind::NowPlaying, 0, 4)]),
        SavedOutput {
            output: OutputKey::Unnamed,
            kinds: Some(Vec::new()),
            places: vec![at(WidgetKind::WorldClock, 2, 2)],
        },
    ];
    let text = encode(&outputs);
    assert!(text.contains(r#""kind": "up_next""#), "{text}");
    assert!(
        !text.contains(r#""widgets": null"#),
        "an output that follows the setting writes no list: {text}"
    );
    assert_eq!(decode(&text), outputs);
    assert_eq!(decode(&encode(&[])), Vec::new());
}

#[test]
fn what_cannot_be_read_is_skipped() {
    let cases = [
        ("", Vec::new()),
        ("not json", Vec::new()),
        ("[]", Vec::new()),
        (r#"{"outputs":3}"#, Vec::new()),
        (
            r#"{"outputs":[7,{"output":"DP-2","places":[{"kind":"weather","column":1,"row":1},{"row":2,"column":3,"kind":"battery"},{"kind":"calendar","column":-1,"row":0},{"kind":"calendar","column":1}]}]}"#,
            vec![named("DP-2", None, vec![at(WidgetKind::Battery, 3, 2)])],
        ),
        (
            r#"{"outputs":[{"widgets":["calendar","weather","battery"]}]}"#,
            vec![SavedOutput {
                output: OutputKey::Unnamed,
                kinds: Some(vec![WidgetKind::Calendar, WidgetKind::Battery]),
                places: Vec::new(),
            }],
        ),
    ];
    for (text, want) in cases {
        assert_eq!(decode(text), want, "{text}");
    }
}

#[test]
fn an_output_without_a_name_is_unnamed() {
    let cases = [
        (Some("DP-1"), OutputKey::Named("DP-1".to_owned())),
        (None, OutputKey::Unnamed),
        (Some(""), OutputKey::Unnamed),
    ];
    for (name, want) in cases {
        assert_eq!(OutputKey::of(name), want, "{name:?}");
    }
}

#[test]
fn each_output_reads_only_its_own_state() {
    let dp1 = OutputKey::of(Some("DP-1"));
    let hdmi = OutputKey::of(Some("HDMI-A-1"));
    let outputs = vec![
        SavedOutput {
            output: OutputKey::Unnamed,
            kinds: None,
            places: vec![at(WidgetKind::Calendar, 5, 0)],
        },
        named(
            "DP-1",
            Some(vec![WidgetKind::Calendar]),
            vec![at(WidgetKind::Calendar, 1, 1)],
        ),
    ];
    assert_eq!(
        places_of(&outputs, &dp1),
        vec![at(WidgetKind::Calendar, 1, 1)]
    );
    assert_eq!(kinds_of(&outputs, &dp1), Some(vec![WidgetKind::Calendar]));
    assert_eq!(places_of(&outputs, &hdmi), Vec::new());
    assert_eq!(kinds_of(&outputs, &hdmi), None);
    let saved = with_output(
        outputs,
        named("HDMI-A-1", None, vec![at(WidgetKind::Battery, 0, 0)]),
    );
    assert_eq!(
        places_of(&saved, &hdmi),
        vec![at(WidgetKind::Battery, 0, 0)]
    );
    assert_eq!(
        places_of(&saved, &dp1),
        vec![at(WidgetKind::Calendar, 1, 1)],
        "saving one output keeps the others"
    );
    let replaced = with_output(
        saved,
        named("DP-1", None, vec![at(WidgetKind::Calendar, 2, 2)]),
    );
    assert_eq!(replaced.len(), 3, "{replaced:?}");
}

#[test]
fn the_file_is_written_and_read_back() {
    let dir = std::env::temp_dir().join(format!("sill-dw-saved-{}", std::process::id()));
    let file = PlacementFile(dir.join("state").join("desktop-widgets.json"));
    let dp1 = OutputKey::of(Some("DP-1"));
    let dp2 = OutputKey::of(Some("DP-2"));
    assert_eq!(file.load(&dp1), Vec::new(), "missing is none");
    assert_eq!(file.kinds(&dp1), None);
    let one = vec![at(WidgetKind::Battery, 4, 0)];
    let two = vec![at(WidgetKind::Battery, 0, 3)];
    file.save(&dp1, &one)
        .unwrap_or_else(|e| panic!("saving: {e}"));
    file.save_layout(&dp2, &[WidgetKind::Battery], &two)
        .unwrap_or_else(|e| panic!("saving: {e}"));
    assert_eq!(file.load(&dp1), one);
    assert_eq!(file.load(&dp2), two);
    assert_eq!(file.kinds(&dp1), None, "a save of places makes no list");
    assert_eq!(file.kinds(&dp2), Some(vec![WidgetKind::Battery]));
    let moved = vec![at(WidgetKind::Battery, 1, 1)];
    file.save(&dp2, &moved)
        .unwrap_or_else(|e| panic!("saving: {e}"));
    assert_eq!(file.load(&dp2), moved);
    assert_eq!(
        file.kinds(&dp2),
        Some(vec![WidgetKind::Battery]),
        "a drop keeps the output's list"
    );
    let _ = std::fs::remove_dir_all(dir);
}
