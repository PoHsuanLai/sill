//! The D-Bus half of launching: `org.freedesktop.Application.Activate` for a `DBusActivatable`
//! entry, and `org.freedesktop.FileManager1.ShowItems` for "Show in Files" (dock Cmd-click).

use std::collections::HashMap;

use zbus::Connection;
use zbus::zvariant::Value;

use crate::apps::App;
use crate::launch::LaunchRequest;

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

/// `org.freedesktop.FileManager1.ShowItems([file:// URI of the entry's own .desktop file], "")`.
pub async fn show_in_files(connection: &Connection, app: &App) -> zbus::Result<()> {
    let uri = format!("file://{}", app.path.display());
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
    use super::object_path;

    #[test]
    fn object_paths_replace_dots_and_dashes() {
        const CASES: &[(&str, &str)] = &[
            ("org.mozilla.firefox", "/org/mozilla/firefox"),
            ("com.system76.CosmicTerm", "/com/system76/CosmicTerm"),
            ("spotify-launcher", "/spotify_launcher"),
        ];
        for (id, want) in CASES {
            assert_eq!(object_path(id), *want, "{id}");
        }
    }
}
