//! What each of the shell's actions is: the one table of its words, whether it asks first, the
//! chord that stands for it, and the closed [`Effect`] the daemon carries out. Surfaces name an
//! action by [`ActionId`] and hand it over (`sill_ui_kit::links::ActionLink`); the daemon's
//! interpreter matches `Effect` once. Pure data: no entrance decides an output, a token or an
//! "ask first" for itself.

mod ask;

#[cfg(test)]
mod tests;

pub use ask::{Ask, Needs, Subject};
use chordkit::StandardAction;
use sill_launcher::actions::ActionId;
use sill_launcher::{Confirm, ProviderKind, SessionAction};

use crate::screenshots::ShotKind;
use crate::windows::Step;

/// What an action is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionDef {
    pub id: ActionId,
    /// What a menu row or a launcher row calls it, in the Mac's words.
    pub label: &'static str,
    /// Whether choosing it asks first (the power menu) or happens at once.
    pub confirm: Confirm,
    /// The standard action whose chord does it, where it has one.
    pub chord: Option<StandardAction>,
    /// What carrying it out comes to.
    pub effect: Effect,
}

/// What carrying an action out comes to: a closed set, matched once in the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// A power action, by the session service.
    Session(SessionAction),
    /// Show, hide or flip a panel: the way its action opens it is [`Panel::way`].
    Panel(Panel),
    /// The launcher, opened on one kind of result alone.
    Find(ProviderKind),
    /// A step of the app switcher.
    Switcher(SwitcherStep),
    /// A screenshot with the person's tool.
    Shot(ShotKind),
    /// Something done to the desktop as a whole.
    Desktop(DesktopStep),
    /// Open Settings.
    Settings,
    /// Something done to an app.
    App(AppVerb),
    /// Flip a preference the shell keeps.
    Pref(Pref),
    /// Re-read settings.
    Reload,
}

/// The panels the shell shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Panel {
    Launcher,
    PowerMenu,
    ForceQuit,
    NotificationCenter,
    ControlCenter,
}

impl Panel {
    /// How the panel's action opens it. The power menu opens and stays up until Escape or a
    /// button, as the Mac's does; every other panel flips.
    pub fn way(self) -> Way {
        match self {
            Panel::PowerMenu => Way::Open,
            Panel::Launcher
            | Panel::ForceQuit
            | Panel::NotificationCenter
            | Panel::ControlCenter => Way::Toggle,
        }
    }
}

/// How an action changes a panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Way {
    /// Open when closed, close when open.
    Toggle,
    /// Open, or stay open.
    Open,
}

/// One step of the app switcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwitcherStep {
    Next,
    Prev,
    Commit,
    Cancel,
}

/// What is done to the desktop as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopStep {
    /// Minimise every window, or bring back the ones the last firing put away.
    ShowDesktop,
    MissionControl,
    /// Go to the Space one over.
    Space(Step),
    /// Bring forward the front app's next window.
    NextWindow,
}

/// What is done to an app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppVerb {
    Open,
    ShowInFiles,
    Hide,
    Quit,
    /// End the scopes the shell started it in.
    ForceQuit,
    Pin,
    Unpin,
}

/// A preference the shell keeps and an action flips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pref {
    DockAutoHide,
    Focus,
}

impl Effect {
    /// What the action needs to be told to do it.
    pub fn needs(self) -> Needs {
        match self {
            Effect::Settings => Needs::Place,
            Effect::App(AppVerb::Hide | AppVerb::Quit) => Needs::Windows,
            Effect::App(
                AppVerb::Open
                | AppVerb::ShowInFiles
                | AppVerb::ForceQuit
                | AppVerb::Pin
                | AppVerb::Unpin,
            ) => Needs::App,
            Effect::Session(_)
            | Effect::Panel(_)
            | Effect::Find(_)
            | Effect::Switcher(_)
            | Effect::Shot(_)
            | Effect::Desktop(_)
            | Effect::Pref(_)
            | Effect::Reload => Needs::Nothing,
        }
    }

    /// Whether choosing it asks first: only a power action that ends the apps does.
    pub fn confirm(self) -> Confirm {
        match self {
            Effect::Session(action) => action.confirm(),
            Effect::Panel(_)
            | Effect::Find(_)
            | Effect::Switcher(_)
            | Effect::Shot(_)
            | Effect::Desktop(_)
            | Effect::Settings
            | Effect::App(_)
            | Effect::Pref(_)
            | Effect::Reload => Confirm::Now,
        }
    }
}

/// The action that takes a screenshot of `kind`.
pub fn screenshot_action(kind: ShotKind) -> ActionId {
    match kind {
        ShotKind::Area => ActionId::ScreenshotArea,
        ShotKind::Window => ActionId::ScreenshotWindow,
        ShotKind::Screen => ActionId::ScreenshotScreen,
    }
}

/// The action that opens `panel`.
pub fn panel_action(panel: Panel) -> ActionId {
    match panel {
        Panel::Launcher => ActionId::Launcher,
        Panel::PowerMenu => ActionId::PowerMenu,
        Panel::ForceQuit => ActionId::ForceQuitPanel,
        Panel::NotificationCenter => ActionId::NotificationCenter,
        Panel::ControlCenter => ActionId::ControlCenter,
    }
}

