//! The tracker's transitions (design/13 §13.4 table).

use shell_host::Point;
use sill_model::units::Stamp;

use super::triangle::guarded;
use super::types::{
    Anim, Branch, Entered, Guard, Held, ItemPath, MenuId, MenuIn, MenuKey, MenuOut, MenuParams,
    MenuTrack, Pick, Shown, Side, Sub, Target,
};

/// The open menu and how it came to show, which every transition that keeps it carries over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Current {
    menu: MenuId,
    shown: Shown,
}

/// `track` after `input` at `now`, and what the bar must do.
pub fn step(
    track: MenuTrack,
    input: MenuIn,
    now: Stamp,
    p: &MenuParams,
) -> (MenuTrack, Vec<MenuOut>) {
    let MenuTrack::Tracking {
        menu,
        shown,
        held,
        entered,
        hot,
        sub,
        pointer,
        under,
    } = track
    else {
        return match input {
            MenuIn::PressTitle(menu) => (
                opened(menu, Shown::Pressed, Held::Held),
                vec![MenuOut::Open(menu, Anim::MenuPop)],
            ),
            _ => (MenuTrack::Closed, Vec::new()),
        };
    };
    let closed = |mut out: Vec<MenuOut>| {
        out.insert(0, MenuOut::Close(Anim::Fade));
        (MenuTrack::Closed, out)
    };
    let current = Current { menu, shown };
    let tracking = |shown, held, entered, hot, sub, pointer, under| MenuTrack::Tracking {
        menu,
        shown,
        held,
        entered,
        hot,
        sub,
        pointer,
        under,
    };
    match input {
        MenuIn::PressTitle(other) if other == menu => match shown {
            Shown::Pressed => closed(Vec::new()),
            // The pointer switched to this menu on its way to the press: the press is the click
            // that opens it, held as a fresh press on the title is.
            Shown::Hovered => (
                tracking(
                    Shown::Pressed,
                    Held::Held,
                    entered,
                    hot,
                    sub,
                    pointer,
                    under,
                ),
                Vec::new(),
            ),
        },
        MenuIn::PressTitle(other) => (
            opened(other, Shown::Pressed, Held::Held),
            vec![MenuOut::Switch(other)],
        ),
        MenuIn::OutsidePress => closed(Vec::new()),
        MenuIn::Release(Target::Item {
            path,
            pick: Pick::Enabled,
            ..
        }) => closed(vec![MenuOut::Pick(path)]),
        // A release on the title, even after dragging onto it, makes the menu the person's own.
        MenuIn::Release(Target::Title(t)) if t == menu && held == Held::Held => (
            tracking(
                Shown::Pressed,
                Held::Released,
                entered,
                hot,
                sub,
                pointer,
                under,
            ),
            Vec::new(),
        ),
        MenuIn::Release(_) if held == Held::Held => closed(Vec::new()),
        MenuIn::Release(_) => (
            tracking(shown, held, entered, hot, sub, pointer, under),
            Vec::new(),
        ),
        MenuIn::Move(_, Target::Title(t)) if t != menu => {
            (opened(t, Shown::Hovered, held), vec![MenuOut::Switch(t)])
        }
        MenuIn::Move(at, target) => moved(current, held, entered, hot, sub, at, target, now, p),
        MenuIn::SubPlaced { top, bottom } => {
            let sub = match sub {
                Sub::Open { item, .. } => Sub::Open {
                    item,
                    guard: Some(Guard {
                        from: pointer,
                        top,
                        bottom,
                        still_since: now,
                    }),
                },
                other => other,
            };
            (
                tracking(shown, held, entered, hot, sub, pointer, under),
                Vec::new(),
            )
        }
        MenuIn::Tick => ticked(current, held, entered, hot, sub, pointer, under, now, p),
        MenuIn::Key(key) => keyed(current, held, entered, hot, sub, pointer, under, key),
    }
}

fn opened(menu: MenuId, shown: Shown, held: Held) -> MenuTrack {
    MenuTrack::Tracking {
        menu,
        shown,
        held,
        entered: Entered::NotYet,
        hot: None,
        sub: Sub::None,
        pointer: Point::default(),
        under: Target::Outside,
    }
}

