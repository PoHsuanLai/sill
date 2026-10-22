//! What a binding does, resolved from the settings: the effect a firing carries out.

use std::path::{Path, PathBuf};

use sill_launcher::actions::ActionId;
use sill_launcher::{Activation, AppId, DesktopAction, OpenTarget};
use sill_settings::{CornerAction, CornerDo, OpenItem};

/// What a corner's binding does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CornerEffect {
    Nothing,
    /// An action of the shell, on the corner's output.
    Do(ActionId),
    /// App Exposé of the focused app (the desktop's compositor opens it).
    AppExpose,
    /// Summon the companion, as its double-tap does.
    Assistant,
    /// What the launcher's activation path carries out: a launch, an open, an app's action.
    Activate(Activation),
    /// A child process: `/bin/sh -c COMMAND` for `Command`.
    Run {
        program: String,
        args: Vec<String>,
    },
    /// A binding that cannot be carried out, and why (logged).
    Cannot(String),
}

/// What `action` does; `home` is the person's home folder, which a path's `~` stands for.
pub fn corner_effect(action: &CornerDo, home: Option<&Path>) -> CornerEffect {
    match action {
        CornerDo::Desktop(action) => desktop_effect(*action),
        CornerDo::AppAction { app, intent } => Activation::app_action(app, intent).map_or_else(
            || CornerEffect::Cannot(format!("{app} › {intent} is not an action name")),
            CornerEffect::Activate,
        ),
        CornerDo::Open(item) => open_effect(item, home),
        CornerDo::Command(command) if command.trim().is_empty() => CornerEffect::Nothing,
        CornerDo::Command(command) => CornerEffect::Run {
            program: "/bin/sh".to_owned(),
            args: vec!["-c".to_owned(), command.clone()],
        },
        // `CornerDo` is a foreign `#[non_exhaustive]` enum, so the compiler asks for this arm
        // and no other: a variant added upstream lands here until it is handled above, and it
        // must be seen rather than silently do nothing.
        other => CornerEffect::Cannot(format!("a binding this build does not know ({other:?})")),
    }
}

fn desktop_effect(action: CornerAction) -> CornerEffect {
    match action {
        CornerAction::Launcher => CornerEffect::Do(ActionId::Launcher),
        CornerAction::NotificationCenter => CornerEffect::Do(ActionId::NotificationCenter),
        CornerAction::ControlCenter => CornerEffect::Do(ActionId::ControlCenter),
        CornerAction::ShowDesktop => CornerEffect::Do(ActionId::ShowDesktop),
        CornerAction::Lock => CornerEffect::Do(ActionId::Lock),
        CornerAction::Workspaces => CornerEffect::Do(ActionId::MissionControl),
        CornerAction::AppWindows => CornerEffect::AppExpose,
        CornerAction::Assistant => CornerEffect::Assistant,
        // The idle service takes no "displays off now" (`IdleCommand` has Configure, Use and
        // FakeWake only); until it does, the corner says so rather than doing nothing silently.
        CornerAction::DisplaySleep => {
            CornerEffect::Cannot("putting the displays to sleep is not available yet".to_owned())
        }
        // `CornerAction` is a foreign `#[non_exhaustive]` enum: a word added upstream lands here
        // until it is handled above.
        other => CornerEffect::Cannot(format!(
            "a corner action this build does not know ({other:?})"
        )),
    }
}

fn open_effect(item: &OpenItem, home: Option<&Path>) -> CornerEffect {
    match item {
        OpenItem::App(id) => CornerEffect::Activate(Activation::Launch {
            app: AppId(id.clone()),
            action: DesktopAction::Default,
        }),
        OpenItem::Path(path) => match expand_home(path, home) {
            Some(file) => CornerEffect::Activate(Activation::Open {
                target: OpenTarget::File(file),
            }),
            None => CornerEffect::Cannot(format!("{path} needs a home folder, and there is none")),
        },
        OpenItem::Url(address) => CornerEffect::Activate(Activation::Open {
            target: OpenTarget::Url(address.clone()),
        }),
    }
}

