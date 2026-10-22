//! `use_edge_reveal`: the bar and the dock follow the compositor's reveal of their edge in full
//! screen. The compositor slides the surface itself; the follower only hears that its content
//! is to be drawn and hit-testable (`Shown`) or put back (`Hidden`).

use dioxus::prelude::*;
use shell_host::OutputId;
use sill_model::desktop::Shown;

use crate::links::{EdgeLink, RevealEdge};

/// Call `reveal` with the state of `edge` on `output` each time it changes (first `Shown`, then
/// `Hidden`; nothing at start). A root without the link gets no calls, which is a session
/// without the desktop's compositor.
pub fn use_edge_reveal(edge: RevealEdge, output: OutputId, reveal: Callback<Shown>) {
    let link = try_use_context::<EdgeLink>();
    use_hook(move || {
        let Some(link) = link else {
            return;
        };
        let mut feed = link.subscribe();
        spawn(async move {
            let mut last = Shown::Hidden;
            loop {
                let now = feed.borrow_and_update().at(edge).on(output);
                if now != last {
                    last = now;
                    reveal.call(now);
                }
                if feed.changed().await.is_err() {
                    return;
                }
            }
        });
    });
}
