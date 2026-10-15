//! The Spaces bar's per-output rules: where it is up, where its cards lie and which targets that
//! sends, when a picture is asked for (of every Space), and which tile a drag lights.

use casement_protocols::vocab::{
    Emphasis, LeaseState, Modality, PlacementMark, Responsiveness, SensitivityLevel, Shown,
    SpaceKind, WindowRole, WindowStateKind,
};
use ds_core::vocab::DropState;
use porter_core::SpaceId;

use super::{
    BarDrops, BarLayout, BarLayouts, BarOutputs, BarPictures, BarShow, DesktopCommand, DesktopView,
    Outbox, SpacePicture, SpacePlaceView, SpaceView, ThumbAsks, ThumbRequest, WindowView, add_of,
    bar_space_ids, outgoing, target_commands, thumb_size, thumbs_wanted, tiles_of,
};

fn id(text: &str) -> SpaceId {
    SpaceId::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_bar_is_drawn_only_where_mission_control_is_up() {
    use BarShow::{Hide, Show};
    let none = BarOutputs::default();
    let split = BarOutputs::default()
        .with("DP-1", Shown::Shown)
        .with("DP-2", Shown::Hidden);
    let cases = [
        (&none, Show, Some("DP-1"), Show),
        (&none, Show, None, Show),
        (&none, Hide, Some("DP-1"), Hide),
        (&split, Show, Some("DP-1"), Show),
        (&split, Show, Some("DP-2"), Hide),
        (&split, Show, Some("DP-3"), Hide),
        (&split, Show, None, Hide),
        (&split, Hide, Some("DP-1"), Hide),
    ];
    for (outputs, overall, output, want) in cases {
        assert_eq!(
            outputs.bar_on(overall, output),
            want,
            "{output:?} {overall:?}"
        );
    }
}

#[test]
fn the_cards_lie_in_a_row_centred_in_the_bar_on_the_surfaces_place_in_the_desktop() {
    let spaces = [id("ws-1"), id("ws-2")];
    // A 1000 wide bar at the top of a surface that starts at x 1920: two tiles of 168 and a gap
    // of 16 make a row of 352, which starts 324 in.
    let tiles = tiles_of((1920, 0), (0, 0, 1000, 150), &spaces);
    let rects: Vec<_> = tiles.iter().map(|tile| tile.rect).collect();
    assert_eq!(
        rects,
        [
            (1920 + 324 + 4, 12, 160, 100),
            (1920 + 324 + 184 + 4, 12, 160, 100)
        ]
    );
    assert_eq!(tiles[1].space, id("ws-2"));
    assert!(tiles_of((0, 0), (0, 0, 1000, 150), &[]).is_empty());
}

#[test]
fn the_plus_lies_at_the_bars_right_edge_inside_its_padding() {
    assert_eq!(add_of((0, 0), (0, 0, 1000, 150)), (952, 8, 28, 28));
    assert_eq!(
        add_of((1920, 24), (0, 10, 1000, 150)),
        (1920 + 952, 42, 28, 28)
    );
}

#[test]
fn a_card_is_asked_for_at_the_outputs_scale() {
    assert_eq!(thumb_size(120), (160, 100));
    assert_eq!(thumb_size(240), (320, 200));
    assert_eq!(thumb_size(150), (200, 125));
}

fn layouts(output: &str, spaces: &[SpaceId]) -> BarLayouts {
    let tiles = tiles_of((0, 0), (0, 0, 1000, 150), spaces);
    BarLayouts(
        [(
            output.to_owned(),
            BarLayout {
                tiles,
                thumb: (320, 200),
                add: Some(add_of((0, 0), (0, 0, 1000, 150))),
            },
        )]
        .into(),
    )
}

fn new_target(output: &str, rect: Option<(i32, i32, u32, u32)>) -> DesktopCommand {
    DesktopCommand::SetNewSpaceTarget {
        output: output.to_owned(),
        rect,
    }
}

#[test]
fn targets_are_sent_when_cards_move_and_cleared_when_the_bar_goes() {
    let a = layouts("DP-1", &[id("ws-1")]);
    let b = layouts("DP-1", &[id("ws-1"), id("ws-2")]);
    let nothing = BarLayouts::default();
    assert!(target_commands(&a, &a).is_empty());
    let set = target_commands(&nothing, &a);
    assert_eq!(set.len(), 2);
    assert!(matches!(
        &set[0],
        DesktopCommand::SetSpaceTargets { output, targets }
            if output == "DP-1" && targets.len() == 1
    ));
    assert_eq!(set[1], new_target("DP-1", Some((952, 8, 28, 28))));
    let moved = target_commands(&a, &b);
    assert_eq!(moved.len(), 1, "the plus did not move: {moved:?}");
    assert!(matches!(
        &moved[0],
        DesktopCommand::SetSpaceTargets { targets, .. } if targets.len() == 2
    ));
    assert_eq!(
        target_commands(&b, &nothing),
        [
            DesktopCommand::SetSpaceTargets {
                output: "DP-1".to_owned(),
                targets: Vec::new()
            },
            new_target("DP-1", None),
        ]
    );
}

#[test]
fn the_plus_alone_is_sent_when_only_it_moves() {
    let a = layouts("DP-1", &[id("ws-1")]);
    let mut b = a.clone();
    if let Some(layout) = b.0.get_mut("DP-1") {
        layout.add = Some((900, 8, 28, 28));
    }
    assert_eq!(
        target_commands(&a, &b),
        [new_target("DP-1", Some((900, 8, 28, 28)))]
    );
}

fn window(name: &str, on: &str, x: i32) -> WindowView {
    WindowView {
        identifier: name.to_owned(),
        app: "app".to_owned(),
        title: String::new(),
        state: WindowStateKind::Floating,
        space: SpacePlaceView::On(id(on)),
        output: String::new(),
        emphasis: Emphasis::Inactive,
        placement: PlacementMark::Free,
        role: WindowRole::Document,
        modality: Modality::None,
        geometry: Some((x, 0, 400, 300)),
        responsiveness: Responsiveness::Responding,
        lease: LeaseState::Free,
        sensitivity: SensitivityLevel::Normal,
    }
}

fn view(shown: &str, x: i32) -> DesktopView {
    let space = |name: &str| SpaceView {
        id: id(name),
        kind: SpaceKind::Desktop,
        output: "DP-1".to_owned(),
        position: 0,
        shown: if name == shown {
            Shown::Shown
        } else {
            Shown::Hidden
        },
    };
    DesktopView {
        spaces: vec![space("ws-1"), space("ws-2")],
        windows: vec![window("w1", "ws-1", x), window("w2", "ws-2", 0)],
        ..DesktopView::default()
    }
}

fn want(space: &str) -> ThumbRequest {
    ThumbRequest {
        output: "DP-1".to_owned(),
        space: id(space),
        size: (320, 200),
    }
}

#[test]
fn every_space_is_asked_for_once_and_again_only_after_its_answer_and_a_change() {
    let layout = layouts("DP-1", &[id("ws-1"), id("ws-2")]);
    let png = SpacePicture::Png(vec![1, 2, 3]);
    let first = view("ws-1", 0);
    let none = BarPictures::default();

    // The bar comes up: the Space on show and the hidden one are both asked for.
    let (asks, out) = thumbs_wanted(&ThumbAsks::default(), &layout, &first, &none);
    assert_eq!(out, [want("ws-1"), want("ws-2")]);
    // Nothing changed and nothing came back: not asked again.
    let (asks, out) = thumbs_wanted(&asks, &layout, &first, &none);
    assert!(out.is_empty());
    // A window moved before the answer: still waits for the answer.
    let (asks, out) = thumbs_wanted(&asks, &layout, &view("ws-1", 50), &none);
    assert!(out.is_empty());
    // The answer came and the window had moved: that Space is asked again, the other is not.
    let answered =
        BarPictures::default()
            .with("DP-1", id("ws-1"), &png)
            .with("DP-1", id("ws-2"), &png);
    let moved = view("ws-1", 50);
    let (asks, out) = thumbs_wanted(&asks, &layout, &moved, &answered);
    assert_eq!(out, [want("ws-1")]);
    // The answer came and nothing changed since the ask: the pictures stand.
    let again = answered.with("DP-1", id("ws-1"), &png);
    let (asks, out) = thumbs_wanted(&asks, &layout, &moved, &again);
    assert!(out.is_empty());
    // Switching Space changes no window, so the sessions the compositor keeps are not asked again.
    let (_, out) = thumbs_wanted(&asks, &layout, &view("ws-2", 50), &again);
    assert!(out.is_empty());
}

#[test]
fn a_space_that_leaves_the_output_forgets_its_ask() {
    let both = layouts("DP-1", &[id("ws-1"), id("ws-2")]);
    let seen = view("ws-1", 0);
    let none = BarPictures::default();
    let (asks, _) = thumbs_wanted(&ThumbAsks::default(), &both, &seen, &none);
    let mut fewer = seen.clone();
    fewer.spaces.truncate(1);
    let (asks, out) = thumbs_wanted(&asks, &both, &fewer, &none);
    assert!(out.is_empty());
    let (_, out) = thumbs_wanted(&asks, &both, &seen, &none);
    assert_eq!(out, [want("ws-2")]);
}

#[test]
fn a_bar_that_went_away_asks_afresh_when_it_comes_back() {
    let layout = layouts("DP-1", &[id("ws-1")]);
    let seen = view("ws-1", 0);
    let none = BarPictures::default();
    let (asks, _) = thumbs_wanted(&ThumbAsks::default(), &layout, &seen, &none);
    let (asks, out) = thumbs_wanted(&asks, &BarLayouts::default(), &seen, &none);
    assert!(out.is_empty());
    let (_, out) = thumbs_wanted(&asks, &layout, &seen, &none);
    assert_eq!(out, [want("ws-1"), want("ws-2")]);
}

#[test]
fn a_failed_answer_keeps_the_last_picture_and_is_not_retried_unchanged() {
    let png = SpacePicture::Png(vec![9]);
    let kept = BarPictures::default().with("DP-1", id("ws-1"), &png).with(
        "DP-1",
        id("ws-1"),
        &SpacePicture::Failed,
    );
    assert_eq!(kept.seq_of("DP-1", &id("ws-1")), 2);
    assert_eq!(
        kept.png_of("DP-1", &id("ws-1")).as_deref(),
        Some(&[9_u8][..])
    );
    assert_eq!(kept.png_of("DP-1", &id("ws-2")), None);

    let layout = layouts("DP-1", &[id("ws-1")]);
    let seen = view("ws-1", 0);
    let (asks, _) = thumbs_wanted(
        &ThumbAsks::default(),
        &layout,
        &seen,
        &BarPictures::default(),
    );
    let failed = BarPictures::default().with("DP-1", id("ws-1"), &SpacePicture::Failed);
    let (_, out) = thumbs_wanted(&asks, &layout, &seen, &failed);
    assert!(out.is_empty());
}

#[test]
fn the_tile_under_the_dragged_window_is_lit() {
    let drops = BarDrops::default().with("DP-1", Some(id("ws-2")));
    let cases = [
        (Some("DP-1"), "ws-2", DropState::Target),
        (Some("DP-1"), "ws-1", DropState::Idle),
        (Some("DP-2"), "ws-2", DropState::Idle),
        (None, "ws-2", DropState::Idle),
    ];
    for (output, space, want) in cases {
        assert_eq!(
            drops.state_of(output, &id(space)),
            want,
            "{output:?} {space}"
        );
    }
    let over = drops.with("DP-1", None);
    assert_eq!(over.state_of(Some("DP-1"), &id("ws-2")), DropState::Idle);
}

#[test]
fn the_plus_is_lit_while_the_dragged_window_is_over_it_and_never_with_a_tile() {
    let cases = [
        (Some("DP-1"), DropState::Target),
        (Some("DP-2"), DropState::Idle),
        (None, DropState::Idle),
    ];
    let over = BarDrops::default().with_new("DP-1", Shown::Shown);
    for (output, want) in cases {
        assert_eq!(over.new_state_of(output), want, "{output:?}");
    }
    let left = over.clone().with_new("DP-1", Shown::Hidden);
    assert_eq!(left.new_state_of(Some("DP-1")), DropState::Idle);
    let onto_a_tile = over.with("DP-1", Some(id("ws-1")));
    assert_eq!(onto_a_tile.new_state_of(Some("DP-1")), DropState::Idle);
    assert_eq!(
        onto_a_tile.state_of(Some("DP-1"), &id("ws-1")),
        DropState::Target
    );
}

#[test]
fn the_ids_are_the_outputs_spaces_in_order() {
    let mut seen = view("ws-1", 0);
    seen.spaces[0].position = 1;
    seen.spaces[1].position = 0;
    assert_eq!(bar_space_ids(&seen, Some("DP-1")), [id("ws-2"), id("ws-1")]);
    assert!(bar_space_ids(&seen, Some("DP-2")).is_empty());
}

#[test]
fn a_laid_out_bar_gets_its_targets_and_a_request_per_space_and_nothing_again_until_answered() {
    let layout = layouts("DP-1", &[id("ws-1")]);
    let seen = view("ws-1", 0);
    let none = BarPictures::default();
    let (outbox, commands) = outgoing(Outbox::default(), &layout, &seen, &none);
    assert!(
        matches!(
            commands.as_slice(),
            [
                DesktopCommand::SetSpaceTargets { output, targets },
                DesktopCommand::SetNewSpaceTarget { rect: Some(_), .. },
                DesktopCommand::CaptureSpace { size: (320, 200), space: first, .. },
                DesktopCommand::CaptureSpace { space: second, .. },
            ] if output == "DP-1" && targets.len() == 1 && first == &id("ws-1")
                && second == &id("ws-2")
        ),
        "{commands:?}"
    );
    let (outbox, again) = outgoing(outbox, &layout, &seen, &none);
    assert!(again.is_empty(), "{again:?}");
    // The bar goes: the targets are cleared and nothing is asked for.
    let (_, gone) = outgoing(outbox, &BarLayouts::default(), &seen, &none);
    assert_eq!(
        gone,
        [
            DesktopCommand::SetSpaceTargets {
                output: "DP-1".to_owned(),
                targets: Vec::new()
            },
            new_target("DP-1", None),
        ]
    );
}
