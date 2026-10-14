//! Why a launch failed, and the plain words that tell the person.

use serde::{Deserialize, Serialize};

/// Why an app did not open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCause {
    /// The app is no longer in the list of installed apps.
    NotInstalled,
    /// The desktop could not be asked to start it (no session bus).
    NoStarter,
    /// The app was asked to start and did not answer.
    NoAnswer,
    /// Its program could not be run (missing, or not allowed).
    CannotRun,
    /// It started and ended with an error within moments.
    ClosedAtOnce,
}

/// A notification's words for a failed launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchNotice {
    pub summary: String,
    pub body: String,
}

/// What the person is told when `app_name` did not open because of `cause`.
pub fn failure_text(app_name: &str, cause: FailureCause) -> LaunchNotice {
    let body = match cause {
        FailureCause::NotInstalled => "It is not in your apps any more.",
        FailureCause::NoStarter => "The desktop was not ready to start it. Try again in a moment.",
        FailureCause::NoAnswer => "It did not answer when it was asked to open.",
        FailureCause::CannotRun => "Its program could not be run. It may have been removed.",
        FailureCause::ClosedAtOnce => "It closed again right after it started.",
    };
    LaunchNotice {
        summary: format!("{app_name} couldn't open"),
        body: body.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_cause_names_the_app_and_says_why_in_plain_words() {
        let causes = [
            FailureCause::NotInstalled,
            FailureCause::NoStarter,
            FailureCause::NoAnswer,
            FailureCause::CannotRun,
            FailureCause::ClosedAtOnce,
        ];
        let bodies: Vec<String> = causes
            .iter()
            .map(|cause| {
                let notice = failure_text("Ghostty", *cause);
                assert_eq!(notice.summary, "Ghostty couldn't open");
                notice.body
            })
            .collect();
        let mut distinct = bodies.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), bodies.len());
    }
}
