//! What the sheet says, pure: the headline and the line under it, from the question's cleaned
//! texts, and which installed app the question names.

use sill_launcher::AppId;
use sill_model::apps::AppIndex;
use sill_model::capture_pick::PermissionAsk;

/// What an app with no name is called.
const UNNAMED_APP: &str = "An app";

/// The sheet's texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Words {
    /// The headline: the app's own, else a plain sentence naming the app.
    pub title: String,
    /// The app's name, set apart in the line under the headline.
    pub app: String,
    /// What follows the name: the app's subtitle and explanation, empty when it sent neither.
    pub detail: String,
}

/// `ask`'s texts as the sheet shows them.
pub(super) fn words(ask: &PermissionAsk) -> Words {
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
    Words { title, app, detail }
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
    use sill_model::capture_pick::PermissionAsk;

    use super::{Words, words};

    fn ask(label: &str, title: &str, subtitle: &str, body: &str) -> PermissionAsk {
        PermissionAsk::from_wire(label, title, subtitle, body, "", "")
    }

    #[test]
    fn the_headline_is_the_apps_own_else_a_sentence_and_the_detail_joins_what_is_sent() {
        let words_of = |title: &str, detail: &str, app: &str| Words {
            title: title.to_owned(),
            app: app.to_owned(),
            detail: detail.to_owned(),
        };
        let cases = [
            (
                "everything sent",
                ask("Notes", "Share the screen?", "Notes asks", "To help."),
                words_of("Share the screen?", "Notes asks To help.", "Notes"),
            ),
            (
                "only a body",
                ask("Notes", "Share?", "", "To help."),
                words_of("Share?", "To help.", "Notes"),
            ),
            (
                "no title",
                ask("Notes", "", "", ""),
                words_of("Notes is asking for permission", "", "Notes"),
            ),
            (
                "no name",
                ask("", "", "", "Why."),
                words_of("An app is asking for permission", "Why.", "An app"),
            ),
        ];
        for (name, question, want) in cases {
            assert_eq!(words(&question), want, "{name}");
        }
    }
}
