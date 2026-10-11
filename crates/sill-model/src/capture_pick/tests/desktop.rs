use casement_protocols::vocab::{
    Emphasis, LeaseState, Modality, PlacementMark, Responsiveness, SensitivityLevel, WindowRole,
    WindowStateKind,
};

use super::super::*;
use crate::desktop::{DesktopView, SpacePlaceView, WindowView};

fn window(id: &str, title: &str) -> WindowView {
    WindowView {
        identifier: id.to_owned(),
        app: "Notes".to_owned(),
        title: title.to_owned(),
        state: WindowStateKind::Floating,
        space: SpacePlaceView::Unknown,
        output: String::new(),
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

#[test]
fn only_showing_ordinary_documents_are_offered_by_their_identifiers() {
    let minimised = WindowView {
        state: WindowStateKind::Minimised,
        ..window("w-2", "Hidden")
    };
    let hidden = WindowView {
        sensitivity: SensitivityLevel::CaptureHidden,
        ..window("w-3", "Bank")
    };
    let view = DesktopView {
        windows: vec![window("w-1", "Todo"), minimised, hidden, window("w-4", "")],
        ..DesktopView::default()
    };
    let got: Vec<(String, String)> = windows_of(&view)
        .into_iter()
        .map(|w| (w.identifier, w.title))
        .collect();
    assert_eq!(
        got,
        vec![
            ("w-1".to_owned(), "Todo".to_owned()),
            ("w-4".to_owned(), "Notes".to_owned())
        ]
    );
}
