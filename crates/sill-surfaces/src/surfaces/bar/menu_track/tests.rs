//! design/13 §13.4's table and §13.8 tests 1-5 as input sequences.

use shell_host::Point;
use sill_services::Stamp;

use super::{
    Anim, Branch, Guard, Held, ItemPath, MenuId, MenuIn, MenuKey, MenuOut, MenuParams, MenuTrack,
    Pick, Shown, Side, Sub, Target, inside, step,
};

/// One scripted input sequence: `(ms, input)` pairs.
type Script = Vec<(u64, MenuIn)>;

const A: MenuId = MenuId(1);
const B: MenuId = MenuId(2);

fn p() -> MenuParams {
    MenuParams::new(&sill_settings::MenusSettings::default())
}

fn path(i: u16) -> ItemPath {
    ItemPath(vec![i])
}

fn item(i: u16) -> Target {
    Target::Item {
        path: path(i),
        pick: Pick::Enabled,
        branch: Branch::Leaf,
    }
}

fn parent(i: u16) -> Target {
    Target::Item {
        path: path(i),
        pick: Pick::Enabled,
        branch: Branch::Submenu,
    }
}

fn inert(i: u16) -> Target {
    Target::Item {
        path: path(i),
        pick: Pick::Inert,
        branch: Branch::Leaf,
    }
}

fn pt(x: i32, y: i32) -> Point {
    Point { x, y }
}

/// Feed `(ms, input)`; the final state and every output, in order.
fn run(inputs: Vec<(u64, MenuIn)>) -> (MenuTrack, Vec<MenuOut>) {
    let params = p();
    inputs.into_iter().fold(
        (MenuTrack::Closed, Vec::new()),
        |(track, mut all), (ms, input)| {
            let (track, out) = step(track, input, Stamp(ms), &params);
            all.extend(out);
            (track, all)
        },
    )
}

fn closed(track: &MenuTrack) -> bool {
    *track == MenuTrack::Closed
}

fn held(track: &MenuTrack) -> Option<Held> {
    match track {
        MenuTrack::Tracking { held, .. } => Some(*held),
        MenuTrack::Closed => None,
    }
}

#[test]
fn open_on_press_click_mode_and_toggle() {
    // §13.8 test 1.
    let (t, out) = run(vec![(0, MenuIn::PressTitle(A))]);
    assert_eq!(out, vec![MenuOut::Open(A, Anim::MenuPop)]);
    assert_eq!(held(&t), Some(Held::Held));
    let (t, _) = run(vec![
        (0, MenuIn::PressTitle(A)),
        (90, MenuIn::Release(Target::Title(A))),
    ]);
    assert_eq!(
        held(&t),
        Some(Held::Released),
        "release on the title leaves it open"
    );
    let (t, out) = run(vec![
        (0, MenuIn::PressTitle(A)),
        (90, MenuIn::Release(Target::Title(A))),
        (500, MenuIn::PressTitle(A)),
    ]);
    assert!(closed(&t));
    assert_eq!(out.last(), Some(&MenuOut::Close(Anim::Fade)));
}

#[test]
fn press_drag_release_picks_or_closes() {
    // §13.8 test 2 and the table's Release rows.
    let press_move = |target: Target| {
        vec![
            (0, MenuIn::PressTitle(A)),
            (40, MenuIn::Move(pt(20, 60), target.clone())),
            (80, MenuIn::Release(target)),
        ]
    };
    let cases: Vec<(&str, Script, Vec<MenuOut>)> = vec![
        (
            "release on an enabled item picks it once",
            press_move(item(3)),
            vec![MenuOut::Close(Anim::Fade), MenuOut::Pick(path(3))],
        ),
        (
            "release on a separator closes without picking",
            press_move(inert(2)),
            vec![MenuOut::Close(Anim::Fade)],
        ),
        (
            "release outside after entering closes",
            vec![
                (0, MenuIn::PressTitle(A)),
                (40, MenuIn::Move(pt(20, 60), item(1))),
                (80, MenuIn::Release(Target::Outside)),
            ],
            vec![MenuOut::Close(Anim::Fade)],
        ),
        (
            "release outside before entering closes",
            vec![
                (0, MenuIn::PressTitle(A)),
                (80, MenuIn::Release(Target::Outside)),
            ],
            vec![MenuOut::Close(Anim::Fade)],
        ),
        (
            "in click mode a later release on an item picks it",
            vec![
                (0, MenuIn::PressTitle(A)),
                (60, MenuIn::Release(Target::Title(A))),
                (400, MenuIn::Release(item(4))),
            ],
            vec![MenuOut::Close(Anim::Fade), MenuOut::Pick(path(4))],
        ),
        (
            "an outside press closes",
            vec![(0, MenuIn::PressTitle(A)), (60, MenuIn::OutsidePress)],
            vec![MenuOut::Close(Anim::Fade)],
        ),
    ];
    for (name, inputs, want_tail) in cases {
        let (t, out) = run(inputs);
        assert!(closed(&t), "{name}");
        let tail: Vec<MenuOut> = out
            .iter()
            .filter(|o| matches!(o, MenuOut::Close(_) | MenuOut::Pick(_)))
            .cloned()
            .collect();
        assert_eq!(tail, want_tail, "{name}");
    }
}

