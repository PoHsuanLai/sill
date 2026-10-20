//! `use_sheet_output`: where the sheets show now, read through the daemon's
//! `crate::links::SheetLink`.

use dioxus::prelude::{ReadSignal, try_consume_context, use_hook};
use shell_host::use_watch;

use super::use_link::tell_if_missing;
use crate::links::{SheetLink, SheetOutput};

/// Where the sheets show now, for a sheet that maps itself only on that output. A root built
/// without a `SheetLink` follows a default one (nothing shows) and says so once on stderr.
pub fn use_sheet_output() -> ReadSignal<SheetOutput> {
    let found = use_hook(|| try_consume_context::<SheetLink>());
    tell_if_missing(&found);
    let link = use_hook(|| found.clone().unwrap_or_default());
    use_watch(link.subscribe())
}
