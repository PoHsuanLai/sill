//! The results list: the rows grouped under their headings, as quire's `CommandPalette` draws
//! them (44 px rows, 8 visible, selection following keys and hover).
//!
//! The launcher's panel draws this list inside its own palette, with its handlers
//! (`panel.rs`). This component is the same list with no handlers, for a demo or a picture:
//! the palette keeps its own selection, so `selected` cannot reach it (FINDINGS F141, Q41).

use dioxus::prelude::*;
use sill_launcher::Item;

use crate::surfaces::launcher::entries::{RowRef, palette_groups};
use crate::surfaces::launcher::groups::groups;
use crate::surfaces::launcher::keys::Filter;
use crate::surfaces::launcher::panel::empty_text;

/// `items`, grouped by provider under their top hit.
#[component]
pub fn ResultsList(items: Vec<Item>, selected: usize) -> Element {
    let _palette_owns_the_selection = selected;
    rsx! {
        ds::CommandPalette::<RowRef> {
            label: "Search",
            placeholder: "",
            query: String::new(),
            tokens: Vec::new(),
            groups: palette_groups(&groups(&items, "*", Filter::All)),
            empty: empty_text(""),
            oninput: move |_| {},
            onpick: move |_| {},
            onclose: move |()| {},
        }
    }
}
