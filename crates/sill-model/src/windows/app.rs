//! Which app a window belongs to: the one place the fallback is decided.

use sill_launcher::AppId;

use super::Toplevel;
use crate::apps::{App, AppIndex};

/// The app a window (or a bare app id, such as a notification's) belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMatch {
    /// The shared matcher found a desktop entry.
    Entry(AppId),
    /// No entry matched: the raw app id stands for the app, so it still gets a tile and a name.
    Unmatched(String),
}

impl AppMatch {
    /// The entry's id, or the raw app id as an `AppId`.
    pub fn id(&self) -> AppId {
        match self {
            Self::Entry(id) => id.clone(),
            Self::Unmatched(raw) => AppId(raw.clone()),
        }
    }

    /// The desktop entry in `index`, when one matched.
    pub fn entry<'a>(&self, index: &'a AppIndex) -> Option<&'a App> {
        match self {
            Self::Entry(id) => index.get(id),
            Self::Unmatched(_) => None,
        }
    }

    /// The entry's name, else the raw app id.
    pub fn name(&self, index: &AppIndex) -> String {
        match (self.entry(index), self) {
            (Some(app), _) => app.name.clone(),
            (None, Self::Unmatched(raw)) => raw.clone(),
            (None, Self::Entry(id)) => id.0.clone(),
        }
    }
}

/// The app for the Wayland app id `raw`.
pub fn app_for(raw: &str, index: &AppIndex) -> AppMatch {
    match index.matching(raw, None) {
        Some(app) => AppMatch::Entry(app.id.clone()),
        None => AppMatch::Unmatched(raw.to_owned()),
    }
}

/// The name the app of the Wayland app id `raw` has in the intents manifests: what its entry
/// declares in `X-Quire-IntentsApp`, else `raw` itself when that is a valid name (an app whose
/// window id is its bus name, the case before entries declared one). None when neither names it.
pub fn intents_app_for(raw: &str, index: &AppIndex) -> Option<prov::AppName> {
    let declared = index
        .matching(raw, None)
        .and_then(|app| app.intents_app.as_deref());
    prov::AppName::parse(declared.unwrap_or(raw)).ok()
}

/// The app `window` belongs to.
pub fn app_of(window: &Toplevel, index: &AppIndex) -> AppMatch {
    app_for(&window.app_id, index)
}

#[cfg(test)]
mod tests {
    use sill_launcher::AppId;

    use super::{AppMatch, app_for};
    use crate::apps::{AppIndex, test_app};

    #[test]
    fn an_app_is_its_entry_or_its_raw_id() {
        let index = AppIndex::new(vec![test_app("org.example.Term", "Term")]);
        let cases = [
            (
                "an exact id",
                "org.example.Term",
                AppMatch::Entry(AppId("org.example.Term".to_owned())),
                "Term",
            ),
            (
                "no entry",
                "ghost",
                AppMatch::Unmatched("ghost".to_owned()),
                "ghost",
            ),
            ("empty", "", AppMatch::Unmatched(String::new()), ""),
        ];
        for (name, raw, want, want_name) in cases {
            let got = app_for(raw, &index);
            assert_eq!(got, want, "{name}");
            assert_eq!(got.name(&index), want_name, "{name}");
        }
    }
}
