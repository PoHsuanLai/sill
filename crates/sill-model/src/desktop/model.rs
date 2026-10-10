//! The desktop in the shell's words: apps, windows, Spaces and what is active. Built from a
//! backend's state; nothing here knows which backend it came from.

use casement_protocols::vocab::{
    AppLifeKind, Attention, Emphasis, PlacementMark, ProofKind, Shown, SpaceKind, WindowStateKind,
};
use porter_core::SpaceId;

/// Everything the desktop shows about itself.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesktopView {
    /// Running apps, most recently used first.
    pub apps: Vec<AppView>,
    /// Open windows, most recently used first.
    pub windows: Vec<WindowView>,
    /// Spaces, in the order the desktop lists them.
    pub spaces: Vec<SpaceView>,
    /// What has the keyboard.
    pub active: ActiveView,
}

/// A running app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppView {
    /// How the shell names the app: the desktop's proven name, else what the app claims.
    pub key: String,
    /// How the desktop knows who the app is.
    pub proof: ProofKind,
    /// Where the app is in its life.
    pub life: AppLifeKind,
    /// How much the app asks for the person.
    pub attention: Attention,
    /// Recency rank: 0 is the most recently used app.
    pub mru: u32,
}

/// An open window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowView {
    /// The desktop's stable name for the window; what a [`super::DesktopCommand`] quotes.
    pub identifier: String,
    /// The [`AppView::key`] of its app.
    pub app: String,
    /// The title.
    pub title: String,
    /// Floating, tiled, filled, full screen or minimised.
    pub state: WindowStateKind,
    /// Which Spaces show it.
    pub space: SpacePlaceView,
    /// The output it is on; empty when the desktop does not say.
    pub output: String,
    /// Key, main or inactive.
    pub emphasis: Emphasis,
    /// How it is placed, as its zoom control shows.
    pub placement: PlacementMark,
}

/// Which Spaces show a window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpacePlaceView {
    /// One Space.
    On(SpaceId),
    /// Every Space.
    AllSpaces,
    /// The desktop does not say.
    Unknown,
}

/// A Space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceView {
    /// Its stable id.
    pub id: SpaceId,
    /// What it holds.
    pub kind: SpaceKind,
    /// The output it is on; empty when the desktop does not say.
    pub output: String,
    /// Its place in the order, from 0.
    pub position: u32,
    /// Whether it is the one on show.
    pub shown: Shown,
}

/// What has the keyboard.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActiveView {
    /// The [`AppView::key`] of the active app.
    pub app: Option<String>,
    /// The [`WindowView::identifier`] of the window with the keyboard.
    pub key_window: Option<String>,
}
