//! Pure launch planning: the argument vector and the scope's unit name.

use sill_launcher::{AppId, DesktopAction};

use super::{LaunchRequest, ScopeUnit};
use crate::apps::{App, AppIcon, ExecArgs, ExecError, Terminal, expand_exec};

/// The full command: `systemd-run --user --scope --collect --unit=<unit> -- <exec...>`, with
/// the terminal wrapper (`terminal`, e.g. `["foot", "-e"]`) in front of `Exec` when the
/// entry has `Terminal=true`, and the request's action's `Exec` when it names one.
pub fn launch_argv(
    app: &App,
    request: &LaunchRequest,
    unit: &ScopeUnit,
    terminal: &[String],
) -> Result<Vec<String>, ExecError> {
    let exec = action_exec(app, &request.action);
    let args = ExecArgs {
        files: request.files.clone(),
        uris: request.uris.clone(),
        icon: icon_arg(&app.icon),
        name: app.name.clone(),
        desktop_file: Some(app.path.clone()),
    };
    let mut command = expand_exec(exec, &args)?;
    if app.terminal == Terminal::Yes {
        let mut wrapped = terminal.to_vec();
        wrapped.append(&mut command);
        command = wrapped;
    }
    let mut argv = vec![
        "systemd-run".to_owned(),
        "--user".to_owned(),
        "--scope".to_owned(),
        "--collect".to_owned(),
        format!("--unit={}", unit.0),
        "--".to_owned(),
    ];
    argv.extend(command);
    Ok(argv)
}

/// The `Exec` line for `action`: the entry's own for [`DesktopAction::Default`], or the named
/// `[Desktop Action <id>]`'s. A named action the entry does not have falls back to the entry's
/// own `Exec` (the caller offered an action id we no longer recognise, not a reason to refuse
/// the launch outright).
fn action_exec<'a>(app: &'a App, action: &DesktopAction) -> &'a str {
    match action {
        DesktopAction::Default => &app.exec,
        DesktopAction::Named(id) => app
            .actions
            .iter()
            .find(|entry| &entry.id == id)
            .map_or(app.exec.as_str(), |entry| entry.exec.as_str()),
    }
}

/// `%i`'s value: the entry's resolved icon file (there is no reason to prefer the file over the
/// bare `Icon=` name here, since `App` keeps only the resolved [`AppIcon`]), or empty for none.
fn icon_arg(icon: &AppIcon) -> String {
    match icon {
        AppIcon::File(path) => path.to_string_lossy().into_owned(),
        AppIcon::None => String::new(),
    }
}

/// `app-sill-<id>-<nonce>.scope`, the id escaped the way `systemd-escape` escapes a unit name
/// component (XDG "Desktop Entry / systemd integration" naming): `/` becomes `-`; a leading `.`
/// and every byte outside `[A-Za-z0-9:_.]` (a literal `-` included, since `-` is reserved for an
/// escaped `/`) becomes `\xHH`.
pub fn scope_unit(app: &AppId, nonce: u32) -> ScopeUnit {
    ScopeUnit(format!(
        "app-sill-{}-{nonce}.scope",
        escape_component(&app.0)
    ))
}

