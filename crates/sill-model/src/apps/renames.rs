//! Old desktop ids and the apps that took them over (`X-Flatpak-RenamedFrom`), so what a person
//! stored under an id that was renamed is read back as the app it now is.

use std::collections::BTreeMap;

use sill_launcher::AppId;

use super::AppIndex;

/// Every old id an installed app claims, with the id of the app that claims it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Renames(BTreeMap<AppId, AppId>);

impl Renames {
    /// The id `id` stands for now: the app that lists it in `RenamedFrom`, else `id` itself.
    pub fn of(&self, id: &AppId) -> AppId {
        self.0.get(id).unwrap_or(id).clone()
    }

    /// Whether no installed app was renamed.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl AppIndex {
    /// The renames of the installed apps. An id an installed app has itself is never renamed
    /// away from it, and two apps claiming one old id leave it to the first by id.
    pub fn renames(&self) -> Renames {
        let mut renames = BTreeMap::new();
        for app in &self.apps {
            for old in &app.renamed_from {
                if self.get(old).is_none() {
                    renames.entry(old.clone()).or_insert_with(|| app.id.clone());
                }
            }
        }
        Renames(renames)
    }

    /// The id `id` stands for now (see [`Renames::of`]).
    pub fn current(&self, id: &AppId) -> AppId {
        self.renames().of(id)
    }
}

#[cfg(test)]
mod tests {
    use sill_launcher::AppId;

    use crate::apps::{AppIndex, AppsEvent, step, test_app};

    fn id(text: &str) -> AppId {
        AppId(text.to_owned())
    }

    /// An entry `new` that was `old` before.
    fn renamed(new: &str, old: &[&str]) -> crate::apps::App {
        let mut app = test_app(new, new);
        app.renamed_from = old.iter().map(|o| id(o)).collect();
        app
    }

    #[test]
    fn an_old_id_stands_for_the_app_that_lists_it_unless_an_app_has_that_id_itself() {
        type Case<'a> = (&'a str, Vec<crate::apps::App>, &'a str, &'a str);
        let cases: Vec<Case> = vec![
            (
                "renamed",
                vec![renamed("org.quire.Mail", &["mailo"])],
                "mailo",
                "org.quire.Mail",
            ),
            (
                "one of several old ids",
                vec![renamed("org.quire.Mail", &["mailo", "mail"])],
                "mail",
                "org.quire.Mail",
            ),
            (
                "an installed id wins over the claim",
                vec![
                    renamed("org.quire.Mail", &["mailo"]),
                    test_app("mailo", "Mailo"),
                ],
                "mailo",
                "mailo",
            ),
            (
                "the first claimant by id wins",
                vec![renamed("b.App", &["old"]), renamed("a.App", &["old"])],
                "old",
                "a.App",
            ),
            (
                "an unknown id stays",
                vec![renamed("org.quire.Mail", &["mailo"])],
                "other",
                "other",
            ),
            (
                "an app that renamed nothing",
                vec![test_app("a", "A")],
                "a",
                "a",
            ),
        ];
        for (name, apps, asked, want) in cases {
            let index = step(AppIndex::default(), AppsEvent::Scanned(apps));
            assert_eq!(index.current(&id(asked)), id(want), "{name}");
        }
    }

    #[test]
    fn an_index_without_claims_has_no_renames() {
        let index = step(
            AppIndex::default(),
            AppsEvent::Scanned(vec![test_app("a", "A")]),
        );
        assert!(index.renames().is_empty());
    }
}
