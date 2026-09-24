//! The results list: the rows grouped under their headings, as quire's `CommandPalette` draws
//! them (44 px rows, 8 visible, selection following keys and hover).
//!
//! The launcher's panel draws this list inside its own palette, with its handlers
//! (`panel.rs`). This component is the same list with no handlers, for a demo or a picture,
//! embedded in its container with `selected` highlighted.

use dioxus::prelude::*;
use sill_launcher::Item;

use crate::surfaces::launcher::entries::{RowRef, palette_groups};
use crate::surfaces::launcher::groups::groups;
use crate::surfaces::launcher::keys::Filter;
use crate::surfaces::launcher::panel::empty_text;

/// `items`, grouped by provider under their top hit.
#[component]
pub fn ResultsList(items: Vec<Item>, selected: usize) -> Element {
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
            host: ds::CommandPaletteHost::Surface,
            selected,
        }
    }
}
