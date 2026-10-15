//! What the compositor's shell protocol tells the shell, in the shell's words: the modifiers
//! held over its surfaces, the key gestures, the switcher, the overview, the hot corners, the
//! edge reveals and the moments (a menu flash, a bell, a reopen, the kill chord).
//!
//! Pure data. `sill-services` turns the wire's events into these and hands them to whatever
//! draws or acts; the desktop's picture itself (Spaces, windows, the overview's layout) is the
//! [`super::DesktopView`], not a note.

use casement_protocols::vocab::{
    Corner, KeyGesture, MacMods, OverviewMode, OverviewState, ScreenEdge, Shown, SplitSide,
};
use porter_core::SpaceId;

use super::SpacePicture;

/// One thing the shell protocol said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellNote {
    /// The modifiers held while the pointer is over a shell surface (keycap stays the source of
    /// truth for chords; this is the pointer-over-shell view, for hot corners and the Option
    /// variants of the dock and bar).
    Modifiers(MacMods),
    /// A key gesture the desktop recognised (the companion's summon, the hold overlay).
    Gesture(KeyGesture),
    /// The app switcher.
    Switcher(SwitcherNote),
    /// An overview (Mission Control, App Exposé, Show Desktop).
    Overview(OverviewNote),
    /// The pointer pushed into a screen corner.
    HotCorner(Corner),
    /// The desktop wants an edge's bar or dock shown or hidden (in full screen).
    Reveal {
        /// Which edge.
        edge: ScreenEdge,
        /// Show or hide.
        shown: Shown,
    },
    /// A menu item of an app was activated by a key: flash it.
    MenuFlash {
        /// The app's key.
        app: String,
        /// The item.
        item: u32,
    },
    /// An app rang the bell.
    Bell(String),
    /// The desktop asks the dock to reopen an app that has no windows.
    Reopen(String),
    /// The pointer rested on a window's green light: show that window's menu.
    WindowMenu {
        /// The window's identifier, as [`super::DesktopCommand::PickWindowMenu`] quotes it.
        window: String,
        /// Where the menu's top left goes, in the desktop's logical pixels.
        at: (i32, i32),
        /// A bit per [`casement_protocols::vocab::WindowEntry`] code, set when the entry can be
        /// picked now; the rest are shown dimmed.
        enabled: u32,
    },
    /// A window went full screen on one half of the screen: offer the windows of the
    /// [`ShellNote::SplitCandidate`]s that follow for the other half. The answer is
    /// [`super::DesktopCommand::PickSplit`] or [`super::DesktopCommand::CancelSplit`].
    SplitChoose {
        /// The window already full screen, by its identifier.
        first: String,
        /// The half it took.
        side: SplitSide,
    },
    /// A window that can fill the other half, by its identifier; the most recently used comes
    /// first.
    SplitCandidate(String),
    /// The choosing for this first window is over (a pick, a cancel, or it left full screen):
    /// take the offer away.
    SplitChoiceEnd(String),
    /// Mission Control is up or closed on one output: the Spaces bar is drawn on the outputs where
    /// it is up.
    OverviewOutput {
        /// The output's name.
        output: String,
        /// Whether its Mission Control is up.
        shown: Shown,
    },
    /// The window dragged in Mission Control is over the target of a Space on an output, or over
    /// none: light that Space's tile.
    DropTarget {
        /// The output's name.
        output: String,
        /// The Space under the dragged window; `None` when over none, or the drag ended.
        space: Option<SpaceId>,
    },
    /// The window dragged in Mission Control entered or left the plus that makes a new Space on an
    /// output: light the plus, or put its light out.
    DropTargetNew {
        /// The output's name.
        output: String,
        /// `Shown` while the window is over the plus; `Hidden` when it left or the drag ended.
        over: Shown,
    },
    /// The answer to a [`super::DesktopCommand::CaptureSpace`].
    SpacePicture {
        /// The output's name.
        output: String,
        /// The Space.
        space: SpaceId,
        /// The picture, or that none could be made.
        picture: SpacePicture,
    },
    /// The kill chord was pressed.
    Killed,
}

/// The switcher's part of the shell protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitcherNote {
    /// The app at `index` of the list the switcher will show.
    App {
        /// Its place in the list.
        index: u32,
        /// The app's key.
        app: String,
    },
    /// The switcher opened with `selected` chosen.
    Shown {
        /// The chosen index.
        selected: u32,
    },
    /// The selection moved.
    Selected(u32),
    /// The switcher closed.
    Hidden,
}

/// The overview's part of the shell protocol; the layout itself is read from the desktop's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverviewNote {
    /// The overview's mode, state and progress (thousandths) changed.
    Moved {
        /// Which overview.
        mode: OverviewMode,
        /// Closed, entering, open, leaving or tracking a gesture.
        state: OverviewState,
        /// How far along, in the protocol's units.
        progress: u32,
    },
    /// A complete layout arrived.
    Laid,
}