/// `path` with a leading `~` replaced by `home`; none when it has one and `home` is unknown.
fn expand_home(path: &str, home: Option<&Path>) -> Option<PathBuf> {
    match path.strip_prefix('~') {
        None => Some(PathBuf::from(path)),
        Some("") => home.map(Path::to_path_buf),
        Some(rest) => rest
            .strip_prefix('/')
            .and_then(|below| home.map(|home| home.join(below))),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use sill_launcher::actions::ActionId;
    use sill_launcher::{Activation, AppId, DesktopAction, OpenTarget};
    use sill_settings::{CornerAction, CornerDo, OpenItem};

    use super::{CornerEffect, corner_effect};

    const HOME: &str = "/home/p";

    fn open(target: OpenTarget) -> CornerEffect {
        CornerEffect::Activate(Activation::Open { target })
    }

    #[test]
    fn each_binding_has_its_effect() {
        let run = |program: &str, args: &[&str]| CornerEffect::Run {
            program: program.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
        };
        let desktop = CornerDo::Desktop;
        let item = CornerDo::Open;
        let cases = [
            (
                desktop(CornerAction::Launcher),
                CornerEffect::Do(ActionId::Launcher),
            ),
            (
                desktop(CornerAction::NotificationCenter),
                CornerEffect::Do(ActionId::NotificationCenter),
            ),
            (
                desktop(CornerAction::ControlCenter),
                CornerEffect::Do(ActionId::ControlCenter),
            ),
            (
                desktop(CornerAction::ShowDesktop),
                CornerEffect::Do(ActionId::ShowDesktop),
            ),
            (
                desktop(CornerAction::Lock),
                CornerEffect::Do(ActionId::Lock),
            ),
            (
                desktop(CornerAction::Workspaces),
                CornerEffect::Do(ActionId::MissionControl),
            ),
            (desktop(CornerAction::AppWindows), CornerEffect::AppExpose),
            (desktop(CornerAction::Assistant), CornerEffect::Assistant),
            (
                desktop(CornerAction::DisplaySleep),
                CornerEffect::Cannot("putting the displays to sleep is not available yet".into()),
            ),
            (
                item(OpenItem::App("org.quire.Files".into())),
                CornerEffect::Activate(Activation::Launch {
                    app: AppId("org.quire.Files".into()),
                    action: DesktopAction::Default,
                }),
            ),
            (
                item(OpenItem::Path("~/Documents".into())),
                open(OpenTarget::File(PathBuf::from("/home/p/Documents"))),
            ),
            (
                item(OpenItem::Path("~".into())),
                open(OpenTarget::File(PathBuf::from(HOME))),
            ),
            (
                item(OpenItem::Path("/srv/share".into())),
                open(OpenTarget::File(PathBuf::from("/srv/share"))),
            ),
            (
                item(OpenItem::Url("https://example.org".into())),
                open(OpenTarget::Url("https://example.org".into())),
            ),
            (
                CornerDo::Command("foot -e htop".into()),
                run("/bin/sh", &["-c", "foot -e htop"]),
            ),
            (CornerDo::Command("  ".into()), CornerEffect::Nothing),
        ];
        for (action, want) in cases {
            assert_eq!(
                corner_effect(&action, Some(Path::new(HOME))),
                want,
                "{action:?}"
            );
        }
    }

    #[test]
    fn an_app_action_is_an_intent_and_a_bad_name_cannot_be_done() {
        let good = corner_effect(
            &CornerDo::AppAction {
                app: "org.quire.Mail".into(),
                intent: "mail.compose".into(),
            },
            None,
        );
        assert!(matches!(
            good,
            CornerEffect::Activate(Activation::Intent(_))
        ));
        let bad = corner_effect(
            &CornerDo::AppAction {
                app: "not an app".into(),
                intent: "x".into(),
            },
            None,
        );
        assert!(matches!(bad, CornerEffect::Cannot(_)), "{bad:?}");
    }

    #[test]
    fn a_tilde_without_a_home_cannot_be_opened() {
        let effect = corner_effect(&CornerDo::Open(OpenItem::Path("~/x".into())), None);
        assert!(matches!(effect, CornerEffect::Cannot(_)), "{effect:?}");
    }
}
