//! One widget at its place on the layer, and its motion (FINDINGS "Widget drag"): while dragged
//! it follows the pointer 1:1, lifted (a little larger over quire's drag shadow, `--shadow-drag`,
//! above the others on `--z-drag`); at rest it glides to its cell whenever its cell changes, so
//! the others make room and the dropped one settles into the grid. The motion is quire's
//! `use_tween` (its duration and easing tokens, Reduced motion inside, frames only while it
//! moves): the lift over `--t-quick` at `--e-out`, a glide over `--t-move`, at `--e-spring` for
//! the widget the person dropped and `--e-out` for the ones that made room (a spring is for
//! contact, design/26 R5).

use dioxus::prelude::*;
use ds::detail::{Ease, TweenSpec, use_tween};
use ds::{DurationToken, Fraction};
use sill_settings::WidgetKind;

use super::arrange::LayerPoint;
use super::drag::corner;
use super::geometry::PlaceBox;
use crate::widgets::{Widget, WidgetHost};

/// How much larger a lifted widget is drawn, in thousandths: 1.04, between quire's drag ghost
/// (1.02) and its drop target (1.045), design/04 section 34.
pub const LIFT_PERMILLE: u32 = 40;

/// Where the widget is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    /// At its cell (gliding there when the cell changed).
    Resting,
    /// Picked up, its corner here.
    Lifted(LayerPoint),
}

/// `kind` at `target` (its cell's box), in `pose`; `ease` is the curve of its next glide.
#[component]
pub fn Placed(kind: WidgetKind, target: PlaceBox, pose: Pose, ease: Ease) -> Element {
    let raised = match pose {
        Pose::Lifted(_) => Fraction(1000),
        Pose::Resting => Fraction(0),
    };
    let lift = use_tween(
        raised,
        TweenSpec {
            duration: DurationToken::Quick,
            ease: Ease::Out,
        },
    );
    let at = use_glide(corner(&target), pose, ease);
    let scale = 1000 + LIFT_PERMILLE * u32::from(lift.now().0) / 1000;
    let style = place_style(at, &target, scale);
    let (state, shield) = match pose {
        Pose::Lifted(_) => ("up", Shield::On),
        Pose::Resting => ("down", Shield::Off),
    };
    rsx! {
        div { class: "sill-dw-place", "data-lift": state, style,
            Widget { kind, host: WidgetHost::Desktop }
            // While dragged, a press's release lands here and never on the widget's own controls,
            // which move along under the pointer.
            if shield == Shield::On {
                div { class: "sill-dw-shield" }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shield {
    On,
    Off,
}

/// The inline box at `at`, `target`'s size, scaled by `scale` thousandths.
pub fn place_style(at: LayerPoint, target: &PlaceBox, scale: u32) -> String {
    let transform = match scale {
        1000 => String::new(),
        _ => format!("transform:scale({}.{:03});", scale / 1000, scale % 1000),
    };
    format!(
        "left:{}px;top:{}px;width:{}px;height:{}px;{transform}",
        at.x, at.y, target.width, target.height
    )
}

/// Which way the glide's tween runs next: each new cell flips it, so every move is a new run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leg {
    Out,
    Back,
}

impl Leg {
    fn next(self) -> Leg {
        match self {
            Leg::Out => Leg::Back,
            Leg::Back => Leg::Out,
        }
    }

    fn share(self) -> Fraction {
        match self {
            Leg::Out => Fraction(1000),
            Leg::Back => Fraction(0),
        }
    }
}

/// What this render draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Glide {
    /// Held under the pointer.
    Held(LayerPoint),
    /// A new glide asked this render: still where it was.
    Starting,
    /// On its way (or arrived).
    Going,
}

/// The corner to draw: the pointer's while lifted; else gliding from wherever it was drawn to
/// `target` each time `target` changes.
fn use_glide(target: LayerPoint, pose: Pose, ease: Ease) -> LayerPoint {
    let mut from = use_hook(|| CopyValue::new(target));
    let mut to = use_hook(|| CopyValue::new(target));
    let mut drawn = use_hook(|| CopyValue::new(target));
    let mut leg = use_hook(|| CopyValue::new(Leg::Out));
    let glide = match pose {
        Pose::Lifted(held) => {
            from.set(held);
            to.set(held);
            Glide::Held(held)
        }
        Pose::Resting if target != *to.peek() => {
            from.set(*drawn.peek());
            to.set(target);
            let next = leg.peek().next();
            leg.set(next);
            Glide::Starting
        }
        Pose::Resting => Glide::Going,
    };
    let spec = TweenSpec {
        duration: DurationToken::Move,
        ease,
    };
    let tween = use_tween(leg.peek().share(), spec);
    let now = match glide {
        Glide::Held(held) => held,
        Glide::Starting => *from.peek(),
        Glide::Going if tween.landed() => *to.peek(),
        Glide::Going => between(*from.peek(), *to.peek(), tween.progress()),
    };
    drawn.set(now);
    now
}

/// `share` thousandths of the way from `a` to `b` (past `b` on a spring's overshoot).
pub fn between(a: LayerPoint, b: LayerPoint, share: Fraction) -> LayerPoint {
    let part = |from: i32, to: i32| {
        let span = i64::from(to) - i64::from(from);
        let step = (span * i64::from(share.0) + span.signum() * 500) / 1000;
        i32::try_from(i64::from(from) + step).unwrap_or(to)
    };
    LayerPoint {
        x: part(a.x, b.x),
        y: part(a.y, b.y),
    }
}

#[cfg(test)]
mod tests {
    use ds::Fraction;

    use super::{LayerPoint, between, place_style};
    use crate::surfaces::desktop_widgets::geometry::PlaceBox;

    #[test]
    fn a_glide_mixes_the_two_corners() {
        let a = LayerPoint { x: 100, y: 16 };
        let b = LayerPoint { x: 820, y: 196 };
        let cases = [
            (0, a),
            (500, LayerPoint { x: 460, y: 106 }),
            (1000, b),
            (1100, LayerPoint { x: 892, y: 214 }),
        ];
        for (share, want) in cases {
            assert_eq!(between(a, b, Fraction(share)), want, "{share}");
        }
    }

    #[test]
    fn the_style_places_and_scales() {
        let target = PlaceBox {
            left: 0,
            top: 0,
            width: 164,
            height: 164,
        };
        let at = LayerPoint { x: -3, y: 40 };
        assert_eq!(
            place_style(at, &target, 1000),
            "left:-3px;top:40px;width:164px;height:164px;"
        );
        assert_eq!(
            place_style(at, &target, 1040),
            "left:-3px;top:40px;width:164px;height:164px;transform:scale(1.040);"
        );
    }
}