/// What `id` is. Every action is a row here: the match has no wildcard, so a new `ActionId` is
/// a compile error until it is described.
pub fn def(id: ActionId) -> ActionDef {
    use ActionId as A;
    let (label, chord, effect) = match id {
        A::Lock => (
            "Lock Screen",
            Some(StandardAction::LockScreen),
            Effect::Session(SessionAction::Lock),
        ),
        A::LogOut => (
            "Log Out",
            Some(StandardAction::LogOut),
            Effect::Session(SessionAction::LogOut),
        ),
        A::Restart => ("Restart", None, Effect::Session(SessionAction::Restart)),
        A::ShutDown => ("Shut Down", None, Effect::Session(SessionAction::PowerOff)),
        A::Sleep => ("Sleep", None, Effect::Session(SessionAction::Suspend)),
        A::Launcher => (
            "Launcher",
            Some(StandardAction::Launcher),
            Effect::Panel(Panel::Launcher),
        ),
        A::EmojiAndSymbols => (
            "Emoji & Symbols",
            Some(StandardAction::EmojiAndSymbols),
            Effect::Find(ProviderKind::Emoji),
        ),
        A::PowerMenu => ("Power Menu", None, Effect::Panel(Panel::PowerMenu)),
        A::ForceQuitPanel => (
            "Force Quit Applications…",
            Some(StandardAction::ForceQuit),
            Effect::Panel(Panel::ForceQuit),
        ),
        A::NotificationCenter => (
            "Notification Center",
            None,
            Effect::Panel(Panel::NotificationCenter),
        ),
        A::ControlCenter => ("Control Center", None, Effect::Panel(Panel::ControlCenter)),
        A::ScreenshotArea => (
            "Capture Selected Portion",
            Some(StandardAction::ScreenshotSelection),
            Effect::Shot(ShotKind::Area),
        ),
        A::ScreenshotWindow => (
            "Capture Selected Window",
            None,
            Effect::Shot(ShotKind::Window),
        ),
        A::ScreenshotScreen => (
            "Capture Entire Screen",
            Some(StandardAction::ScreenshotScreen),
            Effect::Shot(ShotKind::Screen),
        ),
        A::ShowDesktop => (
            "Show Desktop",
            None,
            Effect::Desktop(DesktopStep::ShowDesktop),
        ),
        A::MissionControl => (
            "Mission Control",
            Some(StandardAction::MissionControl),
            Effect::Desktop(DesktopStep::MissionControl),
        ),
        A::NextSpace => (
            "Move Right a Space",
            Some(StandardAction::SpaceRight),
            Effect::Desktop(DesktopStep::Space(Step::Next)),
        ),
        A::PreviousSpace => (
            "Move Left a Space",
            Some(StandardAction::SpaceLeft),
            Effect::Desktop(DesktopStep::Space(Step::Prev)),
        ),
        A::NextWindow => (
            "Next Window",
            Some(StandardAction::NextWindow),
            Effect::Desktop(DesktopStep::NextWindow),
        ),
        A::SwitcherNext => (
            "Next App",
            Some(StandardAction::NextApp),
            Effect::Switcher(SwitcherStep::Next),
        ),
        A::SwitcherPrev => (
            "Previous App",
            Some(StandardAction::PreviousApp),
            Effect::Switcher(SwitcherStep::Prev),
        ),
        A::SwitcherCommit => ("Choose App", None, Effect::Switcher(SwitcherStep::Commit)),
        A::SwitcherCancel => (
            "Cancel App Switcher",
            None,
            Effect::Switcher(SwitcherStep::Cancel),
        ),
        A::OpenSettings => (
            "System Settings…",
            Some(StandardAction::Settings),
            Effect::Settings,
        ),
        A::OpenApp => ("Open", None, Effect::App(AppVerb::Open)),
        A::ShowInFiles => ("Show in Files", None, Effect::App(AppVerb::ShowInFiles)),
        A::HideApp => (
            "Hide",
            Some(StandardAction::Hide),
            Effect::App(AppVerb::Hide),
        ),
        A::QuitApp => (
            "Quit",
            Some(StandardAction::Quit),
            Effect::App(AppVerb::Quit),
        ),
        A::ForceQuitApp => ("Force Quit", None, Effect::App(AppVerb::ForceQuit)),
        A::PinApp => ("Keep in Dock", None, Effect::App(AppVerb::Pin)),
        A::UnpinApp => ("Remove from Dock", None, Effect::App(AppVerb::Unpin)),
        A::ToggleDockHide => (
            "Turn Dock Hiding On or Off",
            Some(StandardAction::ToggleDock),
            Effect::Pref(Pref::DockAutoHide),
        ),
        A::ToggleFocus => ("Do Not Disturb", None, Effect::Pref(Pref::Focus)),
        A::Reload => ("Reload Settings", None, Effect::Reload),
    };
    ActionDef {
        id,
        label,
        confirm: effect.confirm(),
        chord,
        effect,
    }
}
