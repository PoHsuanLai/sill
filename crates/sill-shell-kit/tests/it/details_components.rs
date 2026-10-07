//! design/26 D0b on the two components whose spinners looped forever (section 6): a module tile
//! that stays Busy, and a lock prompt that stays Checking, spin at once and keep turning for as
//! long as the state holds, then go and paint 0 frames, with nothing new from the caller (the
//! operation is derived from the state's own Pending moment).

use dioxus::prelude::*;
use ds::components::content::avatar::{
    AvatarFace, AvatarShape, AvatarSize, AvatarTone, person_hue,
};
use ds::prelude::*;
use ds_harness::harness::assert_settles_to_zero_frames;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Query, Viewport};
use sill_shell_kit::lock::vocab::{LockUser, PromptState};
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 360,
    height: 240,
    scale_percent: 100,
};

/// Long past where a cap used to hold a still frame.
const PAST_CAP: Duration = Duration::from_millis(10_500);

static MODULE: GlobalSignal<(Check, Availability)> =
    Signal::global(|| (Check::Off, Availability::Enabled));
static PROMPT: GlobalSignal<PromptState> = Signal::global(|| PromptState::Idle);

#[allow(non_snake_case)]
fn Tile() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Popover,
            div { style: "width:300px;padding:12px",
                ModuleTile { glyph: Icon::Wifi, title: TextLine::from("Wi-Fi"), status: None, value: MODULE().0, availability: MODULE().1, onclick: |_| {} }
            }
        }
    }
}

fn pending(harness: &Harness) -> Option<String> {
    harness.attr(".ds-progress", "data-pending")
}

/// The spinner turns at once and is still turning well past where a cap used to stop it.
fn spins_and_keeps_spinning(harness: &mut Harness) {
    harness.advance(Duration::from_millis(50));
    assert_eq!(pending(harness).as_deref(), Some("step"), "no grace");
    harness.advance(PAST_CAP);
    let turn = harness.attr(".ds-progress", "style");
    assert_eq!(
        pending(harness).as_deref(),
        Some("step"),
        "{}",
        harness.html()
    );
    harness.advance(Duration::from_millis(83));
    assert_ne!(harness.attr(".ds-progress", "style"), turn, "still turning");
}

#[test]
fn a_busy_module_tile_spins_until_it_lands() {
    let mut harness = Harness::new(Tile, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.within(|| *MODULE.write() = (Check::Off, Availability::Busy));
    spins_and_keeps_spinning(&mut harness);
    harness.within(|| *MODULE.write() = (Check::On, Availability::Enabled));
    harness.advance(Duration::from_millis(30));
    assert_eq!(harness.count(".ds-progress"), 0);
    assert_settles_to_zero_frames(&mut harness);
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
fn Lock() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Window,
            LockPrompt { user: user(), state: PROMPT(), oninput: |_| {}, onsubmit: |_| {} }
        }
    }
}

#[test]
fn a_checking_lock_prompt_spins_until_the_try_ends() {
    let mut harness = Harness::new(Lock, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.within(|| *PROMPT.write() = PromptState::Checking);
    spins_and_keeps_spinning(&mut harness);
    // The try fails: the ring goes, the field shakes once, and the prompt rests.
    harness.within(|| *PROMPT.write() = PromptState::Wrong);
    harness.advance(Duration::from_millis(30));
    assert_eq!(harness.count(".ds-progress"), 0);
    assert_settles_to_zero_frames(&mut harness);
}
