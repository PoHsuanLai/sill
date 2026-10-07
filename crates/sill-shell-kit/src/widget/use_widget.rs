//! `use_widget`: the entry a widget shows now, from its provider's timeline (design/23-WIDGETS.md
//! section 9.2). The hook reads the design system's clock, so on a test's virtual clock the
//! entries change exactly when the test advances past their dates. It wakes only for what the
//! timeline holds (the next entry's date, the refresh), as the [`Follower`] machine's wake, and
//! sleeps on nothing else: a widget whose provider pushes one entry at a time costs no frame
//! between pushes (the idle-frame rule).

use crate::widget::contract::{Widget, fit};
use crate::widget::follow::{FollowIn, Follower};
use crate::widget::kind::WidgetSize;
use crate::widget::timeline::{RefreshAsk, Timeline};
use dioxus::prelude::*;
use ds::motion::machine::{use_machine_in, use_machine_state};

/// The entry `W` shows now from `timeline` at `size`: the timeline's current entry, or `W`'s
/// placeholder before its first. On each entry's date the caller re-renders with the next; when
/// the refresh policy comes due, `onrefresh` is asked once, and the provider answers with a new
/// timeline (an equal timeline passed again changes nothing).
pub fn use_widget<W: Widget>(
    timeline: &Timeline<W::Entry>,
    size: WidgetSize,
    onrefresh: Option<EventHandler<RefreshAsk>>,
) -> W::Entry {
    let held = use_machine_state(|_| Follower::<W::Entry>::idle());
    let clock = held.clock;
    let follower = use_machine_in(
        held,
        (),
        move || clock,
        move |ask, _| {
            if let Some(handler) = onrefresh {
                handler.call(ask);
            }
        },
    );
    if !follower.state().peek().follows(timeline) {
        follower.send_from_render(FollowIn::Arrive(timeline.clone()));
    }
    // Read so that each wake, which changes the state, re-renders the caller.
    let _ = follower.state().read();
    timeline
        .current(ds::base::time::clock::now())
        .cloned()
        .unwrap_or_else(|| W::placeholder(fit::<W>(size)))
}
