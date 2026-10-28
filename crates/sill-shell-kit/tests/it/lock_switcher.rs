//! The lock prompt, the polkit prompt and the app switcher on a real Blitz document (M11). A
//! wrong password shakes the field once and the field empties only after the shake has settled;
//! Enter hands the typed secret to `onsubmit`, never writing it into the markup; Escape empties
//! the field; the switcher reports the tile the pointer rests on and the one clicked, and keeps
//! the selection inside its view when the row scrolls. An emoji in the lock prompt plays its own
//! animation once as the prompt appears and then rests.

use dioxus::prelude::*;
use ds::components::content::avatar::{
    AvatarFace, AvatarShape, AvatarSize, AvatarTone, person_hue,
};
use ds::prelude::*;
use ds::style::icon::family::PlateFamily;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
use sill_shell_kit::emoji::id::EmojiId;
use sill_shell_kit::lock::vocab::{LockUser, PromptState};
use sill_shell_kit::prelude::*;
use sill_shell_kit::switcher::app_switcher::{AppKey, SwitcherApp};
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 900,
    height: 420,
    scale_percent: 100,
};

static STATE: GlobalSignal<PromptState> = Signal::global(PromptState::default);
static HEARD: GlobalSignal<Vec<String>> = Signal::global(Vec::new);
static SUBMITTED: GlobalSignal<Vec<String>> = Signal::global(Vec::new);
static CANCELLED: GlobalSignal<u32> = Signal::global(|| 0);

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn user() -> LockUser {
    LockUser::new(
        "Dana Reyes",
        AvatarFace {
            initial: 'D',
            size: AvatarSize::Size34,
            tone: AvatarTone::Person(person_hue("dana")),
            shape: AvatarShape::Round,
        },
    )
}

#[allow(non_snake_case)]
fn EmojiLock() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { style: "display:flex; justify-content:center; padding:24px",
                LockPrompt {
                    user: LockUser::new("Dana Reyes", EmojiId::Wink),
                    state: STATE(),
                    oninput: move |text: String| HEARD.write().push(text),
                    onsubmit: move |text: String| SUBMITTED.write().push(text),
                }
            }
        }
    }
}

#[allow(non_snake_case)]
fn Lock() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            div { style: "display:flex; justify-content:center; padding:24px",
                LockPrompt {
                    user: user(),
                    state: STATE(),
                    oninput: move |text: String| HEARD.write().push(text),
                    onsubmit: move |text: String| SUBMITTED.write().push(text),
                }
            }
        }
    }
}

#[allow(non_snake_case)]
fn Polkit() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Sheet,
            div { style: "height:400px" }
            PolkitPrompt {
                action: "Authentication is required to change the system's time zone.",
                user: user(),
                state: STATE(),
                oninput: move |text: String| HEARD.write().push(text),
                onsubmit: move |text: String| SUBMITTED.write().push(text),
                oncancel: move |()| *CANCELLED.write() += 1,
            }
        }
    }
}

/// Type `text` into the focused field, one key at a time, letting each render land.
fn type_text(harness: &mut Harness, text: &str) {
    for c in text.chars() {
        harness.send(Input::key(ShortcutKey::Char(c)));
        harness.advance(ms(20));
    }
}

// On `Clock::Virtual` so `set_state`'s 1 ms advances between successive state writes cannot
// drift: on `Clock::Wall` a loaded machine's overshoot on such a short `advance` could let a
// mood or shake timer settle inside a step meant to hold it (the emoji glances test
// flaked this way).
fn mounted(app: fn() -> Element) -> Harness {
    let mut harness = Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.within(|| {
        *STATE.write() = PromptState::Idle;
        HEARD.write().clear();
        SUBMITTED.write().clear();
        *CANCELLED.write() = 0;
    });
    harness.advance(ms(60));
    harness
}

fn set_state(harness: &mut Harness, state: PromptState) {
    harness.within(|| *STATE.write() = state);
    harness.advance(ms(1));
}

fn shaking(harness: &Harness) -> bool {
    harness.has_class(".ds-lock-prompt .ds-password-field", "a-shake-x")
}

fn dots(harness: &Harness, field: &str) -> Option<String> {
    harness.text_of(&format!("{field} .ds-input-mask"))
}

const LOCK_INPUT: &str = ".ds-lock-prompt .ds-password-field input";

