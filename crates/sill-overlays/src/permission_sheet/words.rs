//! What the sheet says, pure: the headline and the line under it, from the question's cleaned
//! texts, which installed app the question names, and what Escape answers.

use sill_launcher::AppId;
use sill_model::apps::AppIndex;
use sill_model::capture_pick::{AskedBy, PermissionAsking, PermissionOutcome};

/// What an app with no name is called.
const UNNAMED_APP: &str = "An app";

/// The sheet's texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Words {
    /// The headline: the app's own, else a plain sentence naming the app.
    pub title: String,
    /// The app's name, set apart in the line under the headline; `None` when the headline names
    /// the program already (the compositor's questions).
    pub lead: Option<String>,
    /// What follows the name: the app's subtitle and explanation, empty when it sent neither.
    pub detail: String,
}

/// `asking`'s texts as the sheet shows them.
pub(super) fn words(asking: &PermissionAsking) -> Words {
    let ask = &asking.ask;
    let app = match ask.app_label.is_empty() {
        true => UNNAMED_APP.to_owned(),
        false => ask.app_label.clone(),
    };
    let title = match ask.title.is_empty() {
        true => format!("{app} is asking for permission"),
        false => ask.title.clone(),
    };
    let detail = [ask.subtitle.as_str(), ask.body.as_str()]
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let lead = match asking.by {
        AskedBy::Portal => Some(app),
        AskedBy::Compositor(_) => None,
    };
    Words {
        title,
        lead,
        detail,
    }
}

/// What Escape (or the sheet closing another way) answers. A portal question has a no button
/// that Escape presses. The compositor's question is dismissed instead: nothing is kept, so a
/// stray Escape does not become a refusal that lasts until the person undoes it in Settings.
pub(super) fn escape_answer(by: AskedBy) -> PermissionOutcome {
    match by {
        AskedBy::Portal => PermissionOutcome::Deny,
        AskedBy::Compositor(_) => PermissionOutcome::Closed,
    }
}

/// The installed app whose name is `label`, compared without regard to case: the portal knows an
/// app by its name, and the icon is the entry's.
pub(super) fn app_named<'a>(index: &'a AppIndex, label: &str) -> Option<&'a AppId> {
    let wanted = label.to_lowercase();
    if wanted.is_empty() {
        return None;
    }
    index
        .apps
        .iter()
        .find(|app| app.name.to_lowercase() == wanted)
        .map(|app| &app.id)
}

#[cfg(test)]
mod tests {
    use sill_model::capture_pick::{
        AskId, AskedBy, PermissionAsk, PermissionAsking, PermissionOutcome,
    };
    use sill_model::desktop::ConsentId;

    use super::{Words, escape_answer, words};

    fn ask(label: &str, title: &str, subtitle: &str, body: &str, by: AskedBy) -> PermissionAsking {
        PermissionAsking {
            id: AskId(1),
            ask: PermissionAsk::from_wire(label, title, subtitle, body, "", ""),
            by,
        }
    }

    #[test]
    fn the_headline_is_the_apps_own_else_a_sentence_and_the_detail_joins_what_is_sent() {
        let words_of = |title: &str, detail: &str, app: &str| Words {
            title: title.to_owned(),
            lead: Some(app.to_owned()),
            detail: detail.to_owned(),
        };
        let portal = AskedBy::Portal;
        let cases = [
            (
                "everything sent",
                ask(
                    "Notes",
                    "Share the screen?",
                    "Notes asks",
                    "To help.",
                    portal,
                ),
                words_of("Share the screen?", "Notes asks To help.", "Notes"),
            ),
            (
                "only a body",
                ask("Notes", "Share?", "", "To help.", portal),
                words_of("Share?", "To help.", "Notes"),
            ),
            (
                "no title",
                ask("Notes", "", "", "", portal),
                words_of("Notes is asking for permission", "", "Notes"),
            ),
            (
                "no name",
                ask("", "", "", "Why.", portal),
                words_of("An app is asking for permission", "Why.", "An app"),
            ),
        ];
        for (name, question, want) in cases {
            assert_eq!(words(&question), want, "{name}");
        }
    }

    #[test]
    fn the_compositors_question_does_not_repeat_the_program_under_its_headline() {
        let said = words(&ask(
            "grim",
            "\u{201c}grim\u{201d} would like to see your screen.",
            "",
            "It will be able to see everything.",
            AskedBy::Compositor(ConsentId::new(4)),
        ));
        assert_eq!(said.lead, None);
        assert_eq!(said.detail, "It will be able to see everything.");
    }

    #[test]
    fn escape_refuses_the_portal_and_dismisses_the_compositor() {
        let cases = [
            (AskedBy::Portal, PermissionOutcome::Deny),
            (
                AskedBy::Compositor(ConsentId::new(1)),
                PermissionOutcome::Closed,
            ),
        ];
        for (by, want) in cases {
            assert_eq!(escape_answer(by), want, "{by:?}");
        }
    }
}
