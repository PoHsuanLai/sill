//! The compositor's edge reveals, on the daemon's side: in full screen the pointer resting at the
//! top edge asks for that output's bar and at the bottom edge for its dock, and going away asks
//! for them hidden. A version 7 compositor names the output; an older one does not, and then a
//! reveal is for the focused window's output and a hide for every output. The surfaces follow
//! [`EdgeLink`]; the compositor does the sliding.
//!
//! The daemon also tells the compositor what the bar does in full screen and how much motion the
//! person allows ([`FullScreenPrefs`]), and writes which outputs show a full-screen Space, which
//! keeps a `desktop_only` bar up there.

use std::collections::BTreeSet;

use shell_host::OutputId;
use sill_model::desktop::{
    DesktopCommand, DesktopView, FullScreenBar, Motion as WireMotion, ScreenEdge, Shown, SpaceKind,
};
use sill_settings::MenuBarHiding;
use sill_ui_kit::links::{EdgeLink, FullScreens, RevealEdge};

/// Where a reveal note lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Landing {
    /// On this output.
    On(OutputId),
    /// On every output (a hide that names none).
    Everywhere,
    /// Nowhere: an output that is not connected, or none to pick.
    Nowhere,
}

/// Where a note lands: the output it names, `named` (`None` when not connected); else for a reveal
/// the `focused` window's output and for a hide every output.
pub fn landing(
    named: Option<Option<OutputId>>,
    shown: Shown,
    focused: Option<OutputId>,
) -> Landing {
    let on = |output: Option<OutputId>| output.map_or(Landing::Nowhere, Landing::On);
    match (named, shown) {
        (Some(output), _) => on(output),
        (None, Shown::Shown) => on(focused),
        (None, Shown::Hidden) => Landing::Everywhere,
    }
}

/// What a reveal note does to `link`: the top edge is the bar's, the bottom edge the dock's; the
/// sides carry nothing.
pub fn apply(link: &EdgeLink, edge: ScreenEdge, shown: Shown, landing: Landing) {
    let edge = match edge {
        ScreenEdge::Top => RevealEdge::Top,
        ScreenEdge::Bottom => RevealEdge::Bottom,
        ScreenEdge::Left | ScreenEdge::Right => return,
    };
    match landing {
        Landing::On(output) => link.set(edge, output, shown),
        Landing::Everywhere => link.clear(edge),
        Landing::Nowhere => {}
    }
}

/// The outputs whose shown Space is full screen (one window, or two side by side), by `id` of
/// the output's name.
pub fn full_screens(view: &DesktopView, id: impl Fn(&str) -> Option<OutputId>) -> FullScreens {
    let outputs: BTreeSet<OutputId> = view
        .spaces
        .iter()
        .filter(|space| space.shown == Shown::Shown)
        .filter(|space| matches!(space.kind, SpaceKind::FullScreen | SpaceKind::SplitView))
        .filter_map(|space| id(&space.output))
        .collect();
    FullScreens(outputs)
}

/// What the compositor is told about full screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FullScreenPrefs {
    /// Whether the bar slides in at the top edge or stays up.
    pub bar: FullScreenBar,
    /// Slides, or cross-fades.
    pub motion: WireMotion,
}

impl FullScreenPrefs {
    /// The prefs of the bar's `hiding` mode and the `motion` allowed
    /// (`sill_ui_kit::shell_motion`): the modes that keep the bar in full screen
    /// (`desktop_only`, `never`) keep it there.
    pub fn of(hiding: MenuBarHiding, motion: WireMotion) -> FullScreenPrefs {
        FullScreenPrefs {
            bar: match hiding {
                MenuBarHiding::Always | MenuBarHiding::FullScreenOnly => FullScreenBar::Reveal,
                MenuBarHiding::DesktopOnly | MenuBarHiding::Never => FullScreenBar::Keep,
            },
            motion,
        }
    }

