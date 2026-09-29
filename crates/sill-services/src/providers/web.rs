//! "Search the web for …": one row, last, for any typed text while `launcher.web_search` is on,
//! opening `launcher.web_engine_url` with `{query}` replaced by the percent-encoded text.
//! Suggestions from the engine are later and off.
//!
//! The row's id is fixed (`web:search`), never the query, so frecency counts the fallback's
//! use without writing what was searched to disk. Enter and ⌘B open the address
//! (`Activation::Open { Url }`).

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use sill_launcher::{
    Action, ActionId, ActionKeys, Activation, Batch, Choice, IconRef, Item, ItemId, OpenTarget,
    Provider, ProviderKind, Query, ResultSink, Score, Subject, WebFacts, chords,
};
use tokio::sync::watch;

use crate::results::{LauncherPolicy, WebPolicy};

/// The fallback row's id.
pub const WEB_ROW_ID: &str = "web:search";

/// The placeholder a template's query goes in.
pub const QUERY_PLACEHOLDER: &str = "{query}";

/// The row's one action's id (Enter does the same thing; the action just carries the chord).
const SEARCH_WEB_ACTION: &str = "search_web";

/// A single row is always the sole, most relevant row this provider offers.
const ROW_SCORE: Score = Score(1000);

/// RFC 3986 unreserved (letters, digits, `-`, `.`, `_`, `~`) kept literal; everything else,
/// including space, is percent-encoded.
const UNRESERVED: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Why a template cannot make an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateFault {
    /// No `{query}` in it.
    NoPlaceholder,
    /// Not `https://` (or `http://`): the launcher opens nothing else from a setting.
    NotWeb,
}

/// The web fallback.
#[derive(Debug, Clone)]
pub struct WebProvider {
    pub policy: watch::Receiver<LauncherPolicy>,
}

impl Provider for WebProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Web
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        let WebPolicy::On { template } = self.policy.borrow().web.clone() else {
            return;
        };
        let text = query.text.trim();
        if text.is_empty() {
            return;
        }
        let Ok(item) = web_item(text, &template) else {
            // A template that refuses (no `{query}`, or not `https`/`http`) offers nothing; it is
            // never a reason to fail the launcher.
            return;
        };

        if query.is_cancelled() {
            return;
        }
        sink.push(Batch {
            provider: ProviderKind::Web,
            generation: query.generation,
            items: vec![item],
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        match (&item.subject, choice) {
            (Subject::Web(web), Choice::Primary) => Activation::Open {
                target: OpenTarget::Url(web.url.clone()),
            },
            _ => Activation::Nothing,
        }
    }
}

/// `template` with `{query}` replaced by `text`, percent-encoded (RFC 3986 unreserved kept,
/// space as `%20`).
pub fn search_url(template: &str, text: &str) -> Result<String, TemplateFault> {
    if !template.contains(QUERY_PLACEHOLDER) {
        return Err(TemplateFault::NoPlaceholder);
    }
    if !(template.starts_with("https://") || template.starts_with("http://")) {
        return Err(TemplateFault::NotWeb);
    }
    let encoded = utf8_percent_encode(text, UNRESERVED).to_string();
    Ok(template.replace(QUERY_PLACEHOLDER, &encoded))
}

/// The row for `text` with `template` (title `Search the web for “text”`, subtitle the
/// engine's host, subject `Subject::Web`, glyph `search`, one action “Search the web” ⌘B).
pub fn web_item(text: &str, template: &str) -> Result<Item, TemplateFault> {
    let url = search_url(template, text)?;
    let engine = host_of(template);
    Ok(Item {
        id: ItemId(WEB_ROW_ID.to_owned()),
        provider: ProviderKind::Web,
        title: format!("Search the web for \u{201c}{text}\u{201d}"),
        subtitle: engine.clone(),
        icon: IconRef::Glyph("search".to_owned()),
        score: ROW_SCORE,
        actions: vec![Action {
            id: ActionId(SEARCH_WEB_ACTION.to_owned()),
            label: "Search the web".to_owned(),
            keys: ActionKeys::Chord(chords::search_web()),
        }],
        subject: Subject::Web(WebFacts { engine, url }),
    })
}

/// `template`'s host (`https://duckduckgo.com/?q={query}` -> `duckduckgo.com`), for the row's
/// subtitle and `WebFacts::engine`. Called only once `search_url` has already accepted the
/// template's scheme.
fn host_of(template: &str) -> String {
    let rest = template
        .strip_prefix("https://")
        .or_else(|| template.strip_prefix("http://"))
        .unwrap_or(template);
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{TemplateFault, search_url};

    #[test]
    fn the_query_is_percent_encoded_into_the_template() {
        let ddg = "https://duckduckgo.com/?q={query}";
        let cases: Vec<(&str, &str, Result<String, TemplateFault>)> = vec![
            (
                ddg,
                "rust wayland",
                Ok("https://duckduckgo.com/?q=rust%20wayland".to_owned()),
            ),
            (
                ddg,
                "a&b=c#d",
                Ok("https://duckduckgo.com/?q=a%26b%3Dc%23d".to_owned()),
            ),
            (
                ddg,
                "日本",
                Ok("https://duckduckgo.com/?q=%E6%97%A5%E6%9C%AC".to_owned()),
            ),
            (
                "https://example.com/search",
                "x",
                Err(TemplateFault::NoPlaceholder),
            ),
            ("file:///etc/{query}", "passwd", Err(TemplateFault::NotWeb)),
            ("javascript:{query}", "x", Err(TemplateFault::NotWeb)),
        ];
        for (template, text, want) in cases {
            assert_eq!(search_url(template, text), want, "{template} {text}");
        }
    }
}
