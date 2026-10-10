//! Which windows have a target, and the commands between two sets of targets.

use casement_protocols::vocab::{
    Emphasis, LeaseState, Modality, PlacementMark, Responsiveness, SensitivityLevel, WindowRole,
    WindowStateKind,
};

use super::target::{AppRects, MinimiseTargets, OutputRects, Rect};
use super::{DesktopCommand, DesktopView, SpacePlaceView, WindowView};

const A: Rect = (10, 20, 48, 48);
const B: Rect = (66, 20, 48, 48);

fn window(id: &str, app: &str, output: &str) -> WindowView {
    WindowView {
        identifier: id.to_owned(),
        app: app.to_owned(),
        title: String::new(),
        state: WindowStateKind::Floating,
        space: SpacePlaceView::Unknown,
        output: output.to_owned(),
        emphasis: Emphasis::Inactive,
        placement: PlacementMark::Free,
        role: WindowRole::Document,
        modality: Modality::None,
        geometry: None,
        responsiveness: Responsiveness::Responding,
        lease: LeaseState::Free,
        sensitivity: SensitivityLevel::Normal,
    }
}

fn targets(of: &[(&str, Rect)]) -> MinimiseTargets {
    MinimiseTargets(of.iter().map(|(id, r)| ((*id).to_owned(), *r)).collect())
}

fn set(id: &str, rect: Rect) -> DesktopCommand {
    DesktopCommand::SetMinimiseTarget {
        toplevel: id.to_owned(),
        rect,
    }
}

fn clear(id: &str) -> DesktopCommand {
    DesktopCommand::ClearMinimiseTarget(id.to_owned())
}

#[test]
fn a_window_goes_to_its_apps_icon_on_its_output() {
    let docks = |name: &str, apps: &[(&str, Rect)]| {
        (
            name.to_owned(),
            AppRects(apps.iter().map(|(a, r)| ((*a).to_owned(), *r)).collect()),
        )
    };
    let rects = OutputRects(
        [
            docks("DP-1", &[("term", A), ("files", B)]),
            docks("DP-2", &[("term", B)]),
        ]
        .into(),
    );
    let view = DesktopView {
        windows: vec![
            window("w1", "term", "DP-1"),
            window("w2", "term", "DP-2"),
            window("w3", "files", "DP-2"),
            window("w4", "term", ""),
            window("w5", "gone", "DP-1"),
        ],
        ..DesktopView::default()
    };
    assert_eq!(
        MinimiseTargets::of(&rects, &view),
        targets(&[("w1", A), ("w2", B)])
    );
}

#[test]
fn the_commands_between_two_sets_are_only_what_changed() {
    let cases: [(&str, MinimiseTargets, MinimiseTargets, Vec<DesktopCommand>); 7] = [
        ("nothing", targets(&[]), targets(&[]), vec![]),
        (
            "unchanged",
            targets(&[("w1", A)]),
            targets(&[("w1", A)]),
            vec![],
        ),
        (
            "new window",
            targets(&[]),
            targets(&[("w1", A)]),
            vec![set("w1", A)],
        ),
        (
            "icon moved",
            targets(&[("w1", A)]),
            targets(&[("w1", B)]),
            vec![set("w1", B)],
        ),
        (
            "window left",
            targets(&[("w1", A)]),
            targets(&[]),
            vec![clear("w1")],
        ),
        (
            "one moved, one new, one gone",
            targets(&[("w1", A), ("w2", A)]),
            targets(&[("w1", B), ("w3", A)]),
            vec![set("w1", B), set("w3", A), clear("w2")],
        ),
        (
            "untouched stays quiet",
            targets(&[("w1", A), ("w2", B)]),
            targets(&[("w1", A), ("w2", A)]),
            vec![set("w2", A)],
        ),
    ];
    for (name, old, new, want) in cases {
        assert_eq!(new.since(&old), want, "{name}");
    }
}