#[test]
fn a_wrong_password_shakes_once_and_empties_the_field_after_the_shake() {
    let mut harness = mounted(Lock);
    assert!(harness.focus_of(LOCK_INPUT), "the field takes the keyboard");
    type_text(&mut harness, "abc");
    assert_eq!(
        dots(&harness, ".ds-lock-prompt .ds-password-field").as_deref(),
        Some("•••")
    );
    assert!(!shaking(&harness), "nothing shakes before it was wrong");

    let shake = settle(Anim::ShakeX, MotionLevel::Standard);
    // Marked before the state write that starts the settle timer, so the comparison below is a
    // true lower bound.
    let wrong = harness.now();
    set_state(&mut harness, PromptState::Wrong);
    assert!(shaking(&harness), "{}", harness.html());
    assert_eq!(
        harness
            .attr(".ds-lock-prompt .ds-password-field", "data-pulse")
            .as_deref(),
        Some("a")
    );
    // Half the shake: still playing, and the field still holds what was typed.
    harness.advance(shake / 2);
    assert!(shaking(&harness), "still shaking at half the settle");
    assert_eq!(
        dots(&harness, ".ds-lock-prompt .ds-password-field").as_deref(),
        Some("•••")
    );

    let rested = settle_until(&mut harness, |h| !shaking(h));
    assert_eq!(
        rested.duration_since(wrong),
        shake,
        "the shake rested only once it had played"
    );
    assert_eq!(
        dots(&harness, ".ds-lock-prompt .ds-password-field"),
        None,
        "the field emptied"
    );
    assert_eq!(
        harness.within(|| HEARD.peek().last().cloned()),
        Some(String::new()),
        "the caller heard the field empty"
    );
    assert!(
        harness.focus_of(LOCK_INPUT),
        "the caret is back in the field"
    );

    // Once: it stays at rest while the state stays wrong.
    harness.advance(ms(500));
    assert!(!shaking(&harness), "the shake does not loop");
    assert_eq!(
        harness.attr(".ds-lock-prompt", "data-state").as_deref(),
        Some("wrong")
    );
}

fn face(harness: &Harness) -> Option<String> {
    harness.attr(".ds-lock-prompt .ds-emoji-face", "data-emoji")
}

fn frame(harness: &Harness) -> Option<String> {
    harness.attr(".ds-lock-prompt .ds-emoji-face", "data-frame")
}

#[test]
fn the_prompt_shows_the_users_emoji_at_64_and_it_plays_once_then_rests() {
    let mut harness = mounted(EmojiLock);
    assert_eq!(
        harness
            .attr(".ds-lock-prompt .ds-emoji", "data-size")
            .as_deref(),
        Some("64")
    );
    assert_eq!(face(&harness).as_deref(), Some("wink"));
    assert_eq!(
        harness.attr(".ds-lock-prompt .ds-emoji", "data-mood"),
        None,
        "the picture has no mood"
    );
    settle_until(&mut harness, |h| frame(h).as_deref() != Some("0"));
    settle_until(&mut harness, |h| frame(h).as_deref() == Some("0"));
    // A wrong password shakes the field; the picture stays as it is.
    type_text(&mut harness, "abc");
    set_state(&mut harness, PromptState::Checking);
    set_state(&mut harness, PromptState::Wrong);
    settle_until(&mut harness, |h| !shaking(h));
    assert_eq!(face(&harness).as_deref(), Some("wink"));
    assert_eq!(frame(&harness).as_deref(), Some("0"));
}

#[test]
fn enter_hands_the_secret_to_onsubmit_and_never_writes_it() {
    let mut harness = mounted(Lock);
    type_text(&mut harness, "hunter2");
    assert_eq!(
        harness.attr(LOCK_INPUT, "value"),
        None,
        "no value attribute"
    );
    assert!(
        !harness.html().contains("hunter2"),
        "the secret is not in the markup"
    );
    let before = harness.within(|| SUBMITTED.peek().len());
    harness.send(Input::key(ShortcutKey::Enter));
    harness.advance(ms(30));
    assert_eq!(
        harness.within(|| SUBMITTED.peek().clone()),
        vec!["hunter2".to_owned()]
    );
    assert_eq!(harness.within(|| SUBMITTED.peek().len()), before + 1);
}

#[test]
fn the_arrow_submits_and_shows_only_once_something_is_typed() {
    let mut harness = mounted(Lock);
    assert_eq!(
        harness.attr(".ds-lock-go", "data-filled").as_deref(),
        Some("empty")
    );
    type_text(&mut harness, "pw");
    assert_eq!(
        harness.attr(".ds-lock-go", "data-filled").as_deref(),
        Some("typed")
    );
    harness.send(Input::click(
        harness.centre(".ds-lock-go").expect("the arrow"),
    ));
    harness.advance(ms(30));
    assert_eq!(
        harness.within(|| SUBMITTED.peek().clone()),
        vec!["pw".to_owned()]
    );
}

