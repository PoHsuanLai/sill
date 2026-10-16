//! A `com.canonical.dbusmenu` layout, read once at the wire into typed nodes, as the tray's rows.
//! The properties are dbusmenu's own with its defaults: a node is enabled and visible unless it
//! says otherwise, and a toggle with no state is indeterminate.

use crate::tray::{
    Check, Chord, MENU_DEPTH_MAX, MENU_LABEL_MAX_CHARS, MENU_ROWS_MAX, MenuCommand, TrayMenuEntry,
    TrayMenuIcon, TrayMenuKind, plain_label,
};
use crate::units::{Availability, Visibility};
use crate::untrusted::for_display;

/// `type`: a row or a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeType {
    #[default]
    Standard,
    Separator,
}

/// `toggle-type`: the mark a row can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToggleKind {
    #[default]
    None,
    Checkmark,
    Radio,
}

/// The properties of one node that the shell reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeProps {
    pub kind: NodeType,
    pub label: Option<String>,
    pub enabled: Availability,
    pub visible: Visibility,
    pub toggle: ToggleKind,
    /// `toggle-state` as sent: 0 off, 1 on, anything else indeterminate.
    pub toggle_state: i32,
    pub shortcut: Option<Chord>,
    /// `x-quire-command-id`.
    pub command: Option<MenuCommand>,
    pub icon_name: Option<String>,
}

impl Default for NodeProps {
    fn default() -> NodeProps {
        NodeProps {
            kind: NodeType::Standard,
            label: None,
            enabled: Availability::Enabled,
            visible: Visibility::Shown,
            toggle: ToggleKind::None,
            toggle_state: -1,
            shortcut: None,
            command: None,
            icon_name: None,
        }
    }
}

/// One node of a layout: its id, its properties and its children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: i32,
    pub props: NodeProps,
    pub children: Vec<Node>,
}

/// How many rows may still be taken from the layout being read.
#[derive(Debug, Clone, Copy)]
struct RowsLeft(usize);

/// `nodes` as the tray's rows. A rule has no words; a row's label loses its access-key marks and
/// is cut. A layout is another program's tree: it is read for at most [`MENU_ROWS_MAX`] rows and
/// [`MENU_DEPTH_MAX`] levels, the first of each level.
pub fn rows_of(nodes: Vec<Node>) -> Vec<TrayMenuEntry> {
    rows(nodes, 1, &mut RowsLeft(MENU_ROWS_MAX))
}

fn rows(nodes: Vec<Node>, depth: usize, left: &mut RowsLeft) -> Vec<TrayMenuEntry> {
    if depth > MENU_DEPTH_MAX {
        return Vec::new();
    }
    let mut taken = Vec::new();
    for node in nodes {
        let Some(rest) = left.0.checked_sub(1) else {
            break;
        };
        left.0 = rest;
        taken.push(row(node, depth, left));
    }
    taken
}

fn row(node: Node, depth: usize, left: &mut RowsLeft) -> TrayMenuEntry {
    let Node {
        id,
        props,
        children,
    } = node;
    let kind = match props.kind {
        NodeType::Separator => TrayMenuKind::Separator,
        NodeType::Standard => TrayMenuKind::Item {
            label: for_display(
                &plain_label(props.label.as_deref().unwrap_or_default()),
                MENU_LABEL_MAX_CHARS,
            ),
            enabled: props.enabled,
            visibility: props.visible,
            check: check_of(props.toggle, props.toggle_state),
            icon: props
                .icon_name
                .filter(|name| !name.is_empty())
                .map_or(TrayMenuIcon::None, TrayMenuIcon::Named),
            children: rows(children, depth + 1, left),
            shortcut: props.shortcut,
            command: props.command,
        },
    };
    TrayMenuEntry { id, kind }
}

fn check_of(toggle: ToggleKind, state: i32) -> Check {
    match (toggle, state) {
        (ToggleKind::None, _) => Check::None,
        (ToggleKind::Checkmark | ToggleKind::Radio, 0) => Check::Unchecked,
        (ToggleKind::Checkmark | ToggleKind::Radio, 1) => Check::Checked,
        (ToggleKind::Checkmark | ToggleKind::Radio, _) => Check::Mixed,
    }
}
