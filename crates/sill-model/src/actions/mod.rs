//! What each of the shell's actions is: the one table of its words, whether it asks first, the
//! chord that stands for it, and the closed [`Effect`] the daemon carries out. Surfaces name an
//! action by [`ActionId`] and hand it over (`sill_ui_kit::links::ActionLink`); the daemon's
//! interpreter matches `Effect` once. Pure data: no entrance decides an output, a token or an
//! "ask first" for itself.

mod ask;
mod intent;

#[cfg(test)]
mod tests;

pub use ask::{Ask, Needs, Start, Subject};
use chordkit::StandardAction;
use ds_core::word::Word;
pub use intent::intent_of;
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
    /// Whether choosing it asks first (a power action that ends the apps, erasing the Trash) or
    /// happens at once.
    pub confirm: Confirm,
    /// What the action needs to be told to do it.
    pub needs: Needs,
    /// The standard action whose chord does it, where it has one.
    pub chord: Option<StandardAction>,
    /// What carrying it out comes to.
    pub effect: Effect,
}

impl ActionDef {
    /// This row, told what it needs to act on.
    fn needing(self, needs: Needs) -> ActionDef {
        ActionDef { needs, ..self }
    }

    /// This row, asking first.
    fn asking(self) -> ActionDef {
        ActionDef {
            confirm: Confirm::Ask,
            ..self
        }
    }
}

/// What carrying an action out comes to: a closed set, matched once in the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// A power action, by the session service.
    Session(SessionAction),
    /// Show, hide or flip a panel: whether its action only opens it is [`Panel::opens`].
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
    /// Ask whether to erase the Trash for good, and empty it when the answer is yes.
    EmptyTrash,
    /// Switch to the next input source of the running input-method framework.
    InputSource,
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
    /// True when the panel's action opens it and leaves it open (the power menu stays up until
    /// Escape or a button, as the Mac's does); false when the action flips it, open when closed
    /// and closed when open (every other panel).
    pub fn opens(self) -> bool {
        match self {
            Panel::PowerMenu => true,
            Panel::Launcher
            | Panel::ForceQuit
            | Panel::NotificationCenter
            | Panel::ControlCenter => false,
        }
    }
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
    DockMagnification,
    Focus,
}

/// The action that takes a screenshot of `kind`.
pub fn screenshot_action(kind: ShotKind) -> ActionId {
    action_with(Effect::Shot(kind))
}

/// The action that opens `panel`.
pub fn panel_action(panel: Panel) -> ActionId {
    action_with(Effect::Panel(panel))
}

/// The action whose row carries `effect`.
fn action_with(effect: Effect) -> ActionId {
    ActionId::ALL
        .iter()
        .copied()
        .find(|id| def(*id).effect == effect)
        // A test holds that every shot kind and panel has a row.
        .expect("every shot kind and panel has an action")
}

