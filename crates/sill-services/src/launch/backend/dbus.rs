//! The D-Bus half of launching: `org.freedesktop.Application.Activate` for a `DBusActivatable`
//! entry, and `org.freedesktop.FileManager1.ShowItems` for "Show in Files" (dock Cmd-click).

use std::collections::HashMap;

use ds_core::word::Word;
use sill_model::apps::App;
use sill_model::launch::{LaunchRequest, SETTINGS_BUS_NAME, SettingsAccount, SettingsPage};
use zbus::Connection;
use zbus::zvariant::Value;

use super::uri::file_uri;

/// `org.freedesktop.Application.Activate` on the bus name `app.id`, object path derived from it
/// (GApplication's own convention: each `.` becomes `/`, each `-` becomes `_`, prefixed with
/// `/`), with the activation token as `platform_data["desktop-startup-id"]` when one was
/// granted.
pub async fn activate(
    connection: &Connection,
    app: &App,
    request: &LaunchRequest,
) -> zbus::Result<()> {
    let mut platform_data: HashMap<&str, Value<'_>> = HashMap::new();
    if let Some(token) = &request.token {
        platform_data.insert("desktop-startup-id", Value::from(token.as_str()));
    }
    connection
        .call_method(
            Some(app.id.0.as_str()),
            object_path(&app.id.0).as_str(),
            Some("org.freedesktop.Application"),
            "Activate",
            &(platform_data,),
        )
        .await?;
    Ok(())
}

/// `org.freedesktop.Application.ActivateAction("open-account", [path])` on `org.quire.Settings`:
/// the bus starts it as its own unit when it is not running, and Settings opens at the account.
pub async fn open_settings(connection: &Connection, account: &SettingsAccount) -> zbus::Result<()> {
    settings_action(connection, "open-account", &account.0).await
}

/// `org.freedesktop.Application.ActivateAction("open-page", [slug])` on `org.quire.Settings`:
/// Settings opens at the page the slug names.
pub async fn open_settings_page(connection: &Connection, page: SettingsPage) -> zbus::Result<()> {
    settings_action(connection, "open-page", page.slug()).await
}

/// One action of Settings with its one string parameter.
async fn settings_action(connection: &Connection, action: &str, target: &str) -> zbus::Result<()> {
    let platform_data: HashMap<&str, Value<'_>> = HashMap::new();
    connection
        .call_method(
            Some(SETTINGS_BUS_NAME),
            object_path(SETTINGS_BUS_NAME).as_str(),
            Some("org.freedesktop.Application"),
            "ActivateAction",
            &(action, vec![Value::from(target)], platform_data),
        )
        .await?;
    Ok(())
}

/// `org.freedesktop.FileManager1.ShowItems([file:// URI of the entry's own .desktop file], "")`.
/// The URI is percent-encoded (a space or `#` in the path would otherwise break it).
pub async fn show_in_files(connection: &Connection, app: &App) -> zbus::Result<()> {
    let uri = file_uri(&app.path);
    connection
        .call_method(
            Some("org.freedesktop.FileManager1"),
            "/org/freedesktop/FileManager1",
            Some("org.freedesktop.FileManager1"),
            "ShowItems",
            &(vec![uri], String::new()),
        )
        .await?;
    Ok(())
}

/// `/` followed by `id` with every `.` turned into `/` and every `-` into `_` (D-Bus object
/// paths may not contain `-`): the object path a `DBusActivatable` entry's own id maps to.
fn object_path(id: &str) -> String {
    let mut path = String::with_capacity(id.len() + 1);
    path.push('/');
    for c in id.chars() {
        path.push(match c {
            '.' => '/',
            '-' => '_',
            other => other,
        });
    }
    path
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{file_uri, object_path};

    #[test]
    fn a_desktop_files_uri_is_percent_encoded() {
        const CASES: &[(&str, &str)] = &[
            (
                "/usr/share/applications/org.mozilla.firefox.desktop",
                "file:///usr/share/applications/org.mozilla.firefox.desktop",
            ),
            (
                "/home/p/.local/share/applications/My App.desktop",
                "file:///home/p/.local/share/applications/My%20App.desktop",
            ),
            (
                "/home/p/apps/c#-ide.desktop",
                "file:///home/p/apps/c%23-ide.desktop",
            ),
        ];
        for (path, want) in CASES {
            assert_eq!(file_uri(Path::new(path)), *want, "{path}");
        }
    }

    #[test]
    fn object_paths_replace_dots_and_dashes() {
        const CASES: &[(&str, &str)] = &[
            ("org.mozilla.firefox", "/org/mozilla/firefox"),
            ("foot", "/foot"),
            ("spotify-launcher", "/spotify_launcher"),
        ];
        for (id, want) in CASES {
            assert_eq!(object_path(id), *want, "{id}");
        }
    }
}
