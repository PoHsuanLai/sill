//! Where a minimised window flies to: the dock's icon rectangles, and the commands that keep the
//! compositor's targets equal to them. Pure; the dock fills [`OutputRects`], the daemon turns
//! them and the desktop's windows into [`MinimiseTargets`] and sends [`MinimiseTargets::since`].

use std::collections::BTreeMap;

use super::{DesktopCommand, DesktopView};

/// A rectangle as `(x, y, width, height)` in logical pixels of the dock's surface.
pub type Rect = (i32, i32, u32, u32);

/// The resting rectangle of each app's icon on one dock, by the names the app goes by (the
/// icon's desktop entry and the app ids of its windows).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppRects(pub BTreeMap<String, Rect>);

/// Every dock's [`AppRects`], by output name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OutputRects(pub BTreeMap<String, AppRects>);

/// The target of each window, by [`super::WindowView::identifier`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MinimiseTargets(pub BTreeMap<String, Rect>);

impl MinimiseTargets {
    /// The targets of the windows of `view`: a window's is the icon of its app on the dock of
    /// its output. A window of an app with no icon there, or on no output, has none.
    pub fn of(rects: &OutputRects, view: &DesktopView) -> MinimiseTargets {
        MinimiseTargets(
            view.windows
                .iter()
                .filter_map(|window| {
                    let rect = rects.0.get(&window.output)?.0.get(&window.app)?;
                    Some((window.identifier.clone(), *rect))
                })
                .collect(),
        )
    }

    /// The commands that turn `old` into `self`: a set for each window whose rectangle is new
    /// or changed, a clear for each window that had one and has none. Windows in order of
    /// identifier, clears after sets; equal inputs give none.
    pub fn since(&self, old: &MinimiseTargets) -> Vec<DesktopCommand> {
        let set = self
            .0
            .iter()
            .filter(|(id, rect)| old.0.get(*id) != Some(*rect))
            .map(|(id, rect)| DesktopCommand::SetMinimiseTarget {
                toplevel: id.clone(),
                rect: *rect,
            });
        let clear = old
            .0
            .keys()
            .filter(|id| !self.0.contains_key(*id))
            .map(|id| DesktopCommand::ClearMinimiseTarget(id.clone()));
        set.chain(clear).collect()
    }
}
