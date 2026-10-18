//! BannerStack on a real Blitz document: a banner the caller lists slides in and
//! comes to rest; one it stops listing slides out and stays drawn until `settle(PanelOut)`,
//! then the banners after it heal into its place by the height it measured, and `on_hidden`
//! hears its key; the banners before it never move; an empty stack reports every key.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
use sill_shell_kit::notifications::banner_stack::{Banner, BannerKey};
use sill_shell_kit::notifications::parts::AppMark;
use sill_shell_kit::notifications::swipe::NotificationSwipe;
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 420,
    height: 400,
    scale_percent: 100,
};

static SHOWN: GlobalSignal<Vec<u32>> = Signal::global(Vec::new);
static HIDDEN: GlobalSignal<Vec<u32>> = Signal::global(Vec::new);

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
                    body: "Two lines of body, or thereabouts, to give the banner some height.",
                    on_close: |_| {},
                    on_open: |_| {},
                    swipe: NotificationSwipe::Dismiss(EventHandler::new(move |()| SHOWN.write().retain(|k| *k != n))),
                }
            },
        })
        .collect();
    let hidden = HIDDEN()
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Toast,
            BannerStack { banners, on_hidden: move |key: BannerKey| HIDDEN.write().push(key.0) }
            p { class: "hidden", "{hidden}" }
        }
    }
}

fn row(n: u32) -> String {
    format!(".ds-banner[*|data-banner=\"{n}\"]")
}

fn presence(harness: &Harness, n: u32) -> Option<String> {
    harness.attr(&row(n), "data-presence")
}

fn hidden(harness: &Harness) -> String {
    harness.text_of(".hidden").unwrap_or_default()
}

fn show(harness: &mut Harness, keys: &[u32]) {
    harness.within(|| *SHOWN.write() = keys.to_vec());
    harness.advance(Duration::from_millis(1));
}

/// A stack showing `keys`, every banner at rest (it mounts empty, and they arrive).
fn start(keys: &[u32]) -> Harness {
    let mut harness = Harness::new(Stack, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    show(&mut harness, keys);
    for &n in keys {
        settle_until(&mut harness, |h| {
            presence(h, n).as_deref() == Some("present")
        });
    }
    harness
}

#[test]
fn a_banner_listed_slides_in_and_comes_to_rest() {
    let mut harness = start(&[]);
    assert_eq!(harness.count(".ds-banner"), 0);
    show(&mut harness, &[7]);
    assert_eq!(presence(&harness, 7).as_deref(), Some("entering"));
    settle_until(&mut harness, |h| {
        presence(h, 7).as_deref() == Some("present")
    });
}

#[test]
fn one_removed_from_the_middle_leaves_and_those_after_it_heal() {
    let mut harness = start(&[3, 2, 1]);
    let pitch = harness.rect(&row(2)).expect("row 2").size.height.0;
    let out = settle(Anim::PanelOut, MotionLevel::Standard);
    let removed = harness.now();
    show(&mut harness, &[3, 1]);
    assert_eq!(presence(&harness, 2).as_deref(), Some("leaving"));
    assert_eq!(presence(&harness, 1).as_deref(), Some("present"));
    harness.advance(out / 2);
    assert_eq!(
        harness.count(".ds-banner"),
        3,
        "still drawn while it leaves"
    );
    assert_eq!(hidden(&harness), "");

    let gone = settle_until(&mut harness, |h| h.count(".ds-banner") == 2);
    assert_eq!(gone.duration_since(removed), out);
    assert_eq!(presence(&harness, 1).as_deref(), Some("healing"));
    assert_eq!(
        presence(&harness, 3).as_deref(),
        Some("present"),
        "the one before never moves"
    );
    let style = harness.attr(&row(1), "style").unwrap_or_default();
    let dy: f32 = style
        .split(';')
        .find_map(|part| part.strip_prefix("--dy:"))
        .and_then(|value| value.trim_end_matches("px").parse().ok())
        .expect("a heal distance");
    assert!(
        (dy - pitch).abs() < 0.5,
        "heals by the row it replaces: {dy} against {pitch}"
    );
    assert_eq!(hidden(&harness), "2");
    settle_until(&mut harness, |h| {
        presence(h, 1).as_deref() == Some("present")
    });
}

#[test]
fn an_emptied_stack_reports_every_key_once_its_exits_settle() {
    let mut harness = start(&[2, 1]);
    show(&mut harness, &[]);
    assert_eq!(harness.count(".ds-banner"), 2);
    settle_until(&mut harness, |h| h.count(".ds-banner") == 0);
    let mut keys: Vec<String> = hidden(&harness).split(',').map(str::to_owned).collect();
    keys.sort();
    assert_eq!(keys, ["1", "2"]);
}

#[test]
fn a_banner_listed_again_while_it_leaves_stays() {
    let mut harness = start(&[2, 1]);
    show(&mut harness, &[1]);
    assert_eq!(presence(&harness, 2).as_deref(), Some("leaving"));
    show(&mut harness, &[2, 1]);
    assert_eq!(presence(&harness, 2).as_deref(), Some("present"));
    harness.advance(Duration::from_millis(500));
    assert_eq!(harness.count(".ds-banner"), 2);
    assert_eq!(hidden(&harness), "");
}

#[test]
fn a_card_swiped_away_in_the_stack_is_carried_out_by_its_row() {
    let mut harness = start(&[2, 1]);
    let at = harness
        .centre(&format!("{} .ds-notification-plate", row(2)))
        .expect("banner 2");
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
    // Reported at the release, not after a flight of its own: the caller dropped it at once, so
    // its row is already leaving, while the card holds where the finger left it.
    assert_eq!(presence(&harness, 2).as_deref(), Some("leaving"));
    let card = format!("{} .ds-notification", row(2));
    assert_eq!(harness.attr(&card, "data-swipe").as_deref(), Some("gone"));
    assert!(
        harness
            .attr(&card, "style")
            .is_some_and(|style| style.contains("--swipe-dx:100px"))
    );
    settle_until(&mut harness, |h| h.count(".ds-banner") == 1);
    assert_eq!(hidden(&harness), "2");
}