#[test]
fn a_press_on_a_title_the_pointer_switched_to_keeps_its_menu() {
    // F991: with A open, the pointer enters B (the hover switch shows B), then presses B. That
    // press is the click that opens B; only a press on a title already pressed closes.
    let switched = vec![
        (0, MenuIn::PressTitle(A)),
        (60, MenuIn::Release(Target::Title(A))),
        (300, MenuIn::Move(pt(120, 10), Target::Title(B))),
    ];
    let shown = |t: &MenuTrack| match t {
        MenuTrack::Tracking { menu, shown, .. } => Some((*menu, *shown)),
        MenuTrack::Closed => None,
    };
    let cases: Vec<(&str, Script, Option<(MenuId, Shown)>, Option<Held>)> = vec![
        (
            "switched to by hover",
            switched.clone(),
            Some((B, Shown::Hovered)),
            Some(Held::Released),
        ),
        (
            "a press on the switched title keeps it, held",
            [switched.clone(), vec![(400, MenuIn::PressTitle(B))]].concat(),
            Some((B, Shown::Pressed)),
            Some(Held::Held),
        ),
        (
            "released there: click mode",
            [
                switched.clone(),
                vec![
                    (400, MenuIn::PressTitle(B)),
                    (460, MenuIn::Release(Target::Title(B))),
                ],
            ]
            .concat(),
            Some((B, Shown::Pressed)),
            Some(Held::Released),
        ),
        (
            "a second press closes it",
            [
                switched.clone(),
                vec![
                    (400, MenuIn::PressTitle(B)),
                    (460, MenuIn::Release(Target::Title(B))),
                    (900, MenuIn::PressTitle(B)),
                ],
            ]
            .concat(),
            None,
            None,
        ),
        (
            "moving inside B's menu keeps it hover-shown",
            [
                switched.clone(),
                vec![(400, MenuIn::Move(pt(130, 60), item(1)))],
            ]
            .concat(),
            Some((B, Shown::Hovered)),
            Some(Held::Released),
        ),
        (
            "a press-drag onto B released on its title makes it B's own",
            vec![
                (0, MenuIn::PressTitle(A)),
                (100, MenuIn::Move(pt(120, 10), Target::Title(B))),
                (200, MenuIn::Release(Target::Title(B))),
            ],
            Some((B, Shown::Pressed)),
            Some(Held::Released),
        ),
        (
            "a press on A's own title still closes A",
            vec![
                (0, MenuIn::PressTitle(A)),
                (60, MenuIn::Release(Target::Title(A))),
                (500, MenuIn::PressTitle(A)),
            ],
            None,
            None,
        ),
    ];
    for (name, inputs, want, want_held) in cases {
        let (t, out) = run(inputs);
        assert_eq!(shown(&t), want, "{name}");
        assert_eq!(held(&t), want_held, "{name}");
        assert!(
            !out.iter()
                .any(|o| matches!(o, MenuOut::Open(m, _) if *m == B)),
            "{name}: B is swapped in, never opened afresh"
        );
    }
}

#[test]
fn hover_switch_swaps_menus_in_one_step_without_animation() {
    // §13.8 test 3: one Switch, never a Close then an Open.
    let (t, out) = run(vec![
        (0, MenuIn::PressTitle(A)),
        (60, MenuIn::Release(Target::Title(A))),
        (300, MenuIn::Move(pt(120, 10), Target::Title(B))),
    ]);
    assert_eq!(
        out,
        vec![MenuOut::Open(A, Anim::MenuPop), MenuOut::Switch(B)]
    );
    assert!(matches!(
        t,
        MenuTrack::Tracking {
            menu: B,
            held: Held::Released,
            ..
        }
    ));
}

#[test]
fn a_submenu_opens_after_the_delay_or_at_once_on_right_arrow() {
    // §13.8 test 4.
    let rest = vec![
        (0, MenuIn::PressTitle(A)),
        (60, MenuIn::Release(Target::Title(A))),
        (100, MenuIn::Move(pt(20, 60), parent(2))),
    ];
    let (_, out) = run(rest.clone());
    assert_eq!(out.last(), Some(&MenuOut::RequestTick(Stamp(300))));
    let (t, out) = run([rest.clone(), vec![(290, MenuIn::Tick)]].concat());
    assert!(!out.contains(&MenuOut::OpenSub(path(2))), "not at 190 ms");
    assert!(matches!(
        t,
        MenuTrack::Tracking {
            sub: Sub::Pending { .. },
            ..
        }
    ));
    let (_, out) = run([rest.clone(), vec![(300, MenuIn::Tick)]].concat());
    assert_eq!(out.last(), Some(&MenuOut::OpenSub(path(2))));
    let (_, out) = run([rest, vec![(150, MenuIn::Key(MenuKey::Right))]].concat());
    assert_eq!(out.last(), Some(&MenuOut::OpenSub(path(2))));
}

