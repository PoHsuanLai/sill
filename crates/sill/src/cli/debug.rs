//! `sill debug ...`: the development probes' command line, and their pure mappings to requests.
//! Each made-up input is built as the `sill-model` value the service would have received.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use sill_ipc::debug::{
    CornerName, DebugRequest, LauncherKey, OsdTopic, Point, PointerButton, SurfaceName,
};
use sill_ipc::request::Request;
use sill_launcher::AppId;
use sill_model::auth::FakeVerdict;
use sill_model::notifications::{
    Action, ActionKey, AppRef, Expiry, Hints, IconRef, NotificationId, NotifyUrgency, Posting,
    parse_body,
};
use sill_model::polkit_agent::{ActionId, FakeRequest};
use sill_model::units::Percent;
use sill_model::volume::Mute;

/// `sill debug ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum DebugAction {
    /// Frame statistics for every surface.
    Stats,
    /// Move the pointer to X Y on SURFACE (`sill-dock-DP-1`).
    Pointer { surface: String, x: i32, y: i32 },
    /// The pointer leaves SURFACE.
    Leave { surface: String },
    /// Render SURFACE to a PNG at PATH.
    Capture { surface: String, path: PathBuf },
    /// Press and release a pointer button at X Y on SURFACE (`sill-bar-DP-1`).
    Click {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_enum, default_value_t = ClickButton::Primary)]
        button: ClickButton,
    },
    /// Show the OSD for a made-up change, moving no real level.
    Osd {
        /// What changed.
        #[arg(value_enum)]
        topic: TopicArg,
        /// The level to show, 0-100.
        percent: u8,
        /// Whether the made-up volume is muted.
        #[arg(long, value_enum, default_value_t = MuteArg::Audible)]
        mute: MuteArg,
    },
    /// Post a made-up notification without D-Bus: a banner, and a row in the center.
    Notify(NotifyArgs),
    /// Show the screenshot thumbnail for PATH, as if it had just been saved.
    Screenshot { path: PathBuf },
    /// Carry out CORNER's hot corner action, as if the pointer had rested there.
    Corner {
        #[arg(value_enum)]
        corner: CornerArg,
    },
    /// `button` presses at X Y on SURFACE and stays down: the first half of a real drag
    /// (shell-host's `Inject::PointerDown`). `sill debug pointer` moves while it
    /// is held, and `pointer-release` lets go.
    PointerPress {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_enum, default_value_t = ClickButton::Primary)]
        button: ClickButton,
    },
    /// `button` releases at X Y on SURFACE: the second half of a real drag
    /// (`Inject::PointerUp`).
    PointerRelease {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_enum, default_value_t = ClickButton::Primary)]
        button: ClickButton,
    },
    /// Feed the lock screen's password check a made-up verdict (a debug daemon only).
    Auth {
        #[arg(value_enum)]
        verdict: VerdictArg,
    },
    /// Show a made-up polkit request (its stand-in helper accepts `sill-fake`).
    Polkit(PolkitArgs),
    /// Open the app switcher on made-up apps (desktop ids, the current one first).
    Switcher {
        #[arg(required = true)]
        apps: Vec<String>,
    },
    /// Press a key on the open launcher (a debug daemon only).
    LauncherKey {
        #[arg(value_enum)]
        key: LauncherKeyArg,
    },
    /// Print the open launcher's sections, rows and preview (a debug daemon only).
    LauncherRows,
    /// Wake idle as real input would (a debug daemon only; inside the lock grace it opens the
    /// lock).
    IdleWake,
}

/// `sill debug notify SUMMARY ...`: the made-up notification's parts.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct NotifyArgs {
    /// The headline.
    pub summary: String,
    /// The sending app's name (and the group it falls in).
    #[arg(long, default_value = "sill")]
    pub app: String,
    /// Markup, as a sender would write it.
    #[arg(long, default_value = "")]
    pub body: String,
    /// How urgent.
    #[arg(long, value_enum, default_value_t = UrgencyArg::Normal)]
    pub urgency: UrgencyArg,
    /// A button, `KEY=LABEL` (repeatable; `default` is the body's click).
    #[arg(long = "action", value_name = "KEY=LABEL")]
    pub actions: Vec<String>,
    /// How long it stays.
    #[arg(long, value_enum, default_value_t = ExpireArg::Default)]
    pub expire: ExpireArg,
    /// The id it replaces (0: none).
    #[arg(long, default_value_t = 0)]
    pub replaces: u32,
}