#[test]
fn escape_empties_the_field() {
    let mut harness = mounted(Lock);
    type_text(&mut harness, "abc");
    assert_eq!(
        dots(&harness, ".ds-lock-prompt .ds-password-field").as_deref(),
        Some("•••")
    );
    harness.send(Input::key(ShortcutKey::Escape));
    harness.advance(ms(40));
    assert_eq!(dots(&harness, ".ds-lock-prompt .ds-password-field"), None);
    assert_eq!(
        harness.within(|| HEARD.peek().last().cloned()),
        Some(String::new())
    );
    assert!(harness.focus_of(LOCK_INPUT));
    // And it takes typing again from nothing.
    type_text(&mut harness, "z");
    assert_eq!(
        dots(&harness, ".ds-lock-prompt .ds-password-field").as_deref(),
        Some("•")
    );
    assert_eq!(
        harness.within(|| HEARD.peek().last().cloned()),
        Some("z".to_owned())
    );
}

#[test]
fn checking_closes_the_field() {
    let mut harness = mounted(Lock);
    type_text(&mut harness, "ab");
    set_state(&mut harness, PromptState::Checking);
    harness.advance(ms(20));
    let heard = harness.within(|| HEARD.peek().len());
    type_text(&mut harness, "c");
    assert_eq!(
        harness.within(|| HEARD.peek().len()),
        heard,
        "no input while checking"
    );
    harness.send(Input::key(ShortcutKey::Enter));
    harness.advance(ms(20));
    assert!(
        harness.within(|| SUBMITTED.peek().is_empty()),
        "no submit while checking"
    );
    assert_eq!(
        harness.attr(".ds-lock-go", "aria-busy").as_deref(),
        Some("true")
    );
}

#[test]
fn the_polkit_prompt_submits_on_enter_shakes_when_wrong_and_cancels() {
    let mut harness = mounted(Polkit);
    let input = ".ds-polkit .ds-password-field input";
    if !harness.focus_of(input) {
        harness.send(Input::click(harness.centre(input).expect("the field")));
        harness.advance(ms(30));
    }
    type_text(&mut harness, "pw");
    harness.send(Input::key(ShortcutKey::Enter));
    harness.advance(ms(30));
    assert_eq!(
        harness.within(|| SUBMITTED.peek().clone()),
        vec!["pw".to_owned()]
    );
    set_state(&mut harness, PromptState::Wrong);
    assert!(harness.has_class(".ds-polkit .ds-password-field", "a-shake-x"));
    settle_until(&mut harness, |h| {
        !h.has_class(".ds-polkit .ds-password-field", "a-shake-x")
    });
    assert_eq!(
        dots(&harness, ".ds-polkit .ds-password-field"),
        None,
        "emptied"
    );

    let cancel = harness
        .centre(".ds-polkit-actions .ds-button[*|data-answers=escape]")
        .expect("Cancel");
    harness.send(Input::click(cancel));
    harness.advance(ms(30));
    assert_eq!(harness.within(|| *CANCELLED.peek()), 1);
}

#[test]
fn escape_in_the_polkit_field_cancels() {
    let mut harness = mounted(Polkit);
    let input = ".ds-polkit .ds-password-field input";
    if !harness.focus_of(input) {
        harness.send(Input::click(harness.centre(input).expect("the field")));
        harness.advance(ms(30));
    }
    type_text(&mut harness, "pw");
    let before = harness.within(|| *CANCELLED.peek());
    harness.send(Input::key(ShortcutKey::Escape));
    harness.advance(ms(30));
    assert_eq!(harness.within(|| *CANCELLED.peek()), before + 1);
    assert!(harness.within(|| SUBMITTED.peek().is_empty()));
}

static SELECTED: GlobalSignal<AppKey> = Signal::global(|| AppKey("files".to_owned()));
static HOVERED: GlobalSignal<Vec<AppKey>> = Signal::global(Vec::new);
static ACTIVATED: GlobalSignal<Vec<AppKey>> = Signal::global(Vec::new);
static COUNT: GlobalSignal<usize> = Signal::global(|| 5);
static OUTPUT: GlobalSignal<f32> = Signal::global(|| 1440.0);

