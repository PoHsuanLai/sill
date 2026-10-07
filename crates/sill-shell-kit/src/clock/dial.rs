//! The analog clock's parts (design/23-WIDGETS.md sections 2 and 4.2; design/04-COMPONENTS.md
//! "Widgets"), at the reference dial's measured proportions on a 100-unit
//! dial: sixty fine ticks (every fifth in full ink, the rest at .3), twelve numerals, the hour
//! hand to .56 of the radius and the minute hand to .9, each a thin neck from the hub that
//! widens a seventh of the way out, a black hub, and the seconds hand in its own colour from a
//! fifth behind the hub to .96 with a ring round a pin of the face's colour.
//!
//! CSS does not reach inside an SVG on Blitz (spike S6), so every paint is an attribute on
//! `currentColor` and each vector takes its colour from its own element. The numerals are text
//! placed by percentages, so they are set in the display face. `data-ds-svg` tells the markup
//! lint the vectors are quire's own.

use crate::clock::angles::{Hands, Tenths};
use crate::clock::kind::DayPhase;
use dioxus::prelude::*;

/// `rotate(a 50 50)`: a turn about the dial's centre in the 100-unit box.
fn turn(angle: Tenths) -> String {
    format!("rotate({} 50 50)", angle.css())
}

/// The hands' thick width and their neck's, in the 100-unit box.
const HAND: &str = "3.6";
const NECK: &str = "1.7";

/// Where the neck ends and the thick part begins: a seventh of the radius out.
const NECK_END: &str = "43";

/// Where numeral `n` (1 to 12) is centred, as `left:x%;top:y%` on a box whose half-width is
/// the numerals' radius.
pub(crate) fn numeral_place(n: u16) -> String {
    let radians = (f32::from(n % 12) * 30.0).to_radians();
    let x = hundredths(50.0 + 50.0 * radians.sin());
    let y = hundredths(50.0 - 50.0 * radians.cos());
    format!("left:{x:.2}%;top:{y:.2}%")
}

/// `value` rounded to hundredths, with no negative zero (`-0.00` would read as a sign).
fn hundredths(value: f32) -> f32 {
    (value * 100.0).round() / 100.0 + 0.0
}

/// The twelve numerals.
pub(crate) fn numerals() -> Element {
    rsx! {
        div { class: "ds-clock-numerals", "aria-hidden": "true",
            for n in 1..=12u16 {
                span { key: "{n}", class: "ds-clock-numeral", style: numeral_place(n), "{n}" }
            }
        }
    }
}

/// The sixty ticks: every fifth in full ink, the others at .3.
pub(crate) fn ticks_svg() -> Element {
    rsx! {
        svg { class: "ds-clock-ticks", "data-ds-svg": "clock", view_box: "0 0 100 100", "aria-hidden": "true",
            for minute in 0..60u16 {
                line {
                    key: "{minute}",
                    x1: "50",
                    y1: "2",
                    x2: "50",
                    y2: "5.8",
                    stroke: "currentColor",
                    "stroke-width": "1",
                    opacity: if minute % 5 == 0 { "1" } else { ".3" },
                    transform: turn(Tenths(minute * 60)),
                }
            }
        }
    }
}

/// One hand: its neck, then its thick part out to `tip`, turned to `angle`.
fn hand(angle: Tenths, tip: &'static str) -> Element {
    rsx! {
        g { transform: turn(angle),
            line { x1: "50", y1: "50", x2: "50", y2: NECK_END, stroke: "currentColor", "stroke-width": NECK, "stroke-linecap": "round" }
            line { x1: "50", y1: NECK_END, x2: "50", y2: tip, stroke: "currentColor", "stroke-width": HAND, "stroke-linecap": "round" }
        }
    }
}

/// The hour and minute hands and the hub.
pub(crate) fn hands_svg(hands: Hands) -> Element {
    rsx! {
        svg { class: "ds-clock-hands", "data-ds-svg": "clock", view_box: "0 0 100 100", "aria-hidden": "true",
            {hand(hands.hour, "23.8")}
            {hand(hands.minute, "6.8")}
            circle { cx: "50", cy: "50", r: "2.7", fill: "currentColor" }
        }
    }
}

/// The seconds hand and its ring, over the others.
pub(crate) fn second_svg(angle: Tenths) -> Element {
    rsx! {
        svg { class: "ds-clock-second", "data-ds-svg": "clock", view_box: "0 0 100 100", "aria-hidden": "true",
            line {
                x1: "50",
                y1: "60",
                x2: "50",
                y2: "2",
                stroke: "currentColor",
                "stroke-width": "1.2",
                "stroke-linecap": "round",
                transform: turn(angle),
            }
            circle { cx: "50", cy: "50", r: "1.8", fill: "currentColor" }
        }
    }
}

/// The pin in the seconds hand's ring, of the face's colour.
pub(crate) fn pin_svg() -> Element {
    rsx! {
        svg { class: "ds-clock-pin", "data-ds-svg": "clock", view_box: "0 0 100 100", "aria-hidden": "true",
            circle { cx: "50", cy: "50", r: ".9", fill: "currentColor" }
        }
    }
}

/// The day or night mark beside a digital time: the sun a disc, the moon a crescent (a disc
/// with a second disc taken out of it).
pub(crate) fn phase_mark(phase: DayPhase) -> Element {
    let (class, shape) = match phase {
        DayPhase::Day => (
            "ds-clock-sun",
            rsx! { circle { cx: "12", cy: "12", r: "6.5", fill: "currentColor" } },
        ),
        DayPhase::Night => (
            "ds-clock-moon",
            rsx! { path { d: "M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z", fill: "currentColor" } },
        ),
    };
    rsx! {
        svg { class, "data-ds-svg": "clock", view_box: "0 0 24 24", "aria-hidden": "true", {shape} }
    }
}

#[cfg(test)]
mod tests {
    use super::numeral_place;

    #[test]
    fn numerals_stand_round_the_dial() {
        let cases = [
            (12, "left:50.00%;top:0.00%"),
            (3, "left:100.00%;top:50.00%"),
            (6, "left:50.00%;top:100.00%"),
            (9, "left:0.00%;top:50.00%"),
            (1, "left:75.00%;top:6.70%"),
        ];
        for (n, want) in cases {
            assert_eq!(numeral_place(n), want, "{n}");
        }
    }
}
