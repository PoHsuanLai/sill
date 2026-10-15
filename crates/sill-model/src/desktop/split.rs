//! What the Split View picker shows: the windows the compositor offered for the other half, as
//! cards the picker can draw, and the output the first window is on. Pure.

use super::DesktopView;

/// One window on offer for the other half of a Split View.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitCard {
    /// The window's identifier, as [`super::DesktopCommand::PickSplit`] quotes it.
    pub window: String,
    /// The [`super::AppView::key`] of its app, for the icon.
    pub app: String,
    /// The words under the card: the window's title, else its app's key.
    pub title: String,
}

/// A card for each of `candidates` the desktop's view still has, in the order offered. A window
/// the view does not know (it closed, or the view is older than the offer) has no card.
///
/// ```
/// use sill_model::desktop::{DesktopView, cards_of};
/// assert!(cards_of(&DesktopView::default(), &["w1".to_owned()]).is_empty());
/// ```
pub fn cards_of(view: &DesktopView, candidates: &[String]) -> Vec<SplitCard> {
    candidates
        .iter()
        .filter_map(|id| view.windows.iter().find(|window| &window.identifier == id))
        .map(|window| SplitCard {
            window: window.identifier.clone(),
            app: window.app.clone(),
            title: match window.title.trim() {
                "" => window.app.clone(),
                title => title.to_owned(),
            },
        })
        .collect()
}

/// The name of the output the window `first` is on; empty when the view does not know the window
/// or the desktop does not say.
///
/// ```
/// use sill_model::desktop::{DesktopView, output_of};
/// assert_eq!(output_of(&DesktopView::default(), "w1"), "");
/// ```
pub fn output_of(view: &DesktopView, first: &str) -> String {
    view.windows
        .iter()
        .find(|window| window.identifier == first)
        .map(|window| window.output.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use casement_protocols::vocab::{
        Emphasis, LeaseState, Modality, PlacementMark, Responsiveness, SensitivityLevel,
        WindowRole, WindowStateKind,
    };

    use super::{SplitCard, cards_of, output_of};
    use crate::desktop::{DesktopView, SpacePlaceView, WindowView};

    fn window(id: &str, app: &str, title: &str, output: &str) -> WindowView {
        WindowView {
            identifier: id.to_owned(),
            app: app.to_owned(),
            title: title.to_owned(),
            state: WindowStateKind::Floating,
            space: SpacePlaceView::Unknown,
            output: output.to_owned(),
            emphasis: Emphasis::Inactive,
            placement: PlacementMark::Free,
            role: WindowRole::Document,
            modality: Modality::None,
            geometry: None,
            responsiveness: Responsiveness::Responding,
            lease: LeaseState::Free,
            sensitivity: SensitivityLevel::Normal,
        }
    }

    fn view() -> DesktopView {
        DesktopView {
            windows: vec![
                window("w1", "term", "shell", "DP-1"),
                window("w2", "mail", "Inbox", "DP-1"),
                window("w3", "files", "  ", "DP-2"),
            ],
            ..DesktopView::default()
        }
    }

    fn card(window: &str, app: &str, title: &str) -> SplitCard {
        SplitCard {
            window: window.to_owned(),
            app: app.to_owned(),
            title: title.to_owned(),
        }
    }

    /// (case, the candidates offered, the cards drawn).
    type CardCase = (&'static str, &'static [&'static str], Vec<SplitCard>);

    #[test]
    fn cards_follow_the_offer_and_skip_windows_the_view_lost() {
        let cases: [CardCase; 4] = [
            ("none offered", &[], vec![]),
            (
                "in the order offered",
                &["w2", "w1"],
                vec![card("w2", "mail", "Inbox"), card("w1", "term", "shell")],
            ),
            (
                "a blank title is the app's key",
                &["w3"],
                vec![card("w3", "files", "files")],
            ),
            (
                "a window the view lost has no card",
                &["w9", "w1"],
                vec![card("w1", "term", "shell")],
            ),
        ];
        for (name, offered, want) in cases {
            let offered: Vec<String> = offered.iter().map(|id| (*id).to_owned()).collect();
            assert_eq!(cards_of(&view(), &offered), want, "{name}");
        }
    }

    #[test]
    fn the_first_windows_output_is_its_own_or_empty() {
        let cases = [("w1", "DP-1"), ("w3", "DP-2"), ("w9", "")];
        for (first, want) in cases {
            assert_eq!(output_of(&view(), first), want, "{first}");
        }
    }
}
