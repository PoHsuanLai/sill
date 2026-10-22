//! The compositor's edge reveals, on the daemon's side: in full screen the pointer resting at the
//! top edge asks for the bar and at the bottom edge for the dock (`reveal(edge, shown)`), and
//! going away asks for them hidden. The note names no output, so a reveal is for the focused
//! window's output (the full-screen window; [`crate::daemon::compositor`] picks it) and a hide
//! is for every output. The surfaces follow [`EdgeLink`]; the compositor does the sliding.
//!
//! Still to come from the compositor's owner: a request by which sill tells it the bar's mode,
//! so that `desktop_only` and `never` keep the bar in full screen instead of asking for it.

use shell_host::OutputId;
use sill_model::desktop::{ScreenEdge, Shown};
use sill_ui_kit::links::{EdgeLink, EdgeState};

/// What a reveal note does to `link`: the top edge is the bar's, the bottom edge the dock's; the
/// sides carry nothing.
pub fn apply(link: &EdgeLink, edge: ScreenEdge, shown: Shown, on: Option<OutputId>) {
    let state = match (shown, on) {
        (Shown::Shown, Some(output)) => EdgeState::Revealed(output),
        (Shown::Shown, None) | (Shown::Hidden, _) => EdgeState::Away,
    };
    match edge {
        ScreenEdge::Top => link.set_top(state),
        ScreenEdge::Bottom => link.set_bottom(state),
        ScreenEdge::Left | ScreenEdge::Right => {}
    }
}

#[cfg(test)]
mod tests {
    use super::apply;
    use shell_host::OutputId;
    use sill_model::desktop::{ScreenEdge, Shown};
    use sill_ui_kit::links::{EdgeLink, EdgeState};

    #[test]
    fn each_edge_reaches_its_surface_and_a_hide_clears_it() {
        let link = EdgeLink::new();
        let output = Some(OutputId(3));
        // (edge, shown, top after, bottom after)
        let cases = [
            (
                ScreenEdge::Top,
                Shown::Shown,
                EdgeState::Revealed(OutputId(3)),
                EdgeState::Away,
            ),
            (
                ScreenEdge::Bottom,
                Shown::Shown,
                EdgeState::Revealed(OutputId(3)),
                EdgeState::Revealed(OutputId(3)),
            ),
            (
                ScreenEdge::Left,
                Shown::Hidden,
                EdgeState::Revealed(OutputId(3)),
                EdgeState::Revealed(OutputId(3)),
            ),
            (
                ScreenEdge::Top,
                Shown::Hidden,
                EdgeState::Away,
                EdgeState::Revealed(OutputId(3)),
            ),
            (
                ScreenEdge::Bottom,
                Shown::Hidden,
                EdgeState::Away,
                EdgeState::Away,
            ),
        ];
        for (edge, shown, top, bottom) in cases {
            apply(&link, edge, shown, output);
            let feed = link.latest();
            assert_eq!((feed.top, feed.bottom), (top, bottom), "{edge:?} {shown:?}");
        }
    }

    #[test]
    fn a_reveal_with_no_output_to_name_reveals_nothing() {
        let link = EdgeLink::new();
        apply(&link, ScreenEdge::Top, Shown::Shown, None);
        assert_eq!(link.latest().top, EdgeState::Away);
    }
}
