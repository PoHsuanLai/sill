//! NotificationCard on a real Blitz document: a press on the close button,
//! an action or a link in the body stays there and never opens the notification, while a press
//! on the card does; under the pointer the card says so (`data-hover`, `on_hover`) and its body
//! opens from two lines to six, measured, and closes again when the pointer leaves.

use dioxus::prelude::*;
use ds::components::content::rich_text::{Rich, RichRun};
use ds::components::content::text_runs::RunTone;
use ds::prelude::*;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, FocusState, Harness, HarnessConfig, Input, Viewport};
use sill_shell_kit::notifications::parts::{AppMark, CardAction, Hover};
use sill_shell_kit::prelude::*;

const VIEW: Viewport = Viewport {
    width: 480,
    height: 360,
    scale_percent: 100,
};

static LOG: GlobalSignal<Vec<String>> = Signal::global(Vec::new);

fn note(entry: &str) {
    LOG.write().push(entry.to_owned());
}

/// A body long enough to run past six lines at 360 px.
fn body() -> Rich {
    let words = "The analytical engine weaves algebraic patterns just as the Jacquard loom weaves \
                 flowers and leaves, and the notes I have added run rather longer than the \
                 memoir itself, so there is much more here than two lines can hold. ";
    Rich(vec![
        RichRun::link("Read the notes", "https://example.org/notes"),
        RichRun::Run(TextRun::new(
            format!(": {}", words.repeat(2)),
            RunTone::Plain,
        )),
    ])
}

#[allow(non_snake_case)]
fn Card() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Toast,
            div { style: "padding:24px",
                NotificationCard {
                    app: AppMark { icon: IconSource::Glyph(Icon::Mail), name: "Mail".into() },
                    age: "now",
                    summary: "Ada Lovelace",
                    body: body(),
                    actions: vec![CardAction { label: "Reply".into(), on_press: EventHandler::new(|_| note("action")) }],
                    on_close: |_| note("close"),
                    on_open: |_| note("open"),
                    on_link: |href: String| note(&format!("link {href}")),
                    on_hover: |hover: Hover| note(&format!("hover {hover:?}")),
                }
            }
        }
    }
}

fn centre(harness: &Harness, selector: &str) -> Point {
    harness
        .centre(selector)
        .unwrap_or_else(|| panic!("{selector} is not in the document:\n{}", harness.html()))
}

fn log(harness: &mut Harness) -> Vec<String> {
    harness.within(|| LOG.peek().clone())
}

fn body_height(harness: &Harness) -> f32 {
    harness
        .rect(".ds-notification-body")
        .map_or(0.0, |rect| rect.size.height.0)
}

#[test]
fn the_close_button_an_action_and_a_link_keep_their_press() {
    let mut harness = Harness::new(Card, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.within(|| LOG.write().clear());
    let over = centre(&harness, ".ds-notification-plate");
    harness.send(Input::pointer_move(over));
    settle_until(&mut harness, |h| {
        h.attr(".ds-notification", "data-hover").as_deref() == Some("on")
    });
    // The actions row opens over --t-move: wait until it holds its button whole.
    settle_until(&mut harness, |h| {
        let row = h.rect(".ds-notification-actions");
        let button = h.rect(".ds-notification-actions .ds-button");
        matches!((row, button), (Some(row), Some(button)) if button.size.height.0 > 0.0
            && row.size.height.0 >= button.size.height.0)
    });

    let close = centre(&harness, ".ds-notification-close");
    harness.send(Input::click(close));
    let action = centre(&harness, ".ds-notification-actions .ds-button");
    harness.send(Input::click(action));
    let link = centre(&harness, ".ds-run-link");
    harness.send(Input::click(link));
    let entries = log(&mut harness);
    assert!(
        entries.contains(&"close".to_owned()),
        "the close press was heard: {entries:?}"
    );
    assert!(entries.contains(&"action".to_owned()), "{entries:?}");
    assert!(
        entries.contains(&"link https://example.org/notes".to_owned()),
        "{entries:?}"
    );
    assert!(
        !entries.contains(&"open".to_owned()),
        "no press on the close button, an action or a link opened the card: {entries:?}"
    );

    // A press on the card itself opens it, on its words as on its body's text.
    let head = centre(&harness, ".ds-notification-summary");
    harness.send(Input::click(head));
    assert_eq!(log(&mut harness).iter().filter(|e| *e == "open").count(), 1);
    let text = harness.rect(".ds-notification-body").expect("a body");
    harness.send(Input::click(Point {
        x: Px(text.origin.x.0 + text.size.width.0 - 20.0),
        y: Px(text.origin.y.0 + 8.0),
    }));
    assert_eq!(log(&mut harness).iter().filter(|e| *e == "open").count(), 2);
}

#[test]
fn under_the_pointer_the_body_opens_and_it_closes_again_when_the_pointer_leaves() {
    let mut harness = Harness::new(Card, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    // The body measures its lines after layout, then writes the clip.
    settle_until(&mut harness, |h| {
        h.attr(".ds-notification-body", "data-clip").as_deref() == Some("always")
    });
    let rest = body_height(&harness);
    assert_eq!(
        harness.attr(".ds-notification", "data-hover").as_deref(),
        Some("off")
    );

    let over = centre(&harness, ".ds-notification-plate");
    harness.send(Input::pointer_move(over));
    settle_until(&mut harness, |h| body_height(h) >= rest * 2.95);
    let open = body_height(&harness);
    assert!(
        (open / rest - 3.0).abs() < 0.2,
        "six lines under the pointer, two at rest: {open} against {rest}"
    );
    harness.within(|| {
        assert!(LOG.peek().contains(&"hover Over".to_owned()));
    });

    harness.send(Input::pointer_move(Point {
        x: Px(470.0),
        y: Px(350.0),
    }));
    settle_until(&mut harness, |h| (body_height(h) - rest).abs() < 0.5);
    assert_eq!(
        harness.attr(".ds-notification", "data-hover").as_deref(),
        Some("off")
    );
    harness.within(|| {
        assert!(LOG.peek().contains(&"hover Away".to_owned()));
    });
}

/// Tab until `selector` has the keyboard, at most `limit` presses.
fn tab_to(harness: &mut Harness, selector: &str, limit: usize) -> bool {
    (0..limit).any(|_| {
        harness.send(Input::key(ShortcutKey::Tab));
        harness.focus_of(selector) == FocusState::Focused
    })
}

#[test]
fn the_close_button_takes_the_keyboard_and_closes_on_enter_and_space() {
    let mut harness = Harness::new(Card, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.within(|| LOG.write().clear());
    assert!(
        tab_to(&mut harness, ".ds-notification-close", 6),
        "Tab reaches the close button"
    );
    let closes = |harness: &mut Harness| {
        log(harness)
            .iter()
            .filter(|entry| entry.as_str() == "close")
            .count()
    };
    let before = closes(&mut harness);
    harness.send(Input::key(ShortcutKey::Enter));
    harness.advance(std::time::Duration::from_millis(50));
    assert_eq!(closes(&mut harness), before + 1, "Enter closes");
    harness.send(Input::key(ShortcutKey::Space));
    harness.advance(std::time::Duration::from_millis(50));
    assert_eq!(closes(&mut harness), before + 2, "Space closes");
    assert!(
        !log(&mut harness).contains(&"open".to_owned()),
        "a key on the close button never opens the card"
    );
}
