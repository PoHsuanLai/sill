//! `sill notifications {toggle,open,close,dismiss-all,status}` and `sill debug notify ...`: the
//! notification center's and the banners' command line, and their pure mappings to IPC requests.

use clap::{Args, Subcommand, ValueEnum};
use sill_ipc::{FakeAction, FakeExpire, FakeNotification, FakeUrgency, NotificationsRequest};

/// `sill notifications ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum NotificationsAction {
    /// Open the notification center when closed, close it when open.
    Toggle,
    Open,
    Close,
    /// Dismiss every banner on screen.
    DismissAll,
    /// Who holds org.freedesktop.Notifications, and how many banners and kept notifications.
    Status,
}

/// The request `action` sends.
pub fn notifications_request(action: NotificationsAction) -> NotificationsRequest {
    match action {
        NotificationsAction::Toggle => NotificationsRequest::Toggle,
        NotificationsAction::Open => NotificationsRequest::Open,
        NotificationsAction::Close => NotificationsRequest::Close,
        NotificationsAction::DismissAll => NotificationsRequest::DismissAll,
        NotificationsAction::Status => NotificationsRequest::Status,
    }
}

/// `sill debug notify SUMMARY ...`: a made-up notification, posted without D-Bus.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct FakeNotifyArgs {
    /// The notification's summary (its bold first line).
    pub summary: String,
    /// The sending app's name (the group it falls in).
    #[arg(long, default_value = "sill")]
    pub app: String,
    /// The body, in the spec's markup.
    #[arg(long, default_value = "")]
    pub body: String,
    #[arg(long, value_enum, default_value_t = UrgencyArg::Normal)]
    pub urgency: UrgencyArg,
    /// An action as KEY=LABEL (`default` is the body's click); repeat for more.
    #[arg(long = "action", value_name = "KEY=LABEL")]
    pub actions: Vec<String>,
    /// `never` makes it an alert that stays until acted on.
    #[arg(long, value_enum, default_value_t = ExpireArg::Default)]
    pub expire: ExpireArg,
    /// The id it replaces (0: none).
    #[arg(long, default_value_t = 0)]
    pub replaces: u32,
}

/// `--urgency`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum UrgencyArg {
    Low,
    Normal,
    Critical,
}

/// `--expire`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExpireArg {
    Default,
    Never,
}

/// `KEY=LABEL`, or a bare KEY that is its own label.
fn action(word: &str) -> FakeAction {
    let (key, label) = word.split_once('=').unwrap_or((word, word));
    FakeAction {
        key: key.to_owned(),
        label: label.to_owned(),
    }
}

/// The made-up notification `args` describe.
pub fn fake_notification(args: &FakeNotifyArgs) -> FakeNotification {
    FakeNotification {
        app: args.app.clone(),
        summary: args.summary.clone(),
        body: args.body.clone(),
        urgency: match args.urgency {
            UrgencyArg::Low => FakeUrgency::Low,
            UrgencyArg::Normal => FakeUrgency::Normal,
            UrgencyArg::Critical => FakeUrgency::Critical,
        },
        actions: args.actions.iter().map(|word| action(word)).collect(),
        expire: match args.expire {
            ExpireArg::Default => FakeExpire::Default,
            ExpireArg::Never => FakeExpire::Never,
        },
        replaces: args.replaces,
    }
}

#[cfg(test)]
mod tests {
    use super::action;

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
                (got.key.as_str(), got.label.as_str()),
                (key, label),
                "{word}"
            );
        }
    }
}
