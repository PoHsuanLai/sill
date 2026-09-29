//! The merged result list.

use serde::{Deserialize, Serialize};
use sill_launcher::{Generation, Item, SectionSpan};

/// The rows of the newest generation, best first: score descending, then provider (Tab
/// order), then title. Items are appended batch by batch as providers answer.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Results {
    pub generation: Generation,
    pub items: Vec<Item>,
    /// The published rows' sections, in order, covering every row (`sill_launcher::place`).
    #[serde(default)]
    pub sections: Vec<SectionSpan>,
}
