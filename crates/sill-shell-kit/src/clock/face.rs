//! ClockFace: a world clock's face (design/23-WIDGETS.md section 4.2; design/04-COMPONENTS.md
//! "Widgets"), an analog dial for the medium and large widgets or the time
//! as digits for the small one, with the zone's name under it.
//!
//! The analog dial is flat and bright, as the reference measures (design/23 section 2): a white
//! face by day and a dark one by night, twelve heavy numerals, sixty fine ticks where the dial is
//! large enough for them (a small widget's), thin dark hands (pale by night) and an orange
//! seconds hand. The digital face sets the time in the display face and marks the phase with a
//! sun or a moon beside the city. The hand angles are `clock_angles.rs`; the parts
//! `clock_dial.rs`.

use crate::clock::angles::hands;
use crate::clock::dial::{hands_svg, numerals, phase_mark, pin_svg, second_svg, ticks_svg};
use crate::clock::kind::{ClockLook, ClockTime, DayPhase, Seconds};
use dioxus::prelude::*;
use ds::base::word::Word;
use ds::components::content::text_runs::{TextLine, text};
use ds::root::common::Common;

/// A clock showing `time`, for `phase`, drawn as `look`, with `label` (the city) under it. A
/// digital face shows the digits; an analog one moves its hands.
#[component]
pub fn ClockFace(
    time: ClockTime,
    #[props(default)] phase: DayPhase,
    #[props(default)] look: ClockLook,
    #[props(into)] label: TextLine,
    #[props(default)] common: Common,
) -> Element {
    let class = common.class("ds-clock");
    let data = common.data_attributes();
    let (face, mark) = match look {
        ClockLook::Analog => (rsx! { AnalogDial { time } }, None),
        ClockLook::Digital => (rsx! { Digits { time } }, Some(phase_mark(phase))),
    };
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "data-look": look.slug(),
            "data-phase": phase.slug(),
            role: "img",
            "aria-label": common.aria_label.clone().unwrap_or_else(|| format!("{} {}", label.plain_text(), time.digits())),
            onmounted: move |event| common.mounted(event),
            ..data,
            {face}
            span { class: "ds-clock-label",
                {mark}
                span { class: "ds-clock-city", {text(&label)} }
            }
        }
    }
}

/// The dial: the face, its ticks (shown only on a large dial), the numerals, the hands, and the
/// seconds hand with its pin when shown.
#[component]
fn AnalogDial(time: ClockTime) -> Element {
    let at = hands(time);
    let second = match time.second {
        Seconds::Shown(_) => Some(rsx! {
            {second_svg(at.second)}
            {pin_svg()}
        }),
        Seconds::Hidden => None,
    };
    rsx! {
        div { class: "ds-clock-dial",
            {ticks_svg()}
            {numerals()}
            {hands_svg(at)}
            {second}
        }
    }
}

/// The time as digits.
#[component]
fn Digits(time: ClockTime) -> Element {
    rsx! {
        span { class: "ds-clock-digits", "{time.digits()}" }
    }
}
