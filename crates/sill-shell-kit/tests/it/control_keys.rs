//! Key activation on a real Blitz document (design/30 section 1.4, Press): the renderer raises no
//! `click` for a key on a focused `button`, so every button-like control reports the activation
//! itself. Return and Space press a `Button` (a toolbar button with only an image included: the
//! one that once did nothing), each once; Space alone flips a `Toggle` and a `Checkbox` and picks
//! a segment; the arrows check the neighbouring radio button and move the focus with it.

use dioxus::prelude::*;
use ds::components::controls::button_model::{Answers, Bezel, ImagePosition};
use ds::components::controls::checkbox::Checkbox;
use ds::components::controls::segmented::Tracking;
use ds::prelude::*;
use ds::style::tokens::control_size::ControlSize;
use ds_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 360,
    height: 200,
    scale_percent: 100,
};

thread_local! {
    static PRESSES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[allow(non_snake_case)]
fn Buttons() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            Button {
                bezel: Bezel::Toolbar,
                size: ControlSize::Large,
                image: ImagePosition::Only,
                icon: Icon::Trash,
                label: "Delete",
                onclick: move |_| PRESSES.set(PRESSES.get() + 1),
            }
            Button { answers: Answers::Escape, label: "Cancel", onclick: move |_| PRESSES.set(PRESSES.get() + 100) }
        }
    }
}

fn harness(app: fn() -> Element) -> Harness {
    let mut harness = Harness::new(app, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(50));
    harness
}

#[test]
fn return_and_space_press_an_image_only_button_once_each() {
    PRESSES.set(0);
    let mut harness = harness(Buttons);
    harness.send(Input::key(ShortcutKey::Tab));
    harness.send(Input::key(ShortcutKey::Enter));
    assert_eq!(PRESSES.get(), 1, "Return presses it once");
    harness.send(Input::key(ShortcutKey::Space));
    assert_eq!(PRESSES.get(), 2, "Space presses it once");
}

#[test]
fn a_button_that_answers_escape_takes_space_and_leaves_return_to_the_dialog() {
    PRESSES.set(0);
    let mut harness = harness(Buttons);
    harness.send(Input::key(ShortcutKey::Tab));
    harness.send(Input::key(ShortcutKey::Tab));
    harness.send(Input::key(ShortcutKey::Enter));
    assert_eq!(PRESSES.get(), 0, "Return is the default button's");
    harness.send(Input::key(ShortcutKey::Space));
    assert_eq!(PRESSES.get(), 100, "Space presses it");
}

#[allow(non_snake_case)]
fn Choices() -> Element {
    let mut on = use_signal(|| Check::Off);
    let mut ticked = use_signal(|| Check::Off);
    let mut which = use_signal(|| 0u8);
    let mut radio = use_signal(|| 0u8);
    let pairs = || vec![(0u8, "One"), (1u8, "Two"), (2u8, "Three")];
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            Toggle { label: "Wi-Fi", value: on(), onchange: move |next| on.set(next) }
            Checkbox { label: "Remember", value: ticked(), onchange: move |next| ticked.set(next) }
            SegmentedControl::<u8> {
                label: "View",
                choices: Choice::pairs(pairs()),
                tracking: Tracking::SelectOne(which()),
                onchange: move |next| which.set(next),
            }
            RadioGroup::<u8> {
                label: "Size",
                choices: Choice::pairs(pairs()),
                value: radio(),
                onchange: move |next| radio.set(next),
            }
            p { class: "state", "{on().slug_of()} {ticked().slug_of()} {which()} {radio()}" }
        }
    }
}

trait SlugOf {
    fn slug_of(self) -> &'static str;
}

impl SlugOf for Check {
    fn slug_of(self) -> &'static str {
        match self {
            Check::On => "on",
            Check::Off => "off",
            Check::Mixed => "mixed",
        }
    }
}

fn state(harness: &Harness) -> String {
    harness.text_of(".state").unwrap_or_default()
}

#[test]
fn space_flips_a_switch_and_a_checkbox_and_picks_a_segment() {
    let mut harness = harness(Choices);
    assert_eq!(state(&harness), "off off 0 0");
    harness.send(Input::key(ShortcutKey::Tab));
    harness.send(Input::key(ShortcutKey::Space));
    assert_eq!(state(&harness), "on off 0 0", "the switch");
    harness.send(Input::key(ShortcutKey::Enter));
    assert_eq!(
        state(&harness),
        "on off 0 0",
        "Return is not a switch's key"
    );
    harness.send(Input::key(ShortcutKey::Tab));
    harness.send(Input::key(ShortcutKey::Space));
    assert_eq!(state(&harness), "on on 0 0", "the checkbox");
    harness.send(Input::key(ShortcutKey::Tab));
    harness.send(Input::key(ShortcutKey::Right));
    assert_eq!(
        state(&harness),
        "on on 1 0",
        "an arrow moves the segments' choice"
    );
}

#[test]
fn the_arrows_check_the_next_radio_button_and_the_group_has_one_tab_stop() {
    let mut harness = harness(Choices);
    for _ in 0..4 {
        harness.send(Input::key(ShortcutKey::Tab));
    }
    assert_eq!(
        harness
            .attr(".ds-radio-group-item[*|aria-checked=true]", "tabindex")
            .as_deref(),
        Some("0")
    );
    harness.send(Input::key(ShortcutKey::Down));
    assert_eq!(state(&harness), "off off 0 1");
    harness.advance(Duration::from_millis(50));
    harness.send(Input::key(ShortcutKey::Down));
    assert_eq!(state(&harness), "off off 0 2");
    harness.send(Input::key(ShortcutKey::Down));
    assert_eq!(state(&harness), "off off 0 2", "the end stops");
    harness.send(Input::key(ShortcutKey::Home));
    assert_eq!(state(&harness), "off off 0 0");
}

#[allow(non_snake_case)]
fn Pills() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            WorkspacePills { label: "Workspaces",
                WorkspacePill { label: "Work", onclick: move |_| PRESSES.set(PRESSES.get() + 1) }
            }
        }
    }
}

#[test]
fn a_workspace_pill_takes_return_and_space_too() {
    PRESSES.set(0);
    let mut harness = harness(Pills);
    harness.send(Input::key(ShortcutKey::Tab));
    harness.send(Input::key(ShortcutKey::Enter));
    harness.send(Input::key(ShortcutKey::Space));
    assert_eq!(PRESSES.get(), 2, "one press for each key");
}
