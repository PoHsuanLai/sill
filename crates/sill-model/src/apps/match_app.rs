//! Which desktop entry a window belongs to: `match_app(app_id, wm_class, &AppIndex)` (exact,
//! lowercase, StartupWMClass, reverse-DNS suffix, Flatpak id).
//!
//! Every rule compares whole tokens: an id is compared whole or by whole dot-separated labels,
//! never by substring (CONVENTIONS "Substrings are not tokens": `fox` must not match
//! `org.mozilla.firefox`).

use sill_launcher::AppId;

use super::{App, AppIndex};

/// The entry a toplevel with Wayland `app_id` (and, through XWayland, `wm_class`) belongs to.
/// The first rule that matches any entry wins; within a rule, the first entry in id order.
pub fn match_app(app_id: &str, wm_class: Option<&str>, index: &AppIndex) -> Option<AppId> {
    let app_id = app_id.trim();
    if app_id.is_empty() && wm_class.is_none_or(|c| c.trim().is_empty()) {
        return None;
    }
    let lower = app_id.to_lowercase();
    let rules: [&dyn Fn(&App) -> bool; 5] = [
        &|app| app.id.0 == app_id,
        &|app| app.id.0.to_lowercase() == lower,
        &|app| {
            app.startup_wm_class.as_deref().is_some_and(|class| {
                same_class(class, app_id) || wm_class.is_some_and(|w| same_class(class, w))
            })
        },
        &|app| !lower.is_empty() && is_label_suffix(&app.id.0.to_lowercase(), &lower),
        &|app| {
            app.flatpak
                .as_deref()
                .is_some_and(|flatpak| !lower.is_empty() && flatpak.to_lowercase() == lower)
        },
    ];
    rules
        .iter()
        .find_map(|rule| index.apps.iter().find(|app| rule(app)))
        .map(|app| app.id.clone())
}

/// `StartupWMClass` compares whole, ignoring case; an empty class matches nothing.
fn same_class(class: &str, given: &str) -> bool {
    let given = given.trim();
    !given.is_empty() && class.eq_ignore_ascii_case(given)
}

/// Whether `suffix` is the last whole dot-separated labels of `id`: `firefox` of
/// `org.mozilla.firefox`, `mozilla.firefox` of it too, but not `fox`.
fn is_label_suffix(id: &str, suffix: &str) -> bool {
    id.len() > suffix.len() && id.ends_with(suffix) && id[..id.len() - suffix.len()].ends_with('.')
}

#[cfg(test)]
mod tests {
    use super::match_app;
    use crate::apps::index::tests::app;
    use crate::apps::{App, AppIndex, AppsEvent, step};

    fn index() -> AppIndex {
        let with = |mut a: App, edit: fn(&mut App)| {
            edit(&mut a);
            a
        };
        step(
            AppIndex::default(),
            AppsEvent::Scanned(vec![
                app("org.mozilla.firefox", "Firefox"),
                app("org.gnome.Nautilus", "Files"),
                app("foot", "Foot"),
                with(app("code", "Visual Studio Code"), |a| {
                    a.startup_wm_class = Some("Code".to_owned())
                }),
                with(app("com.valvesoftware.Steam", "Steam"), |a| {
                    a.startup_wm_class = Some("steam".to_owned());
                    a.flatpak = Some("com.valvesoftware.Steam".to_owned());
                }),
                with(app("spotify-launcher", "Spotify"), |a| {
                    a.flatpak = Some("com.spotify.Client".to_owned())
                }),
                app("org.kde.fox", "Fox"),
                app("net.lutris.Lutris", "Lutris"),
                app("lutris", "Lutris (native)"),
            ]),
        )
    }

    #[test]
    fn each_rule_matches_in_order() {
        const CASES: &[(&str, &str, Option<&str>, Option<&str>)] = &[
            ("exact", "foot", None, Some("foot")),
            (
                "exact reverse-dns",
                "org.gnome.Nautilus",
                None,
                Some("org.gnome.Nautilus"),
            ),
            (
                "lowercase",
                "org.gnome.nautilus",
                None,
                Some("org.gnome.Nautilus"),
            ),
            ("lowercase simple", "FOOT", None, Some("foot")),
            ("startup wm class by app id", "code", None, Some("code")),
            ("startup wm class, case ignored", "CODE", None, Some("code")),
            (
                "startup wm class by wm_class",
                "",
                Some("steam"),
                Some("com.valvesoftware.Steam"),
            ),
            (
                "reverse-dns suffix",
                "firefox",
                None,
                Some("org.mozilla.firefox"),
            ),
            (
                "reverse-dns suffix of two labels",
                "mozilla.firefox",
                None,
                Some("org.mozilla.firefox"),
            ),
            (
                "flatpak id",
                "com.spotify.Client",
                None,
                Some("spotify-launcher"),
            ),
            (
                "flatpak id, case ignored",
                "com.spotify.client",
                None,
                Some("spotify-launcher"),
            ),
            ("an exact id beats a suffix", "lutris", None, Some("lutris")),
            ("a substring is not a label", "irefox", None, None),
            ("a label is whole", "fox", None, Some("org.kde.fox")),
            ("unknown", "org.example.Nothing", None, None),
            ("empty", "", None, None),
            ("blank wm class", " ", Some(" "), None),
        ];
        let index = index();
        for &(name, app_id, wm_class, want) in CASES {
            assert_eq!(
                match_app(app_id, wm_class, &index).map(|id| id.0),
                want.map(str::to_owned),
                "{name}: {app_id:?} / {wm_class:?}"
            );
        }
    }

    #[test]
    fn a_rule_earlier_in_the_list_wins_over_a_later_one() {
        // `steam` is both a StartupWMClass (rule 3) and nothing else; `Steam` app id by suffix
        // (rule 4) would find the same entry; the flatpak id (rule 5) too. One answer.
        let index = index();
        for app_id in ["steam", "Steam", "com.valvesoftware.Steam"] {
            assert_eq!(
                match_app(app_id, None, &index).map(|id| id.0),
                Some("com.valvesoftware.Steam".to_owned()),
                "{app_id}"
            );
        }
    }
}
