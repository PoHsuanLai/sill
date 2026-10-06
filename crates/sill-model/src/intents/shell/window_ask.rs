//! The shell's window and workspace actions: `shell.window.*`, `shell.app.*`,
//! `shell.workspace.list` and `shell.workspace.switch` given a workspace. They are not
//! [`ShellAction`](super::ShellAction)s, which the daemon does by an IPC request: the compositor's
//! state answers them (`window_plan::plan`), and what they do to a window is the app-name menu's
//! own rows (`app_menu::OwnItem`), so the bar and an agent share one implementation.

use docket_core::{AppRefusal, FailText, Invocation, Value};
use prov::Effect;

use super::invocation::{param, text};

/// A workspace an ask names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceRef {
    /// The position in `shell.workspace.list`, counting from 1.
    Index(u32),
    /// The id `shell.workspace.list` gave it.
    Id(String),
}

/// One ask about windows or workspaces, its parameters read. Window and workspace ids are the
/// shell's own handles from the listings; whether one is known is the plan's to say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowAsk {
    /// `shell.window.list`.
    List,
    /// `shell.window.focus`: raise and activate the window.
    Focus { window: String },
    /// `shell.window.minimise`.
    Minimise { window: String },
    /// `shell.window.close`: ask the window to close; the app may lose unsaved work.
    Close { window: String },
    /// `shell.window.move_to_workspace`.
    MoveToWorkspace { window: String, to: WorkspaceRef },
    /// `shell.app.hide`: minimise every window of `app` (the focused app when `None`).
    Hide { app: Option<String> },
    /// `shell.app.hide_others`: minimise every window of every app but `app` (the focused one).
    HideOthers { app: Option<String> },
    /// `shell.app.show_all`: bring every minimised window back.
    ShowAll,
    /// `shell.workspace.list`.
    WorkspaceList,
    /// `shell.workspace.switch` given a workspace.
    WorkspaceGo(WorkspaceRef),
}

impl WindowAsk {
    /// What the ask costs: closing can lose an app's unsaved work, everything else only changes
    /// what is on screen and can be put back.
    pub fn effect(&self) -> Effect {
        match self {
            WindowAsk::Close { .. } => Effect::Destructive,
            WindowAsk::List
            | WindowAsk::Focus { .. }
            | WindowAsk::Minimise { .. }
            | WindowAsk::MoveToWorkspace { .. }
            | WindowAsk::Hide { .. }
            | WindowAsk::HideOthers { .. }
            | WindowAsk::ShowAll
            | WindowAsk::WorkspaceList
            | WindowAsk::WorkspaceGo(_) => Effect::Read,
        }
    }

    /// What the person is told once it is done, in the shell's own words.
    pub fn said(&self) -> &'static str {
        match self {
            WindowAsk::List => "Listed the windows",
            WindowAsk::Focus { .. } => "Brought the window forward",
            WindowAsk::Minimise { .. } => "Minimised the window",
            WindowAsk::Close { .. } => "Asked the window to close",
            WindowAsk::MoveToWorkspace { .. } => "Moved the window to the workspace",
            WindowAsk::Hide { .. } => "Hid the app",
            WindowAsk::HideOthers { .. } => "Hid the other apps",
            WindowAsk::ShowAll => "Showed every window",
            WindowAsk::WorkspaceList => "Listed the workspaces",
            WindowAsk::WorkspaceGo(_) => "Switched workspace",
        }
    }
}

/// The window or workspace ask `invocation` is, `None` when it is another action of the shell
/// (`shell.workspace.switch` with only a direction is one).
pub fn window_ask_of(invocation: &Invocation) -> Result<Option<WindowAsk>, AppRefusal> {
    let ask = match invocation.action.as_str() {
        "shell.window.list" => WindowAsk::List,
        "shell.window.focus" => WindowAsk::Focus {
            window: window(invocation)?,
        },
        "shell.window.minimise" => WindowAsk::Minimise {
            window: window(invocation)?,
        },
        "shell.window.close" => WindowAsk::Close {
            window: window(invocation)?,
        },
        "shell.window.move_to_workspace" => WindowAsk::MoveToWorkspace {
            window: window(invocation)?,
            to: workspace(invocation)?.ok_or(AppRefusal::NeedsParam {
                param: param("workspace")?,
                options: Vec::new(),
            })?,
        },
        "shell.app.hide" => WindowAsk::Hide {
            app: text_given(invocation, "app")?,
        },
        "shell.app.hide_others" => WindowAsk::HideOthers {
            app: text_given(invocation, "app")?,
        },
        "shell.app.show_all" => WindowAsk::ShowAll,
        "shell.workspace.list" => WindowAsk::WorkspaceList,
        "shell.workspace.switch" => match workspace(invocation)? {
            Some(to) => WindowAsk::WorkspaceGo(to),
            None => return Ok(None),
        },
        _ => return Ok(None),
    };
    Ok(Some(ask))
}

/// The window id, required.
fn window(invocation: &Invocation) -> Result<String, AppRefusal> {
    text_given(invocation, "window")?.ok_or(AppRefusal::NeedsParam {
        param: param("window")?,
        options: Vec::new(),
    })
}

/// The text argument `name` when it is given and not blank.
fn text_given(invocation: &Invocation, name: &str) -> Result<Option<String>, AppRefusal> {
    Ok(text(invocation, name)?
        .map(|given| given.trim().to_owned())
        .filter(|given| !given.is_empty()))
}

/// The workspace asked for: `workspace` (an id) wins over `index` when both are given, and
/// neither is `None`. An index below 1 is refused by name.
fn workspace(invocation: &Invocation) -> Result<Option<WorkspaceRef>, AppRefusal> {
    if let Some(id) = text_given(invocation, "workspace")? {
        return Ok(Some(WorkspaceRef::Id(id)));
    }
    let index = param("index")?;
    match invocation.args.get(&index).map(|arg| &arg.value) {
        None => Ok(None),
        Some(Value::Integer(at)) => u32::try_from(*at)
            .ok()
            .filter(|at| *at >= 1)
            .map(|at| Some(WorkspaceRef::Index(at)))
            .ok_or_else(|| failed("index counts from 1")),
        Some(_) => Err(failed("index is not a whole number")),
    }
}

fn failed(why: &str) -> AppRefusal {
    AppRefusal::Failed(FailText(why.to_owned()))
}
