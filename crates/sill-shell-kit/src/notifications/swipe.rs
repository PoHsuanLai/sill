//! A notification card's swipe to dismiss: whether the card takes one, and how its flight ends,
//! over the swipe glue a toast shares (`ds::components::overlays::swipe_glue::SwipeGlue`).
//!
//! When a swipe ends past a threshold the card flies out to the right from where it is. Alone,
//! it plays `panel-out` itself and reports `on_dismiss` at `settle(PanelOut)`, so a caller that
//! unmounts it there never cuts the flight short. Inside a `BannerStack` the stack's row carries
//! the flight: the card holds its offset, reports at once, and the caller's removal of it makes
//! the row slide out from that offset (the two transforms compose), then the rows below heal.

use dioxus::prelude::*;
use ds::components::overlays::swipe_glue::{SwipeGlue, use_swipe_glue};
use ds::motion::anim::Anim;
use ds::motion::swipe::SwipeMetrics;
use ds::motion::timer::use_motion_timer;

/// Whether a card can be swiped away, and who hears it.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum NotificationSwipe {
    /// It cannot (a row of the center that the caller dismisses otherwise).
    #[default]
    Off,
    /// A drag or a horizontal scroll to the right dismisses it; the handler hears when the card
    /// has gone (`notifications.swipe` says whether the caller keeps it in the center).
    Dismiss(EventHandler<()>),
}

/// Marks a card as carried by a `BannerStack` row, whose own exit flies it out: the card then
/// reports its dismissal at once and holds its offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Carried;

/// The card's swipe: the shared glue, on when the caller listens. Hooks run whether or not the
/// swipe is on, so the card's hook order never changes with its props.
pub(crate) fn use_card_swipe(swipe: &NotificationSwipe, metrics: SwipeMetrics) -> SwipeGlue {
    let flight = use_motion_timer(Anim::PanelOut);
    let carried = try_use_context::<Carried>();
    let heard = match swipe {
        NotificationSwipe::Dismiss(handler) => Some(*handler),
        NotificationSwipe::Off => None,
    };
    let on_dismiss = EventHandler::new(move |()| {
        let Some(heard) = heard else { return };
        match carried {
            Some(Carried) => heard.call(()),
            None => flight.start(EventHandler::new(move |()| heard.call(()))),
        }
    });
    let on = heard.is_some();
    use_swipe_glue(on, metrics, on_dismiss)
}
