//! One launcher result and its actions.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::chord::ActionKeys;
use crate::ids::{ActionId, ItemId, ProviderKind};
use crate::rank::Score;
use crate::subject::Subject;

/// One row of results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub provider: ProviderKind,
    /// The row's title: an app name, a window title, `= 42`.
    pub title: String,
    /// The faint second line; empty for none.
    pub subtitle: String,
    pub icon: IconRef,
    /// The provider's rank for this row ([`crate::rank`]); rows sort by it, highest first.
    pub score: Score,
    /// Secondary actions (Ctrl+K / Alt+K), in menu order; empty for none.
    pub actions: Vec<Action>,
    /// What the row is about, for the preview pane and the M9 row kinds (M9 freeze, F641).
    #[serde(default)]
    pub subject: Subject,
}

/// Where a row's icon comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum IconRef {
    /// No icon; the row shows its provider's glyph.
    None,
    /// A resolved icon file (the app index resolves theme names once).
    File(PathBuf),
    /// A quire `Icon` by its kebab-case name (`calculator`, `settings`, `power`).
    Glyph(String),
}

/// One secondary action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    pub id: ActionId,
    pub label: String,
    /// The chord that runs it straight from the results, shown beside it (quire design/09 H5:
    /// every action has a key). M9 freeze, F642.
    #[serde(default)]
    pub keys: ActionKeys,
}

/// What was chosen on a row: Enter, or an action from its actions menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Choice {
    /// Enter or click.
    Primary,
    /// One of [`Item::actions`].
    Action(ActionId),
}
