//! An `Invocation` of one of the shell's actions, read into a [`ShellAction`], and the answer an
//! action gives. The names and the choice ids are the manifest's (`dist/intents/org.quire.Shell.toml`);
//! a test holds the two together.

use docket_core::{
    AppRefusal, ChoiceId, FailText, Follow, Invocation, LabelText, Outcome, ParamName, Preview,
    UndoFault, UndoToken, Undoable, Value,
};
use ds_core::word::Word;
use sill_launcher::AppId;

use super::action::{ShellAction, WorkspaceStep};
use crate::screenshots::ShotKind;

/// The action `invocation` asks for. A name the shell does not declare is `Unsupported`; a
/// parameter that is missing, of the wrong type or outside its options is `NeedsParam` or `Failed`
/// with words that name it, never with the argument's own text.
pub fn action_of(invocation: &Invocation) -> Result<ShellAction, AppRefusal> {
    match invocation.action.as_str() {
        "shell.launcher.open" => Ok(ShellAction::LauncherOpen {
            query: text(invocation, "query")?.unwrap_or_default(),
        }),
        "shell.dock.pin" => app(invocation).map(|app| ShellAction::DockPin { app }),
        "shell.dock.unpin" => app(invocation).map(|app| ShellAction::DockUnpin { app }),
        "shell.workspace.switch" => step(invocation).map(ShellAction::WorkspaceSwitch),
        "shell.lock" => Ok(ShellAction::Lock),
        "shell.screenshot" => kind(invocation).map(ShellAction::Screenshot),
        _ => Err(AppRefusal::Unsupported),
    }
}

/// What the person is told once `action` has been asked of the shell.
pub fn outcome_of(action: &ShellAction) -> Outcome {
    let said = match action {
        ShellAction::LauncherOpen { .. } => "Opened the launcher",
        ShellAction::DockPin { .. } => "Kept the app in the dock",
        ShellAction::DockUnpin { .. } => "Took the app out of the dock",
        ShellAction::WorkspaceSwitch(WorkspaceStep::Next) => "Switched to the next workspace",
        ShellAction::WorkspaceSwitch(WorkspaceStep::Prev) => "Switched to the previous workspace",
        ShellAction::Lock => "Locked the screen",
        ShellAction::Screenshot(_) => "Took a screenshot",
    };
    Outcome {
        value: None,
        said: LabelText::parse(said).ok(),
        show: Preview::None,
        undo: undo_token(action).map_or(Undoable::No, Undoable::Yes),
        follow: Follow::Nothing,
    }
}

/// The token that takes `action` back, for the two that can be: a pin is undone by the unpin of
/// the same app and the other way round. The token is the action that undoes it
/// (`dock.unpin:firefox.desktop`), so a restart of the shell loses nothing.
fn undo_token(action: &ShellAction) -> Option<UndoToken> {
    let text = match action {
        ShellAction::DockPin { app } => format!("{UNPIN}{}", app.0),
        ShellAction::DockUnpin { app } => format!("{PIN}{}", app.0),
        ShellAction::LauncherOpen { .. }
        | ShellAction::WorkspaceSwitch(_)
        | ShellAction::Lock
        | ShellAction::Screenshot(_) => return None,
    };
    UndoToken::parse(&text).ok()
}

/// The action that undoes what `token` was minted for; a token that is none of ours is `Gone`.
pub fn undo_of(token: &UndoToken) -> Result<ShellAction, UndoFault> {
    let text = token.as_str();
    if let Some(app) = text.strip_prefix(UNPIN) {
        Ok(ShellAction::DockUnpin {
            app: AppId(app.to_owned()),
        })
    } else if let Some(app) = text.strip_prefix(PIN) {
        Ok(ShellAction::DockPin {
            app: AppId(app.to_owned()),
        })
    } else {
        Err(UndoFault::Gone)
    }
}

const PIN: &str = "dock.pin:";
const UNPIN: &str = "dock.unpin:";

pub(super) fn param(name: &str) -> Result<ParamName, AppRefusal> {
    ParamName::parse(name).map_err(|_| AppRefusal::Unsupported)
}

/// The text argument `name`, when given.
pub(super) fn text(invocation: &Invocation, name: &str) -> Result<Option<String>, AppRefusal> {
    let name = param(name)?;
    match invocation.args.get(&name).map(|arg| &arg.value) {
        None => Ok(None),
        Some(Value::Text(text)) => Ok(Some(text.clone())),
        Some(_) => Err(failed(format!("{name} is not text"))),
    }
}

/// The choice argument `name` as one of `T`'s words; `default` when it is not given.
fn choice<T: Word>(
    invocation: &Invocation,
    name: &str,
    default: Option<T>,
) -> Result<T, AppRefusal> {
    let param = param(name)?;
    match invocation.args.get(&param).map(|arg| &arg.value) {
        None => default.ok_or(AppRefusal::NeedsParam {
            param,
            options: Vec::new(),
        }),
        Some(Value::Choice(id)) => {
            word_of(id).ok_or_else(|| failed(format!("{param} is not one of its options")))
        }
        Some(_) => Err(failed(format!("{param} is not a choice"))),
    }
}

fn word_of<T: Word>(id: &ChoiceId) -> Option<T> {
    T::parse(id.as_str())
}

/// The desktop id the action is about: required, and not empty.
fn app(invocation: &Invocation) -> Result<AppId, AppRefusal> {
    match text(invocation, "app")? {
        Some(id) if !id.trim().is_empty() => Ok(AppId(id.trim().to_owned())),
        Some(_) | None => Err(AppRefusal::NeedsParam {
            param: param("app")?,
            options: Vec::new(),
        }),
    }
}

fn step(invocation: &Invocation) -> Result<WorkspaceStep, AppRefusal> {
    choice(invocation, "direction", Some(WorkspaceStep::Next))
}

fn kind(invocation: &Invocation) -> Result<ShotKind, AppRefusal> {
    choice(invocation, "kind", Some(ShotKind::Screen))
}

fn failed(why: String) -> AppRefusal {
    AppRefusal::Failed(FailText(why))
}
