//! The Spaces bar's pure rules: when it is up, how the Spaces are named and marked per output,
//! and what a click sends.

use casement_protocols::vocab::{
    Emphasis, LeaseState, Modality, OverviewMode, OverviewState, PlacementMark, Responsiveness,
    SensitivityLevel, Shown, SpaceKind, Toggle, WindowRole, WindowStateKind,
};
use porter_core::SpaceId;

use super::{
    BarPick, BarShow, DesktopCommand, DesktopView, SpacePlaceView, SpaceView, WindowView, bar_show,
    bar_spaces, commands_of,
};
use crate::apps::AppIndex;

fn id(text: &str) -> SpaceId {
    SpaceId::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

fn space(name: &str, kind: SpaceKind, output: &str, position: u32, shown: Shown) -> SpaceView {
    SpaceView {
        id: id(name),
        kind,
        output: output.to_owned(),
        position,
        shown,
    }
}

fn window(app: &str, on: &str) -> WindowView {
    WindowView {
        identifier: format!("w-{app}"),
        app: app.to_owned(),
        title: String::new(),
        state: WindowStateKind::Floating,
        space: SpacePlaceView::On(id(on)),
        output: String::new(),
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

fn view() -> DesktopView {
    DesktopView {
        spaces: vec![
            space("ws-3", SpaceKind::FullScreen, "DP-1", 2, Shown::Hidden),
            space("ws-1", SpaceKind::Desktop, "DP-1", 0, Shown::Shown),
            space("ws-2", SpaceKind::Desktop, "DP-1", 1, Shown::Hidden),
            space("ws-4", SpaceKind::SplitView, "DP-1", 3, Shown::Hidden),
            space("ws-5", SpaceKind::Desktop, "DP-2", 0, Shown::Shown),
        ],
        windows: vec![
            window("safari", "ws-3"),
            window("mail", "ws-4"),
            window("notes", "ws-4"),
        ],
        ..DesktopView::default()
    }
}

#[test]
fn only_mission_control_that_is_opening_or_open_has_a_bar() {
    use OverviewMode::{AppExpose, Desktop, MissionControl};
    use OverviewState::{Closed, Entering, Leaving, Open, Tracking};
    let cases = [
        (MissionControl, Closed, BarShow::Hide),
        (MissionControl, Entering, BarShow::Show),
        (MissionControl, Open, BarShow::Show),
        (MissionControl, Tracking, BarShow::Show),
        (MissionControl, Leaving, BarShow::Hide),
        (AppExpose, Open, BarShow::Hide),
        (Desktop, Entering, BarShow::Hide),
    ];
    for (mode, state, want) in cases {
        assert_eq!(bar_show(mode, state), want, "{mode:?} {state:?}");
    }
}

#[test]
fn each_output_lists_its_spaces_in_order_named_as_the_mac_names_them() {
    let apps = AppIndex::default();
    let first: Vec<(String, bool)> = bar_spaces(&view(), Some("DP-1"), &apps)
        .into_iter()
        .map(|s| (s.label, s.current))
        .collect();
    // Without an app index the apps are named by their keys.
    let want: Vec<(String, bool)> = [
        ("Desktop 1", true),
        ("Desktop 2", false),
        ("safari", false),
        ("mail & notes", false),
    ]
    .into_iter()
    .map(|(label, current)| (label.to_owned(), current))
    .collect();
    assert_eq!(first, want);
    let second = bar_spaces(&view(), Some("DP-2"), &apps);
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].label, "Desktop 1");
    assert!(second[0].current);
}

#[test]
fn a_space_with_no_output_is_on_every_output_and_an_unnamed_output_lists_only_those() {
    let mut view = view();
    view.spaces = vec![space("ws-1", SpaceKind::Desktop, "", 0, Shown::Shown)];
    let apps = AppIndex::default();
    assert_eq!(bar_spaces(&view, Some("DP-9"), &apps).len(), 1);
    assert_eq!(bar_spaces(&view, None, &apps).len(), 1);
    assert!(bar_spaces(&DesktopView::default(), Some("DP-1"), &apps).is_empty());
}

#[test]
fn a_full_screen_space_with_no_known_window_is_named_by_its_kind() {
    let mut view = view();
    view.windows.clear();
    let labels: Vec<String> = bar_spaces(&view, Some("DP-1"), &AppIndex::default())
        .into_iter()
        .map(|s| s.label)
        .collect();
    assert_eq!(labels[2..], ["Full Screen", "Split View"]);
}

#[test]
fn a_click_on_a_space_switches_and_leaves_mission_control_and_the_plus_adds() {
    assert_eq!(
        commands_of(&BarPick::Switch(id("ws-2"))),
        vec![
            DesktopCommand::SwitchSpace(id("ws-2")),
            DesktopCommand::Overview {
                mode: OverviewMode::MissionControl,
                toggle: Toggle::Off,
            },
        ]
    );
    assert_eq!(
        commands_of(&BarPick::Add("DP-1".to_owned())),
        vec![DesktopCommand::CreateSpace("DP-1".to_owned())]
    );
}