/// `sill debug polkit ...`: the made-up request's parts.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct PolkitArgs {
    /// The action id shown.
    #[arg(long = "action", default_value = "org.sill.debug.prompt")]
    pub action_id: String,
    /// The message shown.
    #[arg(long, default_value = "sill is testing its password prompt")]
    pub message: String,
}

/// `sill debug osd TOPIC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TopicArg {
    Volume,
    Brightness,
}

/// `sill debug osd --mute ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MuteArg {
    Audible,
    Muted,
}

/// `sill debug click --button ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ClickButton {
    Primary,
    Secondary,
    Middle,
}

/// `sill debug notify --urgency ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum UrgencyArg {
    Low,
    Normal,
    Critical,
}

/// `sill debug notify --expire ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExpireArg {
    Default,
    Never,
}

/// `sill debug corner CORNER`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CornerArg {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// `sill debug auth VERDICT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum VerdictArg {
    Accept,
    Refuse,
}

/// `sill debug launcher-key KEY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LauncherKeyArg {
    Up,
    Down,
    Enter,
    Escape,
    Tab,
    ShiftTab,
    Actions,
    Space,
    QuickLook,
    Right,
    Left,
    Reveal,
}

/// The request `sill debug ACTION` sends.
pub fn debug_request(action: &DebugAction) -> Request {
    Request::Debug(match action {
        DebugAction::Stats => DebugRequest::Stats,
        DebugAction::Pointer { surface, x, y } => DebugRequest::Pointer {
            surface: surface_name(surface),
            at: point(*x, *y),
        },
        DebugAction::Leave { surface } => DebugRequest::Leave {
            surface: surface_name(surface),
        },
        DebugAction::Capture { surface, path } => DebugRequest::Capture {
            surface: surface_name(surface),
            path: path.clone(),
        },
        DebugAction::Click {
            surface,
            x,
            y,
            button,
        } => DebugRequest::Click {
            surface: surface_name(surface),
            at: point(*x, *y),
            button: pointer_button(*button),
        },
        DebugAction::PointerPress {
            surface,
            x,
            y,
            button,
        } => DebugRequest::PointerPress {
            surface: surface_name(surface),
            at: point(*x, *y),
            button: pointer_button(*button),
        },
        DebugAction::PointerRelease {
            surface,
            x,
            y,
            button,
        } => DebugRequest::PointerRelease {
            surface: surface_name(surface),
            at: point(*x, *y),
            button: pointer_button(*button),
        },
        DebugAction::Osd {
            topic,
            percent,
            mute,
        } => DebugRequest::Osd {
            topic: osd_topic(*topic),
            percent: Percent(u16::from(*percent)),
            mute: mute_of(*mute),
        },
        DebugAction::Notify(args) => DebugRequest::Notify(Box::new(posting(args))),
        DebugAction::Screenshot { path } => DebugRequest::Screenshot { path: path.clone() },
        DebugAction::Corner { corner } => DebugRequest::Corner(corner_name(*corner)),
        DebugAction::Auth { verdict } => DebugRequest::Auth(verdict_of(*verdict)),
        DebugAction::Polkit(args) => DebugRequest::Polkit(polkit_request(args)),
        DebugAction::Switcher { apps } => DebugRequest::Switcher {
            apps: apps.iter().cloned().map(AppId).collect(),
        },
        DebugAction::LauncherKey { key } => DebugRequest::LauncherKey(launcher_key(*key)),
        DebugAction::LauncherRows => DebugRequest::LauncherRows,
        DebugAction::IdleWake => DebugRequest::IdleWake,
    })
}

fn surface_name(surface: &str) -> SurfaceName {
    SurfaceName(surface.to_owned())
}

fn point(x: i32, y: i32) -> Point {
    Point { x, y }
}

fn pointer_button(button: ClickButton) -> PointerButton {
    match button {
        ClickButton::Primary => PointerButton::Primary,
        ClickButton::Secondary => PointerButton::Secondary,
        ClickButton::Middle => PointerButton::Middle,
    }
}

fn osd_topic(topic: TopicArg) -> OsdTopic {
    match topic {
        TopicArg::Volume => OsdTopic::Volume,
        TopicArg::Brightness => OsdTopic::Brightness,
    }
}

fn mute_of(mute: MuteArg) -> Mute {
    match mute {
        MuteArg::Audible => Mute::Audible,
        MuteArg::Muted => Mute::Muted,
    }
}