/// What `id` is. Every action is a row here: the match has no wildcard, so a new `ActionId` is
/// a compile error until it is described.
pub fn def(id: ActionId) -> ActionDef {
    use ActionId as A;
    let row = |label, chord, effect| ActionDef {
        id,
        label,
        confirm: Confirm::Now,
        needs: Needs::Nothing,
        chord,
        effect,
    };
    let session = |label, chord, action: SessionAction| ActionDef {
        confirm: action.confirm(),
        ..row(label, chord, Effect::Session(action))
    };
    match id {
        A::Lock => session(
            "Lock Screen",
            Some(StandardAction::LockScreen),
            SessionAction::Lock,
        ),
        A::LogOut => session(
            "Log Out",
            Some(StandardAction::LogOut),
            SessionAction::LogOut,
        ),
        A::Restart => session("Restart", None, SessionAction::Restart),
        A::ShutDown => session("Shut Down", None, SessionAction::PowerOff),
        A::Sleep => session("Sleep", None, SessionAction::Suspend),
        A::Launcher => row(
            "Launcher",
            Some(StandardAction::Launcher),
            Effect::Panel(Panel::Launcher),
        ),
        A::EmojiAndSymbols => row(
            "Emoji & Symbols",
            Some(StandardAction::EmojiAndSymbols),
            Effect::Find(ProviderKind::Emoji),
        ),
        A::NextInputSource => row(
            "Next Input Source",
            Some(StandardAction::NextInputSource),
            Effect::InputSource,
        ),
        A::PowerMenu => row("Power Menu", None, Effect::Panel(Panel::PowerMenu)),
        A::ForceQuitPanel => row(
            "Force Quit Applications…",
            Some(StandardAction::ForceQuit),
            Effect::Panel(Panel::ForceQuit),
        ),
        A::NotificationCenter => row(
            "Notification Center",
            None,
            Effect::Panel(Panel::NotificationCenter),
        ),
        A::ControlCenter => row("Control Center", None, Effect::Panel(Panel::ControlCenter)),
        A::ScreenshotArea => row(
            "Capture Selected Portion",
            Some(StandardAction::ScreenshotSelection),
            Effect::Shot(ShotKind::Area),
        ),
        A::ScreenshotWindow => row(
            "Capture Selected Window",
            None,
            Effect::Shot(ShotKind::Window),
        ),
        A::ScreenshotScreen => row(
            "Capture Entire Screen",
            Some(StandardAction::ScreenshotScreen),
            Effect::Shot(ShotKind::Screen),
        ),
        A::ShowDesktop => row(
            "Show Desktop",
            None,
            Effect::Desktop(DesktopStep::ShowDesktop),
        ),
        A::MissionControl => row(
            "Mission Control",
            Some(StandardAction::MissionControl),
            Effect::Desktop(DesktopStep::MissionControl),
        ),
        A::NextSpace => row(
            "Move Right a Space",
            Some(StandardAction::SpaceRight),
            Effect::Desktop(DesktopStep::Space(Step::Next)),
        ),
        A::PreviousSpace => row(
            "Move Left a Space",
            Some(StandardAction::SpaceLeft),
            Effect::Desktop(DesktopStep::Space(Step::Prev)),
        ),
        A::NextWindow => row(
            "Next Window",
            Some(StandardAction::NextWindow),
            Effect::Desktop(DesktopStep::NextWindow),
        ),
        A::SwitcherNext => row(
            "Next App",
            Some(StandardAction::NextApp),
            Effect::Switcher(SwitcherStep::Next),
        ),
        A::SwitcherPrev => row(
            "Previous App",
            Some(StandardAction::PreviousApp),
            Effect::Switcher(SwitcherStep::Prev),
        ),
        A::SwitcherCommit => row("Choose App", None, Effect::Switcher(SwitcherStep::Commit)),
        A::SwitcherCancel => row(
            "Cancel App Switcher",
            None,
            Effect::Switcher(SwitcherStep::Cancel),
        ),
        A::OpenSettings => row(
            "System Settings…",
            Some(StandardAction::Settings),
            Effect::Settings,
        )
        .needing(Needs::Place),
        A::OpenApp => row("Open", None, Effect::App(AppVerb::Open)).needing(Needs::App),
        A::ShowInFiles => {
            row("Show in Files", None, Effect::App(AppVerb::ShowInFiles)).needing(Needs::App)
        }
        A::HideApp => row(
            "Hide",
            Some(StandardAction::Hide),
            Effect::App(AppVerb::Hide),
        )
        .needing(Needs::Windows),
        A::QuitApp => row(
            "Quit",
            Some(StandardAction::Quit),
            Effect::App(AppVerb::Quit),
        )
        .needing(Needs::Windows),
        A::ForceQuitApp => {
            row("Force Quit", None, Effect::App(AppVerb::ForceQuit)).needing(Needs::App)
        }
        A::PinApp => row("Keep in Dock", None, Effect::App(AppVerb::Pin)).needing(Needs::App),
        A::UnpinApp => {
            row("Remove from Dock", None, Effect::App(AppVerb::Unpin)).needing(Needs::App)
        }
        A::ToggleDockHide => row(
            "Turn Dock Hiding On or Off",
            Some(StandardAction::ToggleDock),
            Effect::Pref(Pref::DockAutoHide),
        ),
        A::ToggleDockMagnification => row(
            "Turn Dock Magnification On or Off",
            None,
            Effect::Pref(Pref::DockMagnification),
        ),
        A::EmptyTrash => row("Empty Trash", None, Effect::EmptyTrash).asking(),
        A::ToggleFocus => row("Do Not Disturb", None, Effect::Pref(Pref::Focus)),
        A::Reload => row("Reload Settings", None, Effect::Reload),
    }
}
