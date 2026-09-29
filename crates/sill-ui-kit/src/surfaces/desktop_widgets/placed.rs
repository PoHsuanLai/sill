//! One widget at its place on the layer, and its motion: while dragged it follows the pointer 1:1
//! and its card is lifted (quire's `Lift::Lifted`: the card's own scale, drag shadow and drag
//! layer); let go, it springs into its cell from where the hand let go at the hand's
//! velocity; a widget making room springs to its new cell, and back, from where it is at the
//! speed it has. The springs are quire's `SpringPointMotion` (one per axis, Reduced motion
//! inside, frames only while it moves); `glide` decides what each render asks of it.
//!
//! A card rises into its place when it first shows: quire's `Reveal` (design/26 Appear, the
//! `rise` at `--t-move --e-out`, once, never on a re-render; nothing under Reduced), so a widget
//! Edit Widgets adds arrives visibly, and the layer's first map shows its
//! widgets arriving as the reference's desktop does after a login.

use dioxus::core::queue_effect;
use dioxus::prelude::*;
use ds::Lift;
use ds::detail::{FirstShow, Reveal};
use ds::motion::{SpringPointMotion, SpringResponse, use_spring_point_motion};
use sill_settings::WidgetKind;

use super::arrange::LayerPoint;
use super::drag::corner;
use super::geometry::PlaceBox;
use super::glide::{Ask, Seen, ask, drawn, to_layer, to_point};
pub use super::glide::{Glide, Pose};
use crate::widgets::{CardPresence, Widget, WidgetHost};

/// `kind` at `target` (its cell's box), in `pose`; `glide` is how it was let go when
/// it is the widget just dropped. `Leaving`, it plays its exit where it stands and calls
/// `on_gone` once it has (`super::leaving`).
#[component]
pub fn Placed(
    kind: WidgetKind,
    target: PlaceBox,
    pose: Pose,
    glide: Glide,
    #[props(default)] presence: CardPresence,
    #[props(default)] on_gone: Option<EventHandler<()>>,
) -> Element {
    let at = use_glide(corner(&target), pose, glide);
    let style = place_style(at, &target);
    let lift = match pose {
        Pose::Lifted(_) => Lift::Lifted,
        Pose::Resting => Lift::Rest,
    };
    rsx! {
        div { class: "sill-dw-place", style,
            Reveal { first: FirstShow::Animate,
                Widget { kind, host: WidgetHost::Desktop, lift, presence, on_gone }
            }
            // While dragged, a press's release lands here and never on the widget's own controls,
            // which move along under the pointer.
            if lift == Lift::Lifted {
                div { class: "sill-dw-shield" }
            }
        }
    }
}

/// The inline box at `at`, `target`'s size.
pub fn place_style(at: LayerPoint, target: &PlaceBox) -> String {
    format!(
        "left:{}px;top:{}px;width:{}px;height:{}px;",
        at.x, at.y, target.width, target.height
    )
}

/// The corner to draw: the pointer's while lifted; else the spring's, which the render's ask
/// moves once this render is done.
fn use_glide(target: LayerPoint, pose: Pose, glide: Glide) -> LayerPoint {
    let motion = use_spring_point_motion(to_point(target));
    let mut seen = use_hook(|| CopyValue::new(Seen { target, pose }));
    let now = Seen { target, pose };
    let asked = ask(*seen.peek(), now, glide);
    seen.set(now);
    if asked != Ask::Nothing {
        queue_effect(move || apply(motion, asked));
    }
    drawn(pose, asked, to_layer(motion.frame().at))
}

/// Hand `asked` to the spring.
fn apply(motion: SpringPointMotion, asked: Ask) {
    match asked {
        Ask::Nothing => {}
        Ask::Hold(at) => motion.snap(to_point(at)),
        Ask::Drop {
            from,
            to,
            touch,
            release,
        } => {
            motion.snap(to_point(from));
            motion.go(to_point(to), touch, release, SpringResponse::Move);
        }
        Ask::Aside(to) => motion.go(
            to_point(to),
            ds::detail::Touch::Remote,
            ds::motion::Release::default(),
            SpringResponse::Move,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{LayerPoint, place_style};
    use crate::surfaces::desktop_widgets::geometry::PlaceBox;

    #[test]
    fn the_style_places() {
        let target = PlaceBox {
            left: 0,
            top: 0,
            width: 164,
            height: 164,
        };
        let at = LayerPoint { x: -3, y: 40 };
        assert_eq!(
            place_style(at, &target),
            "left:-3px;top:40px;width:164px;height:164px;"
        );
    }
}
