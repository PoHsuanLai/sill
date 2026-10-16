//! The program behind each app id on screen, as the compositor established it
//! (`quire_desktop_app_v1.subject`): the image path of a native process, the app id of a
//! sandboxed one. Unlike the app id a window declares, it is a fact, so the app-menu check can
//! compare it with the standing of the process that owns a menu's bus name (QUIRE-12).

use std::collections::BTreeMap;

/// The subject of each app id whose windows agree on one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Subjects(BTreeMap<String, String>);

impl Subjects {
    /// The subjects of the apps whose windows are `windows`, each an app id and the subject the
    /// compositor reported for that window's program (`None` when it reported none).
    ///
    /// An app id whose windows do not all report the same subject is left out: two programs that
    /// declare one id cannot vouch for it, and no subject is better than the wrong one.
    pub fn collect(windows: impl IntoIterator<Item = (String, Option<String>)>) -> Subjects {
        let mut seen: BTreeMap<String, Option<String>> = BTreeMap::new();
        for (app_id, subject) in windows {
            let agreed = match seen.get(&app_id) {
                Some(before) if *before != subject => None,
                _ => subject,
            };
            seen.insert(app_id, agreed);
        }
        Subjects(
            seen.into_iter()
                .filter_map(|(app_id, subject)| subject.map(|subject| (app_id, subject)))
                .collect(),
        )
    }

    /// The subject of the windows declaring `app_id`, when the compositor reported one.
    pub fn of(&self, app_id: &str) -> Option<&str> {
        self.0.get(app_id).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::Subjects;

    fn window(app_id: &str, subject: Option<&str>) -> (String, Option<String>) {
        (app_id.to_owned(), subject.map(str::to_owned))
    }

    #[test]
    fn an_app_has_the_subject_its_windows_agree_on() {
        let subjects = Subjects::collect([
            window("dev.notes", Some("/usr/bin/notes")),
            window("dev.notes", Some("/usr/bin/notes")),
            window("dev.mail", Some("org.example.Mail")),
            window("dev.none", None),
        ]);
        assert_eq!(subjects.of("dev.notes"), Some("/usr/bin/notes"));
        assert_eq!(subjects.of("dev.mail"), Some("org.example.Mail"));
        assert_eq!(subjects.of("dev.none"), None);
        assert_eq!(subjects.of("dev.unknown"), None);
    }

    #[test]
    fn an_app_id_two_programs_declare_has_no_subject_whatever_the_order() {
        let notes = Some("/usr/bin/notes");
        let other = Some("/usr/bin/other");
        for windows in [
            [window("dev.notes", notes), window("dev.notes", other)],
            [window("dev.notes", other), window("dev.notes", notes)],
            [window("dev.notes", notes), window("dev.notes", None)],
            [window("dev.notes", None), window("dev.notes", notes)],
        ] {
            assert_eq!(Subjects::collect(windows).of("dev.notes"), None);
        }
    }

    #[test]
    fn a_disagreement_is_not_forgiven_by_a_later_window() {
        let subjects = Subjects::collect([
            window("dev.notes", Some("/usr/bin/notes")),
            window("dev.notes", Some("/usr/bin/other")),
            window("dev.notes", Some("/usr/bin/notes")),
        ]);
        assert_eq!(subjects.of("dev.notes"), None);
    }
}