const APPS: [(&str, Icon); 14] = [
    ("mail", Icon::Mail),
    ("files", Icon::Folder),
    ("terminal", Icon::Terminal),
    ("notes", Icon::StickyNote),
    ("photos", Icon::Image),
    ("settings", Icon::Settings),
    ("calendar", Icon::Clock),
    ("music", Icon::Headphones),
    ("camera", Icon::Camera),
    ("downloads", Icon::Download),
    ("monitor", Icon::Gauge),
    ("printer", Icon::Printer),
    ("keyboard", Icon::Keyboard),
    ("display", Icon::Monitor),
];

#[allow(non_snake_case)]
fn Switcher() -> Element {
    let apps: Vec<SwitcherApp> = APPS
        .iter()
        .take(COUNT())
        .map(|(key, icon)| SwitcherApp {
            plate: Some(PlateFamily::Blue),
            ..SwitcherApp::new(*key, *key, IconSource::Glyph(*icon))
        })
        .collect();
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Osd, chrome: Some(RootChrome::Transparent),
            div { style: "display:flex; justify-content:center; padding:24px 0",
                AppSwitcher {
                    apps,
                    selected: SELECTED(),
                    output: Some(Px(OUTPUT())),
                    onhover: move |key: AppKey| HOVERED.write().push(key),
                    onactivate: move |key: AppKey| ACTIVATED.write().push(key),
                }
            }
        }
    }
}

fn switcher(count: usize, output: f32, selected: &str) -> Harness {
    let mut harness = Harness::new(
        Switcher,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    harness.within(|| {
        *COUNT.write() = count;
        *OUTPUT.write() = output;
        *SELECTED.write() = AppKey(selected.to_owned());
        HOVERED.write().clear();
        ACTIVATED.write().clear();
    });
    harness.advance(ms(60));
    harness
}

fn cell(key: &str) -> String {
    format!(".ds-switcher-cell[*|aria-label={key}]")
}

#[test]
fn the_switcher_reports_the_tile_hovered_and_the_tile_clicked() {
    let mut harness = switcher(5, 1440.0, "files");
    let notes = harness.centre(&cell("notes")).expect("the notes tile");
    harness.send(Input::pointer_move(notes));
    harness.advance(ms(30));
    assert_eq!(
        harness.within(|| HOVERED.peek().last().cloned()),
        Some(AppKey("notes".to_owned()))
    );
    let terminal = harness
        .centre(&cell("terminal"))
        .expect("the terminal tile");
    harness.send(Input::click(terminal));
    harness.advance(ms(30));
    assert_eq!(
        harness.within(|| ACTIVATED.peek().clone()),
        vec![AppKey("terminal".to_owned())]
    );
    assert_eq!(
        harness.attr(&cell("files"), "aria-selected").as_deref(),
        Some("true"),
        "the selection is the shell's: hover and click only report"
    );
}

/// The selected cell lies inside the view, `view`'s rect, on both axes' left and right.
fn in_view(harness: &Harness, key: &str) -> bool {
    let view = harness.rect(".ds-switcher-view").expect("the view");
    let tile = harness.rect(&cell(key)).expect("the tile");
    let (left, right) = (view.origin.x.0, view.origin.x.0 + view.size.width.0);
    let (a, b) = (tile.origin.x.0, tile.origin.x.0 + tile.size.width.0);
    a >= left - 0.5 && b <= right + 0.5
}

#[test]
fn an_overflowing_row_keeps_the_selection_in_view() {
    let mut harness = switcher(14, 800.0, "display");
    assert!(
        !in_view(&harness, "mail"),
        "the row overflows: the first tile is scrolled out"
    );
    assert!(in_view(&harness, "display"), "{}", harness.html());
    for key in ["mail", "camera", "keyboard"] {
        harness.within(|| *SELECTED.write() = AppKey(key.to_owned()));
        harness.advance(ms(300));
        assert!(in_view(&harness, key), "{key} is in view");
    }
    let tile = harness.rect(&cell("camera")).expect("a tile");
    assert_eq!(tile.size.width.0, 64.0, "cells stop at the 48 px icon");
}

#[test]
fn a_tile_the_shell_stops_listing_leaves_and_goes_when_its_exit_has_settled() {
    let mut harness = switcher(5, 1440.0, "files");
    assert_eq!(harness.count(".ds-switcher-cell"), 5);
    harness.within(|| *COUNT.write() = 4);
    harness.advance(ms(30));
    assert_eq!(
        harness.attr(&cell("photos"), "data-presence").as_deref(),
        Some("leaving"),
        "the dropped tile plays its exit and stays drawn"
    );
    assert_eq!(harness.count(".ds-switcher-cell"), 5);
    settle_until(&mut harness, |h| h.count(&cell("photos")) == 0);
    assert_eq!(harness.count(".ds-switcher-cell"), 4);
}
