//! The windows that may be offered, from the desktop's view of itself.

use casement_protocols::vocab::{SensitivityLevel, WindowRole, WindowStateKind};

use super::plan::WindowCandidate;
use crate::desktop::DesktopView;

/// The windows of `view` the picker offers, in the desktop's order (most recently used first):
/// documents that are showing and not hidden from captures. Minimised windows have nothing to
/// capture, a panel or a sheet is not something to share, and a window the compositor hides from every capture is never
/// offered. A window with no title shows its app's name.
pub fn windows_of(view: &DesktopView) -> Vec<WindowCandidate> {
    view.windows
        .iter()
        .filter(|window| window.role == WindowRole::Document)
        .filter(|window| window.state != WindowStateKind::Minimised)
        .filter(|window| window.sensitivity != SensitivityLevel::CaptureHidden)
        .map(|window| WindowCandidate {
            identifier: window.identifier.clone(),
            title: if window.title.is_empty() {
                window.app.clone()
            } else {
                window.title.clone()
            },
            app: window.app.clone(),
        })
        .collect()
}
