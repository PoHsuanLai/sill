//! `com.canonical.dbusmenu` on the wire: the layout read once into typed nodes
//! ([`sill_model::app_menu::Node`]), the one `Event` the shell sends, and `AboutToShow`.
//! Nothing here decides anything; a node that cannot be read is dropped and a reply that is not
//! a layout is an error.

use std::collections::HashMap;

use serde::Deserialize;
use sill_model::app_menu::{Node, NodeProps, NodeType, ToggleKind};
use sill_model::tray::{Chord, MenuCommand};
use sill_model::units::{Availability, Visibility};
use zbus::Connection;
use zbus::names::BusName;
use zbus::zvariant::{OwnedValue, Type, Value};

/// Where an app serves its menu (`AppMenuAddress::PATH`).
pub(crate) const MENU_PATH: &str = "/org/quire/AppMenu";

/// The interface it serves.
pub(crate) const MENU_INTERFACE: &str = "com.canonical.dbusmenu";

/// The deepest menu nesting read, and the most nodes: past either the reply is not a menu the
/// bar could draw.
const MAX_DEPTH: usize = 12;
const MAX_NODES: usize = 4000;

/// Why a menu could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum WireError {
    #[error("the bus: {0}")]
    Bus(String),
    #[error("the reply is not a dbusmenu layout")]
    NotALayout,
    #[error("the menu is larger than the bar reads")]
    TooLarge,
}

impl From<zbus::Error> for WireError {
    fn from(error: zbus::Error) -> Self {
        WireError::Bus(error.to_string())
    }
}

/// `(ia{sv}av)`: a node, its properties and its children, each a variant of the same shape.
#[derive(Debug, Type, Deserialize, Value, OwnedValue)]
#[zvariant(crate = "zbus::zvariant")]
struct WireNode {
    id: i32,
    props: HashMap<String, OwnedValue>,
    children: Vec<OwnedValue>,
}

/// The menu bar's top-level menus, each with everything below it, read from the app serving
/// `destination` (a well-known or a unique name).
pub(crate) async fn read_layout(
    connection: &Connection,
    destination: &str,
) -> Result<Vec<Node>, WireError> {
    let destination =
        BusName::try_from(destination).map_err(|e| WireError::Bus(e.to_string()))?;
    let reply = connection
        .call_method(
            Some(destination),
            MENU_PATH,
            Some(MENU_INTERFACE),
            "GetLayout",
            &(0_i32, -1_i32, Vec::<String>::new()),
        )
        .await?;
    let (_revision, root): (u32, WireNode) = reply
        .body()
        .deserialize()
        .map_err(|_| WireError::NotALayout)?;
    let mut budget = MAX_NODES;
    children_of(root, MAX_DEPTH, &mut budget)
}

fn children_of(
    node: WireNode,
    depth: usize,
    budget: &mut usize,
) -> Result<Vec<Node>, WireError> {
    let Some(depth) = depth.checked_sub(1) else {
        return Err(WireError::TooLarge);
    };
    node.children
        .into_iter()
        .filter_map(|child| WireNode::try_from(child).ok())
        .map(|child| {
            *budget = budget.checked_sub(1).ok_or(WireError::TooLarge)?;
            Ok(Node {
                id: child.id,
                props: props_of(&child.props),
                children: children_of(child, depth, budget)?,
            })
        })
        .collect()
}

fn get<T: TryFrom<OwnedValue>>(props: &HashMap<String, OwnedValue>, key: &str) -> Option<T> {
    props.get(key)?.try_clone().ok()?.try_into().ok()
}

/// The properties the shell reads, each with dbusmenu's default when it is missing or of the
/// wrong type.
pub(crate) fn props_of(props: &HashMap<String, OwnedValue>) -> NodeProps {
    let flag = |key: &str| get::<bool>(props, key);
    let defaults = NodeProps::default();
    NodeProps {
        kind: match get::<String>(props, "type").as_deref() {
            Some("separator") => NodeType::Separator,
            _ => NodeType::Standard,
        },
        label: get(props, "label"),
        enabled: flag("enabled").map_or(defaults.enabled, |on| {
            if on {
                Availability::Enabled
            } else {
                Availability::Disabled
            }
        }),
        visible: flag("visible").map_or(defaults.visible, |on| {
            if on {
                Visibility::Shown
            } else {
                Visibility::Hidden
            }
        }),
        toggle: match get::<String>(props, "toggle-type").as_deref() {
            Some("checkmark") => ToggleKind::Checkmark,
            Some("radio") => ToggleKind::Radio,
            _ => ToggleKind::None,
        },
        toggle_state: get(props, "toggle-state").unwrap_or(defaults.toggle_state),
        shortcut: get::<Vec<Vec<String>>>(props, "shortcut")
            .and_then(|chords| chords.into_iter().next())
            .map(Chord),
        command: get::<String>(props, "x-quire-command-id").map(MenuCommand),
        icon_name: get(props, "icon-name"),
    }
}

/// `Event(id, "clicked")` on `destination`.
pub(crate) async fn click(
    connection: &Connection,
    destination: &str,
    id: i32,
) -> Result<(), WireError> {
    let destination =
        BusName::try_from(destination).map_err(|e| WireError::Bus(e.to_string()))?;
    connection
        .call_method(
            Some(destination),
            MENU_PATH,
            Some(MENU_INTERFACE),
            "Event",
            &(id, "clicked", Value::from(0_i32), 0_u32),
        )
        .await?;
    Ok(())
}

/// `AboutToShow(id)` on `destination`: the menu is about to show, so the app can refresh it.
pub(crate) async fn about_to_show(
    connection: &Connection,
    destination: &str,
    id: i32,
) -> Result<(), WireError> {
    let destination =
        BusName::try_from(destination).map_err(|e| WireError::Bus(e.to_string()))?;
    connection
        .call_method(
            Some(destination),
            MENU_PATH,
            Some(MENU_INTERFACE),
            "AboutToShow",
            &(id,),
        )
        .await?;
    Ok(())
}