fn corner_name(corner: CornerArg) -> CornerName {
    match corner {
        CornerArg::TopLeft => CornerName::TopLeft,
        CornerArg::TopRight => CornerName::TopRight,
        CornerArg::BottomLeft => CornerName::BottomLeft,
        CornerArg::BottomRight => CornerName::BottomRight,
    }
}

fn verdict_of(verdict: VerdictArg) -> FakeVerdict {
    match verdict {
        VerdictArg::Accept => FakeVerdict::Accept,
        VerdictArg::Refuse => FakeVerdict::Refuse,
    }
}

fn polkit_request(args: &PolkitArgs) -> FakeRequest {
    FakeRequest {
        action_id: ActionId(args.action_id.clone()),
        message: args.message.clone(),
        icon: String::new(),
    }
}

fn launcher_key(key: LauncherKeyArg) -> LauncherKey {
    match key {
        LauncherKeyArg::Up => LauncherKey::Up,
        LauncherKeyArg::Down => LauncherKey::Down,
        LauncherKeyArg::Enter => LauncherKey::Enter,
        LauncherKeyArg::Escape => LauncherKey::Escape,
        LauncherKeyArg::Tab => LauncherKey::Tab,
        LauncherKeyArg::ShiftTab => LauncherKey::ShiftTab,
        LauncherKeyArg::Actions => LauncherKey::Actions,
        LauncherKeyArg::Space => LauncherKey::Space,
        LauncherKeyArg::QuickLook => LauncherKey::QuickLook,
        LauncherKeyArg::Right => LauncherKey::Right,
        LauncherKeyArg::Left => LauncherKey::Left,
        LauncherKeyArg::Reveal => LauncherKey::Reveal,
    }
}

/// A `KEY=LABEL` word as an action; a word with no `=` is both.
fn action(word: &str) -> Action {
    let (key, label) = word.split_once('=').unwrap_or((word, word));
    Action {
        key: ActionKey(key.to_owned()),
        label: label.to_owned(),
    }
}

/// The posting a real `Notify` with these parts would have made. The body is parsed as the
/// D-Bus path parses one, so `--body "<b>bold</b>"` shows as it would from a real sender.
fn posting(args: &NotifyArgs) -> Posting {
    Posting {
        replaces: NotificationId::replacing(args.replaces),
        app: AppRef {
            name: args.app.clone(),
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary: args.summary.clone(),
        body: parse_body(&args.body),
        actions: args.actions.iter().map(|word| action(word)).collect(),
        hints: Hints {
            urgency: match args.urgency {
                UrgencyArg::Low => NotifyUrgency::Low,
                UrgencyArg::Normal => NotifyUrgency::Normal,
                UrgencyArg::Critical => NotifyUrgency::Critical,
            },
            ..Hints::default()
        },
        expiry: match args.expire {
            ExpireArg::Default => Expiry::Default,
            ExpireArg::Never => Expiry::Never,
        },
    }
}

#[cfg(test)]
mod tests {
    use sill_model::notifications::{ActionKey, AppKey, Expiry, NotificationId, NotifyUrgency};

    use super::{ExpireArg, NotifyArgs, UrgencyArg, action, posting};

    #[test]
    fn an_action_word_splits_at_its_first_equals() {
        let cases = [
            ("default=Open", "default", "Open"),
            ("reply", "reply", "reply"),
            ("a=b=c", "a", "b=c"),
        ];
        for (word, key, label) in cases {
            let got = action(word);
            assert_eq!(
                (got.key.0.as_str(), got.label.as_str()),
                (key, label),
                "{word}"
            );
        }
    }

    #[test]
    fn the_arguments_become_the_posting_notify_would_make() {
        let posting = posting(&NotifyArgs {
            summary: "Ada".to_owned(),
            app: "Mail".to_owned(),
            body: "the <b>engine</b> &amp; more".to_owned(),
            urgency: UrgencyArg::Critical,
            actions: vec!["default=Open".to_owned()],
            expire: ExpireArg::Never,
            replaces: 3,
        });
        assert_eq!(posting.replaces, Some(NotificationId(3)));
        assert_eq!(posting.app.key(), AppKey("Mail".to_owned()));
        assert_eq!(posting.summary, "Ada");
        assert_eq!(posting.body.text(), "the engine & more");
        assert_eq!(
            posting.body.runs.len(),
            3,
            "markup parsed: {:?}",
            posting.body
        );
        assert_eq!(posting.actions[0].key, ActionKey::default_action());
        assert_eq!(posting.hints.urgency, NotifyUrgency::Critical);
        assert_eq!(posting.expiry, Expiry::Never);
    }
}
