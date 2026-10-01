//! Telling the person: when a notice is posted or withdrawn, and what it says.

use super::model::Attention;

/// What to do with the notification about keycap, given what it showed and what keycap now needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoticeChange {
    /// What is shown is right.
    Keep,
    /// Show this, in place of what was shown.
    Show(Attention),
    /// What was shown no longer applies.
    Withdraw,
}

/// The change from `previous` (what the notification now says) to `next` (what keycap now needs
/// the person to know).
pub fn notice_for(previous: Option<&Attention>, next: Option<&Attention>) -> NoticeChange {
    match (previous, next) {
        (previous, Some(next)) if previous != Some(next) => NoticeChange::Show(next.clone()),
        (Some(_), None) => NoticeChange::Withdraw,
        _ => NoticeChange::Keep,
    }
}

/// A notification's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub summary: String,
    pub body: String,
}

/// What the person is told about `attention`. The notification carries no action: nothing
/// routes an action of sill's own back to a service, so the way out is the command line.
pub fn notice_text(attention: &Attention) -> Notice {
    let (summary, body) = match attention {
        Attention::Failed => (
            "Mac keyboard layout stopped".to_owned(),
            "Keys work as on a standard keyboard. Run `sill keycap retry` to try again.".to_owned(),
        ),
        Attention::Conflict { device } => (
            "Another keyboard program is running".to_owned(),
            format!("{device} keeps its own mapping while it runs."),
        ),
        Attention::Paused => (
            "Mac keyboard layout paused".to_owned(),
            "Keys work as on a standard keyboard. Run `sill keycap resume` to turn it back on."
                .to_owned(),
        ),
    };
    Notice { summary, body }
}
