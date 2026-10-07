//! One row of a `BannerStack`: the card in a `div.ds-banner` that plays the row's
//! exit and heal (`panel-out`, `heal`), with the entrance (`panel-in`) on the `div.ds-banner-card`
//! inside it, and the height it measures, which is how far the rows after it heal when it
//! leaves. The gap to the next banner is the row's own padding, so the measured height is the
//! whole pitch.
//!
//! The entrance is the inner element's, played once as it mounts, rather than a rule on the
//! row's `data-presence=entering`: Blitz keeps the last animated value when an animation is
//! taken off an element, so a presence that turned `present` before a frame had been resolved
//! past the entrance's end (a loaded machine, a snapshot's clock) froze the row mid-slide.
//!
//! The row marks its card `Carried`: a card swiped away reports at once and the row's own exit
//! flies it on from where the hand left it.

use crate::notifications::banner_stack::{BannerKey, BannerPosition};
use crate::notifications::swipe::Carried;
use dioxus::core::current_scope_id;
use dioxus::prelude::*;
use ds::base::geometry::units::Px;
use ds::base::word::Word;
use ds::host::measure::client_rect;
use ds::motion::presence::Presence;
use ds::motion::roster::{Heal, RowPitch, presence_slug};
use ds::motion::use_roster::Pitches;
use ds::style::task::spawn_in;
use std::rc::Rc;

/// One banner in the stack.
#[component]
pub(crate) fn BannerRow(
    banner: BannerKey,
    presence: Presence,
    heal: Option<Heal>,
    position: BannerPosition,
    pitches: Pitches<BannerKey>,
    card: Element,
) -> Element {
    let mut element = use_signal(|| None::<Rc<MountedData>>);
    let mut leaving_seen = use_hook(|| CopyValue::new(Seen::No));
    let scope = use_hook(current_scope_id);
    use_context_provider(|| Carried);
    // A task of the row's own scope: the read also runs from an effect, which has none.
    let measure = move || {
        if let Some(mounted) = element.peek().clone() {
            spawn_in(scope, async move {
                if let Some(rect) = client_rect(&mounted).await {
                    pitches.set(banner, RowPitch(rect.size.height));
                }
            });
        }
    };
    // Measured again as it starts to leave: hover may have opened it since it arrived.
    if matches!(presence, Presence::Leaving(_)) && *leaving_seen.peek() == Seen::No {
        leaving_seen.set(Seen::Yes);
        dioxus::core::queue_effect(measure);
    }
    rsx! {
        div {
            class: "ds-banner",
            "data-banner": "{banner.0}",
            "data-presence": presence_slug(presence, heal),
            "data-exit": exit_slug(presence),
            style: heal_style(heal, position),
            onmounted: move |event| {
                element.set(Some(event.data()));
                measure();
            },
            div { class: "ds-banner-card", {card} }
        }
    }
}

/// Whether the row has been measured for its exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Seen {
    No,
    Yes,
}

/// `data-exit` while leaving.
fn exit_slug(presence: Presence) -> Option<&'static str> {
    match presence {
        Presence::Leaving(exit) => Some(exit.slug()),
        Presence::Hidden | Presence::Entering | Presence::Present => None,
    }
}

/// A healing row's distance: `--dy` signed for the stack's direction.
fn heal_style(heal: Option<Heal>, position: BannerPosition) -> Option<String> {
    heal.map(|Heal { dy }| format!("--dy:{}px", Px(dy.0 * position.heal_sign()).0))
}

#[cfg(test)]
mod tests {
    use super::heal_style;
    use crate::notifications::banner_stack::BannerPosition;
    use ds::base::geometry::units::Px;
    use ds::motion::roster::Heal;

    #[test]
    fn a_healing_row_moves_towards_the_gap() {
        let healing = Heal { dy: Px(96.0) };
        let cases = [
            (Some(healing), BannerPosition::TopRight, Some("--dy:96px")),
            (
                Some(healing),
                BannerPosition::BottomRight,
                Some("--dy:-96px"),
            ),
            (None, BannerPosition::TopRight, None),
        ];
        for (heal, position, want) in cases {
            assert_eq!(
                heal_style(heal, position).as_deref(),
                want,
                "{heal:?} {position:?}"
            );
        }
    }
}
