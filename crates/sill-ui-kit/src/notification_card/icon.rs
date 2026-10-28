//! Which app a notification came from, and the icon its card shows: the sender's `app_icon`
//! (a file, or a theme name the caller resolves), else its desktop entry's icon, else a bell.
//! The entry is found as a window's is (`sill_model::windows::app_for`: the `desktop-entry` hint,
//! else the app name, whole or by its last labels), then by the entry's own `Name`.

use std::path::PathBuf;

use sill_model::apps::{App, AppIcon, AppIndex};
use sill_model::notifications::{AppRef, IconRef};
use sill_model::windows::app_for;

use crate::app_icons::IconFile;

/// The desktop entry `app` names, if the index has it.
pub fn entry_for<'a>(app: &AppRef, index: &'a AppIndex) -> Option<&'a App> {
    let claimed = app.desktop_entry.as_deref().unwrap_or(app.name.as_str());
    app_for(claimed, index).entry(index).or_else(|| {
        let name = app.name.trim();
        index
            .apps
            .iter()
            .find(|entry| !name.is_empty() && entry.name.eq_ignore_ascii_case(name))
    })
}

/// The icon file `app`'s card draws, `entry` being its desktop entry and `lookup` the icon theme;
/// `None` draws quire's bell.
pub fn card_art(
    app: &AppRef,
    entry: Option<&App>,
    lookup: impl Fn(&str) -> Option<PathBuf>,
) -> Option<IconFile> {
    let sent = match &app.icon {
        IconRef::File(path) => Some(path.clone()),
        IconRef::Name(name) => lookup(name),
        IconRef::None => None,
    };
    let own = entry.and_then(|entry| match &entry.icon {
        AppIcon::File(path) => Some(path.clone()),
        AppIcon::None => None,
    });
    sent.or(own).and_then(|path| IconFile::of(&path))
}

#[cfg(test)]
mod tests {
    use super::{card_art, entry_for};
    use crate::app_icons::IconFile;
    use sill_launcher::AppId;
    use sill_model::apps::{App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal};
    use sill_model::notifications::{AppRef, IconRef};
    use std::path::PathBuf;

    fn entry(id: &str, name: &str, icon: &str) -> App {
        App {
            id: AppId(id.to_owned()),
            name: name.to_owned(),
            generic_name: String::new(),
            comment: String::new(),
            exec: id.to_owned(),
            icon: AppIcon::File(PathBuf::from(icon)),
            startup_wm_class: None,
            flatpak: None,
            bus_names: Vec::new(),
            intents_app: None,
            renamed_from: Vec::new(),
            terminal: Terminal::No,
            activation: DbusActivation::Exec,
            actions: Vec::new(),
            mime_types: Vec::new(),
            categories: Vec::new(),
            keyboard_profile: None,
            keywords: Vec::new(),
            listing: Listing::Shown,
            path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
        }
    }

    fn index() -> AppIndex {
        sill_model::apps::step(
            AppIndex::default(),
            AppsEvent::Scanned(vec![
                entry("org.gnome.Evolution", "Evolution", "/i/evolution.svg"),
                entry("firefox", "Firefox", "/i/firefox.svg"),
            ]),
        )
    }

    fn app(name: &str, entry: Option<&str>, icon: IconRef) -> AppRef {
        AppRef {
            name: name.to_owned(),
            icon,
            desktop_entry: entry.map(str::to_owned),
        }
    }

    #[test]
    fn the_entry_is_found_by_hint_id_or_name() {
        let index = index();
        let id = |a: &AppRef| entry_for(a, &index).map(|e| e.id.0.clone());
        let cases = [
            (
                app("x", Some("org.gnome.Evolution"), IconRef::None),
                Some("org.gnome.Evolution"),
            ),
            (
                app("evolution", None, IconRef::None),
                Some("org.gnome.Evolution"),
            ),
            (app("Firefox", None, IconRef::None), Some("firefox")),
            (app("notify-send", None, IconRef::None), None),
            (app("", None, IconRef::None), None),
        ];
        for (a, want) in cases {
            assert_eq!(id(&a).as_deref(), want, "{a:?}");
        }
    }

    #[test]
    fn the_senders_icon_wins_then_the_entrys_then_the_bell() {
        let index = index();
        let lookup = |name: &str| (name == "mail-unread").then(|| PathBuf::from("/t/mail.svg"));
        let art = |a: AppRef| card_art(&a, entry_for(&a, &index), lookup);
        let cases = [
            (
                app("firefox", None, IconRef::File("/s/own.png".into())),
                Some(IconFile::Themed("/s/own.png".into())),
            ),
            (
                app("firefox", None, IconRef::Name("mail-unread".to_owned())),
                Some(IconFile::Themed("/t/mail.svg".into())),
            ),
            (
                app("firefox", None, IconRef::Name("unknown".to_owned())),
                Some(IconFile::Themed("/i/firefox.svg".into())),
            ),
            (
                app("x", None, IconRef::File("/s/bell-symbolic.svg".into())),
                Some(IconFile::Symbolic("/s/bell-symbolic.svg".into())),
            ),
            (app("x", None, IconRef::None), None),
        ];
        for (a, want) in cases {
            assert_eq!(art(a.clone()), want, "{a:?}");
        }
    }
}