    /// The commands that bring the compositor from `sent` (nothing yet: `None`) to these prefs.
    pub fn since(self, sent: Option<FullScreenPrefs>) -> Vec<DesktopCommand> {
        let bar = (sent.map(|sent| sent.bar) != Some(self.bar))
            .then_some(DesktopCommand::SetFullScreenBar(self.bar));
        let motion = (sent.map(|sent| sent.motion) != Some(self.motion))
            .then_some(DesktopCommand::SetMotion(self.motion));
        bar.into_iter().chain(motion).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{FullScreenPrefs, Landing, apply, full_screens, landing};
    use porter_core::SpaceId;
    use shell_host::OutputId;
    use sill_model::desktop::{
        DesktopCommand, DesktopView, FullScreenBar, Motion as WireMotion, ScreenEdge, Shown,
        SpaceKind, SpaceView,
    };
    use sill_settings::MenuBarHiding;
    use sill_ui_kit::links::{EdgeLink, RevealEdge};

    #[test]
    fn a_named_output_is_used_and_an_unnamed_note_picks_focus_or_everywhere() {
        let (a, b) = (OutputId(1), OutputId(2));
        // (named, shown, focused, landing)
        let cases = [
            (Some(Some(b)), Shown::Shown, Some(a), Landing::On(b)),
            (Some(Some(b)), Shown::Hidden, Some(a), Landing::On(b)),
            (Some(None), Shown::Shown, Some(a), Landing::Nowhere),
            (None, Shown::Shown, Some(a), Landing::On(a)),
            (None, Shown::Shown, None, Landing::Nowhere),
            (None, Shown::Hidden, Some(a), Landing::Everywhere),
        ];
        for (named, shown, focused, want) in cases {
            assert_eq!(landing(named, shown, focused), want, "{named:?} {shown:?}");
        }
    }

    #[test]
    fn each_edge_reaches_its_surface_on_its_output() {
        let link = EdgeLink::new();
        let (a, b) = (OutputId(1), OutputId(2));
        apply(&link, ScreenEdge::Top, Shown::Shown, Landing::On(a));
        apply(&link, ScreenEdge::Bottom, Shown::Shown, Landing::On(b));
        apply(&link, ScreenEdge::Left, Shown::Shown, Landing::On(a));
        let feed = link.latest();
        assert_eq!(
            (feed.top.on(a), feed.top.on(b), feed.bottom.on(b)),
            (Shown::Shown, Shown::Hidden, Shown::Shown)
        );
        apply(&link, ScreenEdge::Top, Shown::Hidden, Landing::On(b));
        assert_eq!(link.latest().at(RevealEdge::Top).on(a), Shown::Shown);
        apply(&link, ScreenEdge::Top, Shown::Hidden, Landing::Everywhere);
        apply(&link, ScreenEdge::Bottom, Shown::Hidden, Landing::Nowhere);
        let feed = link.latest();
        assert_eq!(
            (feed.top.on(a), feed.bottom.on(b)),
            (Shown::Hidden, Shown::Shown)
        );
    }

    #[test]
    fn an_output_showing_a_full_screen_space_is_in_full_screen() {
        let space = |id: &str, kind, output: &str, shown| SpaceView {
            id: SpaceId::parse(id).expect("id"),
            kind,
            output: output.to_owned(),
            position: 0,
            shown,
        };
        let view = DesktopView {
            spaces: vec![
                space("ws-1", SpaceKind::Desktop, "DP-1", Shown::Hidden),
                space("ws-2", SpaceKind::FullScreen, "DP-1", Shown::Shown),
                space("ws-3", SpaceKind::Desktop, "DP-2", Shown::Shown),
                space("ws-4", SpaceKind::SplitView, "DP-3", Shown::Shown),
                space("ws-5", SpaceKind::FullScreen, "DP-2", Shown::Hidden),
                space("ws-6", SpaceKind::FullScreen, "HDMI-9", Shown::Shown),
            ],
            ..DesktopView::default()
        };
        let id = |name: &str| match name {
            "DP-1" => Some(OutputId(1)),
            "DP-2" => Some(OutputId(2)),
            "DP-3" => Some(OutputId(3)),
            _ => None,
        };
        let full = full_screens(&view, id);
        assert_eq!(
            [1, 2, 3].map(|n| full.on(OutputId(n))),
            [Shown::Shown, Shown::Hidden, Shown::Shown]
        );
    }

    #[test]
    fn the_bar_modes_become_the_compositor_prefs_sent_once() {
        use FullScreenBar::{Keep, Reveal};
        use MenuBarHiding::{Always, DesktopOnly, FullScreenOnly, Never};
        // (mode, motion, bar)
        let cases = [
            (Always, WireMotion::Full, Reveal),
            (FullScreenOnly, WireMotion::Reduced, Reveal),
            (DesktopOnly, WireMotion::Reduced, Keep),
            (Never, WireMotion::Full, Keep),
        ];
        for (mode, motion, bar) in cases {
            let prefs = FullScreenPrefs::of(mode, motion);
            assert_eq!(prefs, FullScreenPrefs { bar, motion }, "{mode:?}");
            assert_eq!(
                prefs.since(None),
                vec![
                    DesktopCommand::SetFullScreenBar(bar),
                    DesktopCommand::SetMotion(motion)
                ],
                "{mode:?}"
            );
            assert_eq!(prefs.since(Some(prefs)), vec![], "{mode:?}");
        }
        let keep = FullScreenPrefs::of(Never, WireMotion::Full);
        let reveal = FullScreenPrefs::of(Always, WireMotion::Full);
        assert_eq!(
            keep.since(Some(reveal)),
            vec![DesktopCommand::SetFullScreenBar(Keep)]
        );
    }
}