#[test]
fn the_safe_triangle_keeps_the_submenu_and_times_out() {
    // §13.8 test 5: a diagonal path toward the submenu crossing other parent items keeps it.
    let open = vec![
        (0, MenuIn::PressTitle(A)),
        (60, MenuIn::Release(Target::Title(A))),
        (100, MenuIn::Move(pt(100, 100), parent(2))),
        (300, MenuIn::Tick),
        (
            310,
            MenuIn::SubPlaced {
                top: pt(200, 95),
                bottom: pt(200, 300),
            },
        ),
    ];
    let (t, out) = run([
        open.clone(),
        vec![
            (320, MenuIn::Move(pt(130, 130), item(3))),
            (330, MenuIn::Move(pt(160, 170), item(4))),
        ],
    ]
    .concat());
    assert!(!out.contains(&MenuOut::CloseSub), "{out:?}");
    assert!(
        matches!(&t, MenuTrack::Tracking { hot: Some(h), sub: Sub::Open { .. }, .. } if *h == path(2))
    );
    // Resting inside the triangle for 300 ms selects what is under the pointer.
    let (t, out) = run([
        open.clone(),
        vec![
            (320, MenuIn::Move(pt(130, 130), item(3))),
            (620, MenuIn::Tick),
        ],
    ]
    .concat());
    assert!(out.contains(&MenuOut::CloseSub), "{out:?}");
    assert!(matches!(&t, MenuTrack::Tracking { hot: Some(h), .. } if *h == path(3)));
    // Leaving the triangle onto another parent item closes it at once.
    let (_, out) = run([open, vec![(320, MenuIn::Move(pt(100, 20), item(0)))]].concat());
    assert!(out.contains(&MenuOut::CloseSub), "{out:?}");
}

#[test]
fn keys_close_a_level_cross_menus_and_pick() {
    let open = vec![
        (0, MenuIn::PressTitle(A)),
        (60, MenuIn::Release(Target::Title(A))),
        (100, MenuIn::Move(pt(20, 60), item(1))),
    ];
    let (t, out) = run([open.clone(), vec![(200, MenuIn::Key(MenuKey::Escape))]].concat());
    assert!(closed(&t));
    assert_eq!(out.last(), Some(&MenuOut::Close(Anim::Fade)));
    let (_, out) = run([open.clone(), vec![(200, MenuIn::Key(MenuKey::Left))]].concat());
    assert_eq!(out.last(), Some(&MenuOut::Adjacent(Side::Left)));
    let (t, out) = run([open, vec![(200, MenuIn::Key(MenuKey::Enter))]].concat());
    assert!(closed(&t));
    assert_eq!(
        out[out.len() - 2..],
        [MenuOut::Close(Anim::Fade), MenuOut::Pick(path(1))]
    );
}

#[test]
fn escape_with_a_submenu_open_closes_only_the_submenu() {
    let (t, out) = run(vec![
        (0, MenuIn::PressTitle(A)),
        (60, MenuIn::Release(Target::Title(A))),
        (100, MenuIn::Move(pt(20, 60), parent(2))),
        (150, MenuIn::Key(MenuKey::Right)),
        (200, MenuIn::Key(MenuKey::Escape)),
    ]);
    assert_eq!(out.last(), Some(&MenuOut::CloseSub));
    assert!(matches!(t, MenuTrack::Tracking { sub: Sub::None, .. }));
}

#[test]
fn the_triangle_test_is_a_sign_test_either_winding() {
    let (a, b, c) = (pt(0, 0), pt(100, -10), pt(100, 110));
    const CASES: &[((i32, i32), bool)] = &[
        ((50, 50), true),
        ((10, 0), true),
        ((100, 50), true),
        ((50, -20), false),
        ((120, 50), false),
        ((-1, 0), false),
    ];
    for &((x, y), want) in CASES {
        assert_eq!(inside(pt(x, y), a, b, c), want, "({x}, {y})");
        assert_eq!(inside(pt(x, y), a, c, b), want, "({x}, {y}) reversed");
    }
    let guard = Guard {
        from: pt(0, 50),
        top: pt(100, 0),
        bottom: pt(100, 100),
        still_since: Stamp(0),
    };
    assert!(
        super::guarded(&guard, pt(100, -3)),
        "the corners are inflated 4 px"
    );
    assert!(!super::guarded(&guard, pt(100, -5)));
}
