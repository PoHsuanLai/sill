//! The made-up notification and polkit request `sill debug notify` and `sill debug polkit` send,
//! as the `sill-model` values the services would have received from a real sender.

use clap::{Args, ValueEnum};
use sill_model::notifications::{
    Action, ActionKey, AppRef, Expiry, Hints, IconRef, NotificationId, NotifyUrgency, Posting,
    parse_body,
};
use sill_model::polkit_agent::{ActionId, FakeRequest};

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

/// The request `sill debug polkit` makes: shown like a real one, with no icon.
pub fn polkit_request(args: &PolkitArgs) -> FakeRequest {
    FakeRequest {
        action_id: ActionId(args.action_id.clone()),
        message: args.message.clone(),
        icon: String::new(),
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
pub fn posting(args: &NotifyArgs) -> Posting {
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
