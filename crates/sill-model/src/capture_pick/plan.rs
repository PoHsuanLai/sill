//! What the sheet offers: the tabs the kinds allow and the choices on each.

use super::ask::{Ask, Kinds, Pointer};

/// One screen the shell could share.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputCandidate {
    /// The connector name (`DP-1`): what the reply quotes.
    pub name: String,
    /// What the person calls it.
    pub title: String,
}

/// One window the shell could share.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowCandidate {
    /// The compositor's foreign-toplevel identifier: what the reply quotes.
    pub identifier: String,
    pub title: String,
    /// The app's name, for the row's detail.
    pub app: String,
}

/// Everything that exists to share now.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Candidates {
    pub outputs: Vec<OutputCandidate>,
    pub windows: Vec<WindowCandidate>,
}

/// Which list the sheet shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Screens,
    Windows,
}

/// A thing to share, as the reply names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Output(String),
    Window(String),
}

impl Source {
    /// The reply's `(kind, name)` pair.
    pub fn wire(&self) -> (&'static str, &str) {
        match self {
            Source::Output(name) => ("output", name),
            Source::Window(id) => ("window", id),
        }
    }
}

/// One row of a tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub source: Source,
    pub title: String,
    pub detail: String,
}

/// The sheet's content: the tabs in order (one tab means no tab control), each tab's choices
/// and the pointer modes to pick among (one means no pointer control).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub tabs: Vec<Tab>,
    pub screens: Vec<Choice>,
    pub windows: Vec<Choice>,
    pub pointers: Vec<Pointer>,
}

impl Plan {
    /// The choices on `tab`.
    pub fn on(&self, tab: Tab) -> &[Choice] {
        match tab {
            Tab::Screens => &self.screens,
            Tab::Windows => &self.windows,
        }
    }

    /// Whether `source` is offered on any tab.
    pub fn offers(&self, source: &Source) -> bool {
        self.screens
            .iter()
            .chain(&self.windows)
            .any(|choice| &choice.source == source)
    }
}

/// The plan for `ask` over `candidates`. A kind the app did not accept offers nothing.
pub fn plan(ask: &Ask, candidates: &Candidates) -> Plan {
    let tabs = match ask.kinds {
        Kinds::Screens => vec![Tab::Screens],
        Kinds::Windows => vec![Tab::Windows],
        Kinds::Both => vec![Tab::Screens, Tab::Windows],
    };
    let screens = if tabs.contains(&Tab::Screens) {
        candidates
            .outputs
            .iter()
            .map(|output| Choice {
                source: Source::Output(output.name.clone()),
                title: output.title.clone(),
                detail: output.name.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    let windows = if tabs.contains(&Tab::Windows) {
        candidates
            .windows
            .iter()
            .map(|window| Choice {
                source: Source::Window(window.identifier.clone()),
                title: window.title.clone(),
                detail: window.app.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    Plan {
        tabs,
        screens,
        windows,
        pointers: ask.pointers.clone(),
    }
}
