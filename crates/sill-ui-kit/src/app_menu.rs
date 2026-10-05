//! Where an app's exported menu is served: quire's `AppMenuAddress`, the one derivation, as the
//! function the menu service and the agent's actions are given (a services crate links no
//! design system).

use ds::components::menus::export::AppMenuAddress;
use sill_model::app_menu::{MenuApp, MenuName};

/// The bus name `app`'s menu is served at, or `None` for an id no name can be made from.
pub fn menu_name_of(app: &MenuApp) -> Option<MenuName> {
    AppMenuAddress::for_app_id(&app.0)
        .ok()
        .map(|address| MenuName(address.service))
}

#[cfg(test)]
mod tests {
    use sill_model::app_menu::MenuApp;

    use super::menu_name_of;

    #[test]
    fn a_name_is_what_quire_derives_from_the_app_id() {
        const CASES: &[(&str, Option<&str>)] = &[
            ("dev.mailo.Mailo", Some("org.quire.AppMenu.dev.mailo.Mailo")),
            ("test.app", Some("org.quire.AppMenu.test.app")),
            ("foot", Some("org.quire.AppMenu.foot")),
            ("org.gnome.gedit-2", Some("org.quire.AppMenu.org.gnome.gedit-2")),
            ("1password", Some("org.quire.AppMenu._1password")),
            ("a b.c", Some("org.quire.AppMenu.a_b.c")),
            ("", None),
        ];
        for &(app, want) in CASES {
            let got = menu_name_of(&MenuApp(app.to_owned())).map(|name| name.0);
            assert_eq!(got.as_deref(), want, "{app:?}");
        }
    }
}
