//! What a surface declares about its life, beside where it lives ([`super::OutputPolicy`]): how
//! it is mapped, and what losing the keyboard does to it.

use shell_host::{KeyboardFocus, OutputId};

use crate::links::{CenterLink, LauncherLink, PowerMenuLink, SplitPickerLink, SwitcherLink};

/// How a surface goes on screen and off it. The daemon spawns `Always` surfaces mapped and every
/// other one hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Presence {
    /// Mapped from the start, for good.
    Always,
    /// Follows its panel link: mapped while the link shows on its output.
    FollowsPanel,
    /// A sheet: mapped while a service holds a question for it (`SheetStage`).
    SheetStage,
    /// The daemon maps and unmaps it (the idle dim).
    Daemon,
    /// Its own machine maps it (the OSD, the banners, the switcher): each has an exit of its own.
    SelfMapped,
}

/// What losing keyboard focus does to a surface that holds it, declared per surface as the Mac
/// does: a transient panel closes, a question that waits for an answer stays.
pub enum FocusLoss<Link> {
    /// Nothing: the surface stays as it is.
    Ignore,
    /// The surface's link hears about every focus change on `output` and closes the surface.
    CloseLink(fn(&Link, OutputId, KeyboardFocus)),
}

impl<Link> std::fmt::Debug for FocusLoss<Link> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FocusLoss::Ignore => "FocusLoss::Ignore",
            FocusLoss::CloseLink(_) => "FocusLoss::CloseLink",
        })
    }
}

/// A link whose panel closes when its keyboard focus is lost after it was held. The late leave
/// from an earlier opening's unmap is told apart from this opening's by the link.
pub trait ClosesOnFocusLoss {
    /// The keyboard focus of the surface on `output` changed.
    fn keyboard(&self, output: OutputId, focus: KeyboardFocus);
}

impl ClosesOnFocusLoss for LauncherLink {
    fn keyboard(&self, _output: OutputId, focus: KeyboardFocus) {
        self.focus(focus);
    }
}

impl ClosesOnFocusLoss for PowerMenuLink {
    fn keyboard(&self, _output: OutputId, focus: KeyboardFocus) {
        self.focus(focus);
    }
}

impl ClosesOnFocusLoss for CenterLink {
    fn keyboard(&self, _output: OutputId, focus: KeyboardFocus) {
        self.focus(focus);
    }
}

impl ClosesOnFocusLoss for SwitcherLink {
    fn keyboard(&self, output: OutputId, focus: KeyboardFocus) {
        self.focus(output, focus);
    }
}

impl ClosesOnFocusLoss for SplitPickerLink {
    fn keyboard(&self, _output: OutputId, focus: KeyboardFocus) {
        self.focus(focus);
    }
}
