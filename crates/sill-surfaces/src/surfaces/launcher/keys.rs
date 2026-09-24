//! The panel's keyboard (design/06 §20.2): Up/Down move the selection (clamped), Enter runs
//! it, Tab cycles the provider filter, Ctrl+K / Alt+K open the selected row's actions, Escape
//! closes one level. Pure.

#![allow(unused_variables, dead_code)] // Freeze stubs

use sill_launcher::{Choice, ProviderKind};

/// What the panel is showing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LauncherState {
    /// The selected row, an index into the visible results.
    pub selected: usize,
    /// Tab's filter: every provider, or one.
    pub filter: Filter,
    pub actions: ActionsMenu,
}

/// Which providers' rows show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Only(ProviderKind),
}

/// Whether the selected row's actions menu is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionsMenu {
    #[default]
    Closed,
    Open {
        selected: usize,
    },
}

/// A key the panel handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherKey {
    Up,
    Down,
    Enter,
    Tab,
    ShiftTab,
    /// Ctrl+K or Alt+K.
    Actions,
    Escape,
}

/// What the panel must do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherOut {
    /// Activate the selected row with this choice, then close.
    Activate {
        row: usize,
        choice: Choice,
    },
    Close,
}

/// `state` after `key`, with `rows` results visible and `actions` actions on the selected row.
pub fn step(
    state: LauncherState,
    key: LauncherKey,
    rows: usize,
    actions: usize,
) -> (LauncherState, Vec<LauncherOut>) {
    todo!("M4 launcher-ui")
}