fn escape_component(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for (i, byte) in id.bytes().enumerate() {
        let plain = byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'.');
        if byte == b'/' {
            out.push('-');
        } else if plain && !(i == 0 && byte == b'.') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("\\x{byte:02x}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{escape_component, launch_argv, scope_unit};
    use crate::apps::{
        App, AppIcon, DbusActivation, DesktopActionEntry, ExecError, Listing, Terminal,
    };
    use crate::launch::LaunchRequest;
    use crate::launch::ScopeUnit;
    use sill_launcher::{AppId, DesktopAction};
    use std::path::PathBuf;

    fn id(s: &str) -> AppId {
        AppId(s.to_owned())
    }

    /// A minimal entry with `id`, `name` and an empty `exec`, for this module's tests.
    fn bare_app(id_str: &str, name: &str) -> App {
        App {
            id: id(id_str),
            name: name.to_owned(),
            generic_name: String::new(),
            comment: String::new(),
            exec: String::new(),
            icon: AppIcon::None,
            startup_wm_class: None,
            flatpak: None,
            terminal: Terminal::No,
            activation: DbusActivation::Exec,
            actions: Vec::new(),
            mime_types: Vec::new(),
            categories: Vec::new(),
            keyboard_profile: None,
            keywords: Vec::new(),
            listing: Listing::Shown,
            path: PathBuf::from(format!("/usr/share/applications/{id_str}.desktop")),
        }
    }

    fn unit() -> ScopeUnit {
        ScopeUnit("app-sill-foot-1.scope".to_owned())
    }

    fn request(action: DesktopAction) -> LaunchRequest {
        LaunchRequest {
            app: id("foot"),
            action,
            files: Vec::new(),
            uris: Vec::new(),
            token: None,
        }
    }

    #[test]
    fn escaping_matches_systemd_unit_component_rules() {
        let table: &[(&str, &str, &str)] = &[
            ("plain", "foot", "foot"),
            ("reverse dns", "org.mozilla.firefox", "org.mozilla.firefox"),
            ("slash becomes dash", "kde/foo", "kde-foo"),
            ("leading dot is escaped", ".hidden", "\\x2ehidden"),
            ("space is escaped", "a b", "a\\x20b"),
            ("a literal dash is escaped", "a-b", "a\\x2db"),
            (
                "colon, underscore and mid dot are kept",
                "a:b_c.d",
                "a:b_c.d",
            ),
        ];
        for (name, given, want) in table {
            assert_eq!(escape_component(given), *want, "{name}");
        }
    }

    #[test]
    fn scope_units_are_unique_per_nonce_and_valid() {
        let a = scope_unit(&id("foot"), 1);
        let b = scope_unit(&id("foot"), 2);
        assert_ne!(a, b);
        for unit in [&a, &b] {
            assert!(unit.0.starts_with("app-sill-foot-"));
            assert!(unit.0.ends_with(".scope"));
            assert!(
                unit.0.chars().all(|c| c.is_ascii_alphanumeric()
                    || matches!(c, '-' | '_' | '.' | '\\' | 'x' | ':')),
                "{}",
                unit.0
            );
        }
    }

    #[test]
    fn argv_wraps_the_exec_with_systemd_run_and_the_terminal() {
        let mut a = bare_app("foot", "Foot");
        a.exec = "foot -e htop".to_owned();
        a.terminal = Terminal::No;

        let plain = launch_argv(&a, &request(DesktopAction::Default), &unit(), &[]);
        assert_eq!(
            plain,
            Ok(vec![
                "systemd-run".to_owned(),
                "--user".to_owned(),
                "--scope".to_owned(),
                "--collect".to_owned(),
                "--unit=app-sill-foot-1.scope".to_owned(),
                "--".to_owned(),
                "foot".to_owned(),
                "-e".to_owned(),
                "htop".to_owned(),
            ])
        );

        let mut terminal_app = a.clone();
        terminal_app.terminal = Terminal::Yes;
        terminal_app.exec = "htop".to_owned();
        let wrapped = launch_argv(
            &terminal_app,
            &request(DesktopAction::Default),
            &unit(),
            &["ghostty".to_owned(), "-e".to_owned()],
        );
        assert_eq!(
            wrapped,
            Ok(vec![
                "systemd-run".to_owned(),
                "--user".to_owned(),
                "--scope".to_owned(),
                "--collect".to_owned(),
                "--unit=app-sill-foot-1.scope".to_owned(),
                "--".to_owned(),
                "ghostty".to_owned(),
                "-e".to_owned(),
                "htop".to_owned(),
            ])
        );
    }

    #[test]
    fn a_named_action_replaces_exec_and_an_unknown_one_falls_back() {
        let mut a = bare_app("code", "Visual Studio Code");
        a.exec = "code".to_owned();
        a.actions = vec![DesktopActionEntry {
            id: "new-window".to_owned(),
            name: "New Window".to_owned(),
            exec: "code --new-window".to_owned(),
        }];

        let named = launch_argv(
            &a,
            &request(DesktopAction::Named("new-window".to_owned())),
            &unit(),
            &[],
        );
        assert_eq!(
            named.map(|argv| argv.last().cloned()),
            Ok(Some("--new-window".to_owned()))
        );

        let unknown = launch_argv(
            &a,
            &request(DesktopAction::Named("no-such-action".to_owned())),
            &unit(),
            &[],
        );
        assert_eq!(
            unknown.map(|argv| argv.last().cloned()),
            Ok(Some("code".to_owned()))
        );
    }

    #[test]
    fn an_empty_exec_is_refused() {
        let mut a = bare_app("nothing", "Nothing");
        a.exec = String::new();
        assert_eq!(
            launch_argv(&a, &request(DesktopAction::Default), &unit(), &[]),
            Err(ExecError::Empty)
        );
    }

    #[test]
    fn the_icon_arg_is_the_resolved_file_or_empty() {
        let mut a = bare_app("firefox", "Firefox");
        a.exec = "firefox %i".to_owned();
        a.icon = AppIcon::File(PathBuf::from("/usr/share/icons/firefox.png"));
        let argv = launch_argv(&a, &request(DesktopAction::Default), &unit(), &[])
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(argv[argv.len() - 2], "--icon");
        assert_eq!(argv[argv.len() - 1], "/usr/share/icons/firefox.png");
    }
}
