//! "Search the web for …" (M9 lane c, FINDINGS "M9 freeze" F650): one row, last, for any typed
//! text while `launcher.web_search` is on, opening `launcher.web_engine_url` with `{query}`
//! replaced by the percent-encoded text. Suggestions from the engine are later and off.
//!
//! The row's id is fixed (`web:search`), never the query, so frecency counts the fallback's
//! use without writing what was searched to disk. Enter and ⌘B open the address
//! (`Activation::Open { Url }`).

use sill_launcher::{
    Activation, Choice, Item, OpenTarget, Provider, ProviderKind, Query, ResultSink, Subject,
};
use tokio::sync::watch;

use crate::results::LauncherPolicy;

/// The fallback row's id.
pub const WEB_ROW_ID: &str = "web:search";

/// The placeholder a template's query goes in.
pub const QUERY_PLACEHOLDER: &str = "{query}";

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
        let _ = (query, sink);
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
/// space as `%20`). Lane c fills it.
pub fn search_url(template: &str, text: &str) -> Result<String, TemplateFault> {
    let _ = (template, text);
    todo!("M9 fill: c-emoji-web-units")
}

/// The row for `text` with `template` (title `Search the web for “text”`, subtitle the
/// engine's host, subject `Subject::Web`, glyph `search`, one action "Search the web" ⌘B).
pub fn web_item(text: &str, template: &str) -> Result<Item, TemplateFault> {
    let _ = (text, template);
    todo!("M9 fill: c-emoji-web-units")
}

#[cfg(test)]
mod tests {
    use super::{TemplateFault, search_url};

    #[test]
    #[ignore = "M9 fill: c-emoji-web-units"]
    fn the_query_is_percent_encoded_into_the_template() {
        let ddg = "https://duckduckgo.com/?q={query}";
        let cases: Vec<(&str, &str, Result<String, TemplateFault>)> = vec![
            (ddg, "rust wayland", Ok("https://duckduckgo.com/?q=rust%20wayland".to_owned())),
            (ddg, "a&b=c#d", Ok("https://duckduckgo.com/?q=a%26b%3Dc%23d".to_owned())),
            (ddg, "日本", Ok("https://duckduckgo.com/?q=%E6%97%A5%E6%9C%AC".to_owned())),
            ("https://example.com/search", "x", Err(TemplateFault::NoPlaceholder)),
            ("file:///etc/{query}", "passwd", Err(TemplateFault::NotWeb)),
            ("javascript:{query}", "x", Err(TemplateFault::NotWeb)),
        ];
        for (template, text, want) in cases {
            assert_eq!(search_url(template, text), want, "{template} {text}");
        }
    }
}
