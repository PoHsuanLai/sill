//! The panel's keyboard (design/06 §20.2): Up/Down move the selection (clamped), Enter runs
//! it, Tab cycles the provider filter, Ctrl+K / Alt+K open the selected row's actions, Escape
//! closes one level. Pure.
//!
//! The frozen [`step`] knows counts, not ids, so an Enter inside the actions menu cannot name
//! the action it picks: there `step` closes the menu and emits nothing, and the panel takes the
//! pick from the quire `Menu` it draws, which owns that menu's keys (FINDINGS F142).

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
    let selected = state.selected.min(rows.saturating_sub(1));
    let state = LauncherState { selected, ..state };
    match state.actions {
        ActionsMenu::Open { selected: action } => in_actions(state, key, action, actions),
        ActionsMenu::Closed => in_results(state, key, rows, actions),
    }
}

/// A key while the results have the keyboard.
fn in_results(
    state: LauncherState,
    key: LauncherKey,
    rows: usize,
    actions: usize,
) -> (LauncherState, Vec<LauncherOut>) {
    match key {
        LauncherKey::Up => (moved(state, Step::Up, rows), Vec::new()),
        LauncherKey::Down => (moved(state, Step::Down, rows), Vec::new()),
        LauncherKey::Enter if rows > 0 => {
            let row = state.selected;
            (
                state,
                vec![LauncherOut::Activate {
                    row,
                    choice: Choice::Primary,
                }],
            )
        }
        LauncherKey::Enter => (state, Vec::new()),
        LauncherKey::Tab => (filtered(next_filter(state.filter)), Vec::new()),
        LauncherKey::ShiftTab => (filtered(previous_filter(state.filter)), Vec::new()),
        LauncherKey::Actions if rows > 0 && actions > 0 => (
            LauncherState {
                actions: ActionsMenu::Open { selected: 0 },
                ..state
            },
            Vec::new(),
        ),
        LauncherKey::Actions => (state, Vec::new()),
        LauncherKey::Escape => (state, vec![LauncherOut::Close]),
    }
}

/// A key while the actions menu is open: it moves in the menu, Enter picks there (the menu's
/// own pick names the action), Escape or Ctrl+K again close only the menu, Tab is ignored.
fn in_actions(
    state: LauncherState,
    key: LauncherKey,
    action: usize,
    actions: usize,
) -> (LauncherState, Vec<LauncherOut>) {
    let menu = |selected| LauncherState {
        actions: ActionsMenu::Open { selected },
        ..state
    };
    let closed = LauncherState {
        actions: ActionsMenu::Closed,
        ..state
    };
    match key {
        LauncherKey::Up => (menu(clamp(action, Step::Up, actions)), Vec::new()),
        LauncherKey::Down => (menu(clamp(action, Step::Down, actions)), Vec::new()),
        LauncherKey::Enter | LauncherKey::Escape | LauncherKey::Actions => (closed, Vec::new()),
        LauncherKey::Tab | LauncherKey::ShiftTab => (menu(action), Vec::new()),
    }
}

/// Which way a selection moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Up,
    Down,
}

/// `index` moved one `step` within `count`, clamped (design/06 §2.3: no wrap).
fn clamp(index: usize, step: Step, count: usize) -> usize {
    match step {
        Step::Up => index.saturating_sub(1),
        Step::Down => (index + 1).min(count.saturating_sub(1)),
    }
}

fn moved(state: LauncherState, step: Step, rows: usize) -> LauncherState {
    LauncherState {
        selected: clamp(state.selected, step, rows),
        ..state
    }
}

/// A new filter starts at its first row with the actions menu closed.
fn filtered(filter: Filter) -> LauncherState {
    LauncherState {
        selected: 0,
        filter,
        actions: ActionsMenu::Closed,
    }
}

/// Tab: every provider, then each one in [`ProviderKind::ALL`]'s order, then every one again.
pub fn next_filter(filter: Filter) -> Filter {
    let all = ProviderKind::ALL;
    match filter {
        Filter::All => Filter::Only(all[0]),
        Filter::Only(kind) => match all.iter().position(|k| *k == kind) {
            Some(i) if i + 1 < all.len() => Filter::Only(all[i + 1]),
            _ => Filter::All,
        },
    }
}

