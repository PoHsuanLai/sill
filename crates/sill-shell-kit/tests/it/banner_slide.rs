//! A `BannerStack`'s slide on a real Blitz document (design/30 section 1.3: a banner slides in
//! from past the right edge and out to the right like a toast): a banner starts past its right
//! edge and comes to rest on its place; a card swiped away leaves along the swipe and its row is
//! dropped when the flight has settled.
//!
//! Blitz's layout rects leave transforms out (`Harness::rect`), so where the card is drawn is
//! read with the hit test, which applies them: a point just past the resting card's right edge
//! lands on the card while it is still sliding in, and never once it rests.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Query, Viewport};
use sill_shell_kit::notifications::banner_stack::{Banner, BannerKey};
use sill_shell_kit::notifications::parts::AppMark;
use sill_shell_kit::notifications::swipe::NotificationSwipe;
use sill_shell_kit::prelude::*;
use std::cell::Cell;
use std::time::Duration;

/// Wider than the stack, so a card displaced past its right edge is still on the page.
const VIEW: Viewport = Viewport {
    width: 840,
    height: 400,
    scale_percent: 100,
};

static SHOWN: GlobalSignal<Vec<u32>> = Signal::global(Vec::new);

#[allow(non_snake_case)]
fn Stack() -> Element {
    let banners: Vec<Banner> = SHOWN()
        .into_iter()
        .map(|n| Banner {
            key: BannerKey(n),
            card: rsx! {
                NotificationCard {
                    app: AppMark { icon: IconSource::Glyph(Icon::Mail), name: "Mail".into() },
                    age: "now",
                    summary: "Banner {n}",
                    on_close: |_| {},
                    on_open: |_| {},
                    swipe: NotificationSwipe::Dismiss(EventHandler::new(move |()| SHOWN.write().retain(|k| *k != n))),
                }
            },
        })
        .collect();
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Toast,
            div { style: "width:420px",
                BannerStack { banners }
            }
        }
    }
}

const CARD: &str = ".ds-notification";

/// Probe points around the resting card: its centre, and just past its right edge.
struct Probes {
    rest: Point,
    right: Point,
}

impl Probes {
    fn around(card: Rect) -> Self {
        let centre = Point {
            x: Px(card.origin.x.0 + card.size.width.0 / 2.0),
            y: Px(card.origin.y.0 + card.size.height.0 / 2.0),
        };
        Probes {
            rest: centre,
            right: Point {
                x: Px(card.origin.x.0 + card.size.width.0 + 8.0),
                y: centre.y,
            },
        }
    }
}

/// Whether the card was drawn past its right edge at some point of its entrance.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    PastTheEdge,
    Never,
}

/// Mounts one banner and polls until it rests on its place, noting whether the card was drawn
/// past its right edge on the way.
fn enter() -> (Harness, Probes, Seen) {
    let mut harness = Harness::new(Stack, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.within(|| *SHOWN.write() = vec![1]);
    harness.advance(Duration::from_millis(1));
    // The layout rect is the resting place: transforms never move it.
    let probes = Probes::around(harness.rect(CARD).expect("the card"));
    let right = Cell::new(false);
    settle_until(&mut harness, |h| {
        right.set(right.get() || h.hits(probes.right, CARD));
        h.attr(".ds-banner", "data-presence").as_deref() == Some("present")
            && h.hits(probes.rest, CARD)
            && !h.hits(probes.right, CARD)
    });
    let seen = if right.get() {
        Seen::PastTheEdge
    } else {
        Seen::Never
    };
    (harness, probes, seen)
}

#[test]
fn a_banner_starts_past_its_right_edge_and_rests_on_its_place() {
    let (_, _, seen) = enter();
    assert_eq!(seen, Seen::PastTheEdge);
}

#[test]
fn a_card_swiped_away_leaves_along_the_swipe_and_its_row_goes() {
    let (mut harness, probes, _) = enter();
    let at = probes.rest;
    harness.send(Input::pointer_down(at));
    for step in 1..=4u8 {
        harness.advance(Duration::from_millis(100));
        harness.send(Input::pointer_move(Point {
            x: Px(at.x.0 + 25.0 * f32::from(step)),
            y: at.y,
        }));
    }
    harness.send(Input::pointer_up(Point {
        x: Px(at.x.0 + 100.0),
        y: at.y,
    }));
    assert_eq!(
        harness.attr(".ds-banner", "data-presence").as_deref(),
        Some("leaving")
    );
    settle_until(&mut harness, |h| h.count(".ds-banner") == 0);
}
