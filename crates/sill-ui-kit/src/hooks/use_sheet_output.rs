//! `use_sheet_output`: where the sheets show now, read through the daemon's
//! `crate::links::SheetLink`.

use dioxus::prelude::{ReadSignal, try_consume_context, use_hook};
use shell_host::use_watch;

use crate::links::{SheetLink, SheetOutput};

/// Where the sheets show now, for a sheet that maps itself only on that output. A root built
/// without a `SheetLink` follows a default one (nothing shows).
pub fn use_sheet_output() -> ReadSignal<SheetOutput> {
    let link = use_hook(|| try_consume_context::<SheetLink>().unwrap_or_default());
    use_watch(link.subscribe())
}