/// Shift+Tab: [`next_filter`] backwards.
pub fn previous_filter(filter: Filter) -> Filter {
    let all = ProviderKind::ALL;
    match filter {
        Filter::All => Filter::Only(all[all.len() - 1]),
        Filter::Only(kind) => match all.iter().position(|k| *k == kind) {
            Some(i) if i > 0 => Filter::Only(all[i - 1]),
            _ => Filter::All,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ActionsMenu, Filter, LauncherKey, LauncherOut, LauncherState, next_filter, previous_filter,
        step,
    };
    use sill_launcher::{Choice, ProviderKind};

    fn keys(
        keys: &[LauncherKey],
        rows: usize,
        actions: usize,
    ) -> (LauncherState, Vec<LauncherOut>) {
        keys.iter().fold(
            (LauncherState::default(), Vec::new()),
            |(state, mut out), key| {
                let (next, more) = step(state, *key, rows, actions);
                out.extend(more);
                (next, out)
            },
        )
    }

    #[test]
    fn up_and_down_clamp_and_enter_runs_the_selection() {
        use LauncherKey::{Down, Enter, Up};
        // (keys, rows, the selection after, what came out)
        type Case = (&'static [LauncherKey], usize, usize, Vec<LauncherOut>);
        let run = |row| {
            vec![LauncherOut::Activate {
                row,
                choice: Choice::Primary,
            }]
        };
        let cases: Vec<Case> = vec![
            (&[Down, Down, Enter], 5, 2, run(2)),
            (&[Down, Down, Down, Down], 3, 2, vec![]),
            (&[Down, Down, Down, Down, Enter], 3, 2, run(2)),
            (&[Up, Up, Enter], 3, 0, run(0)),
            (&[Down, Up, Up, Down, Enter], 3, 1, run(1)),
            (&[Down, Enter], 0, 0, vec![]),
        ];
        for (pressed, rows, want_selected, want_out) in cases {
            let (state, out) = keys(pressed, rows, 0);
            assert_eq!(out, want_out, "{pressed:?} over {rows}");
            assert_eq!(state.selected, want_selected, "{pressed:?}");
        }
    }

    #[test]
    fn escape_closes_one_level() {
        use LauncherKey::{Actions, Down, Escape};
        let (state, out) = keys(&[Down, Actions, Down, Escape], 4, 3);
        assert_eq!(state.actions, ActionsMenu::Closed, "the menu closed");
        assert_eq!(state.selected, 1, "the row stays selected");
        assert!(out.is_empty(), "the launcher stays open");
        let (_, out) = keys(&[Down, Actions, Escape, Escape], 4, 3);
        assert_eq!(out, vec![LauncherOut::Close]);
    }

    #[test]
    fn ctrl_k_opens_the_selected_rows_actions_and_toggles() {
        use LauncherKey::{Actions, Down, Up};
        let (state, _) = keys(&[Down, Actions], 4, 3);
        assert_eq!(state.actions, ActionsMenu::Open { selected: 0 });
        let (state, _) = keys(&[Down, Actions, Down, Down, Down, Up], 4, 3);
        assert_eq!(
            state.actions,
            ActionsMenu::Open { selected: 1 },
            "clamped at 2, then up"
        );
        assert_eq!(state.selected, 1, "the row does not move under the menu");
        let (state, _) = keys(&[Actions, Actions], 4, 3);
        assert_eq!(state.actions, ActionsMenu::Closed, "Ctrl+K again closes");
        let (state, _) = keys(&[Actions], 4, 0);
        assert_eq!(state.actions, ActionsMenu::Closed, "a row without actions");
        let (state, _) = keys(&[Actions], 0, 3);
        assert_eq!(state.actions, ActionsMenu::Closed, "no rows");
    }

    #[test]
    fn enter_in_the_actions_menu_is_the_menus_pick() {
        use LauncherKey::{Actions, Enter};
        let (state, out) = keys(&[Actions, Enter], 4, 3);
        assert_eq!(state.actions, ActionsMenu::Closed);
        assert!(out.is_empty(), "the ds Menu's onpick names the action");
    }

    #[test]
    fn tab_cycles_the_provider_filter_and_resets_the_selection() {
        use LauncherKey::{Down, ShiftTab, Tab};
        let (state, _) = keys(&[Down, Down, Tab], 5, 0);
        assert_eq!(state.filter, Filter::Only(ProviderKind::Apps));
        assert_eq!(state.selected, 0);
        let (state, _) = keys(&[Tab, Tab], 5, 0);
        assert_eq!(state.filter, Filter::Only(ProviderKind::Windows));
        let (state, _) = keys(&[ShiftTab], 5, 0);
        assert_eq!(state.filter, Filter::Only(ProviderKind::System));
        let mut filter = Filter::All;
        let mut seen = Vec::new();
        for _ in 0..=ProviderKind::ALL.len() {
            filter = next_filter(filter);
            seen.push(filter);
        }
        assert_eq!(seen.last(), Some(&Filter::All), "Tab comes back round");
        for f in seen {
            assert_eq!(previous_filter(next_filter(f)), f, "{f:?}");
        }
    }

    #[test]
    fn a_selection_past_a_shrunk_list_is_clamped_first() {
        let state = LauncherState {
            selected: 7,
            ..LauncherState::default()
        };
        let (state, out) = step(state, LauncherKey::Enter, 3, 0);
        assert_eq!(state.selected, 2);
        assert_eq!(
            out,
            vec![LauncherOut::Activate {
                row: 2,
                choice: Choice::Primary
            }]
        );
    }
}