/// A pointer sample inside the session: the safe triangle, then highlight and submenu timing.
#[allow(clippy::too_many_arguments)]
fn moved(
    current: Current,
    held: Held,
    entered: Entered,
    hot: Option<ItemPath>,
    sub: Sub,
    at: Point,
    target: Target,
    now: Stamp,
    p: &MenuParams,
) -> (MenuTrack, Vec<MenuOut>) {
    if let Sub::Open {
        item,
        guard: Some(g),
    } = &sub
        && guarded(g, at)
    {
        let guard = Guard {
            from: at,
            still_since: now,
            ..*g
        };
        let sub = Sub::Open {
            item: item.clone(),
            guard: Some(guard),
        };
        let track = MenuTrack::Tracking {
            menu: current.menu,
            shown: current.shown,
            held,
            entered: Entered::Entered,
            hot,
            sub,
            pointer: at,
            under: target,
        };
        return (
            track,
            vec![MenuOut::RequestTick(now.after(p.triangle_timeout_ms))],
        );
    }
    let before = hot.clone();
    let mut out = Vec::new();
    let entered = match target {
        Target::Item { .. } | Target::Menu => Entered::Entered,
        _ => entered,
    };
    let (hot, sub) = match &target {
        Target::Item {
            path,
            pick: Pick::Enabled,
            branch,
        } => {
            let hot = Some(path.clone());
            let sub = select(sub, path, *branch, now, p, &mut out);
            (hot, sub)
        }
        Target::Item {
            pick: Pick::Inert, ..
        } => (None, close_sub(sub, &mut out)),
        _ => (hot, sub),
    };
    if hot != before {
        out.insert(0, MenuOut::Highlight(hot.clone()));
    }
    (
        MenuTrack::Tracking {
            menu: current.menu,
            shown: current.shown,
            held,
            entered,
            hot,
            sub,
            pointer: at,
            under: target,
        },
        out,
    )
}

/// The submenu state after the pointer lands on the pickable item `path`.
fn select(
    sub: Sub,
    path: &ItemPath,
    branch: Branch,
    now: Stamp,
    p: &MenuParams,
    out: &mut Vec<MenuOut>,
) -> Sub {
    match (sub, branch) {
        (Sub::Open { item, guard }, _) if &item == path => Sub::Open { item, guard },
        (Sub::Pending { item, since }, Branch::Submenu) if &item == path => {
            Sub::Pending { item, since }
        }
        (sub, Branch::Submenu) => {
            let _ = close_sub(sub, out);
            out.push(MenuOut::RequestTick(now.after(p.submenu_delay_ms)));
            Sub::Pending {
                item: path.clone(),
                since: now,
            }
        }
        (sub, Branch::Leaf) => close_sub(sub, out),
    }
}

fn close_sub(sub: Sub, out: &mut Vec<MenuOut>) -> Sub {
    if matches!(sub, Sub::Open { .. }) {
        out.push(MenuOut::CloseSub);
    }
    Sub::None
}

/// A timer: the submenu delay, or the safe triangle's timeout.
#[allow(clippy::too_many_arguments)]
fn ticked(
    current: Current,
    held: Held,
    entered: Entered,
    hot: Option<ItemPath>,
    sub: Sub,
    pointer: Point,
    under: Target,
    now: Stamp,
    p: &MenuParams,
) -> (MenuTrack, Vec<MenuOut>) {
    match sub {
        Sub::Pending { item, since } if now.since(since) >= p.submenu_delay_ms => {
            let out = vec![MenuOut::OpenSub(item.clone())];
            let sub = Sub::Open { item, guard: None };
            (
                MenuTrack::Tracking {
                    menu: current.menu,
                    shown: current.shown,
                    held,
                    entered,
                    hot,
                    sub,
                    pointer,
                    under,
                },
                out,
            )
        }
        Sub::Open {
            item,
            guard: Some(g),
        } if now.since(g.still_since) >= p.triangle_timeout_ms => {
            // The pointer stopped inside the triangle: select what is under it.
            let sub = Sub::Open { item, guard: None };
            moved(current, held, entered, hot, sub, pointer, under, now, p)
        }
        sub => (
            MenuTrack::Tracking {
                menu: current.menu,
                shown: current.shown,
                held,
                entered,
                hot,
                sub,
                pointer,
                under,
            },
            Vec::new(),
        ),
    }
}

/// Escape closes one level; Left and Right cross into a submenu or the next bar menu; Enter
/// picks the highlighted item.
#[allow(clippy::too_many_arguments)]
fn keyed(
    current: Current,
    held: Held,
    entered: Entered,
    hot: Option<ItemPath>,
    sub: Sub,
    pointer: Point,
    under: Target,
    key: MenuKey,
) -> (MenuTrack, Vec<MenuOut>) {
    let keep = |sub, out| {
        (
            MenuTrack::Tracking {
                menu: current.menu,
                shown: current.shown,
                held,
                entered,
                hot: hot.clone(),
                sub,
                pointer,
                under: under.clone(),
            },
            out,
        )
    };
    match (key, sub) {
        (MenuKey::Escape | MenuKey::Left, Sub::Open { .. }) => {
            keep(Sub::None, vec![MenuOut::CloseSub])
        }
        (MenuKey::Escape, _) => (MenuTrack::Closed, vec![MenuOut::Close(Anim::Fade)]),
        (MenuKey::Right, Sub::Pending { item, .. }) => {
            let out = vec![MenuOut::OpenSub(item.clone())];
            keep(Sub::Open { item, guard: None }, out)
        }
        (MenuKey::Left, sub) => keep(sub, vec![MenuOut::Adjacent(Side::Left)]),
        (MenuKey::Right, sub) => keep(sub, vec![MenuOut::Adjacent(Side::Right)]),
        (MenuKey::Enter, sub) => match hot.clone() {
            Some(path) => (
                MenuTrack::Closed,
                vec![MenuOut::Close(Anim::Fade), MenuOut::Pick(path)],
            ),
            None => keep(sub, Vec::new()),
        },
    }
}
