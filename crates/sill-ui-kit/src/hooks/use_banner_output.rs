//! `use_banner_output`: where the banners show now, read through the daemon's
//! `crate::links::BannerLink`.

use dioxus::prelude::{ReadSignal, try_consume_context, use_hook};
use shell_host::use_watch;

use crate::links::{BannerLink, BannerOutput};

/// Where the banners show now, for a sheet that maps itself only on that output. A root built
/// without a `BannerLink` follows a default one (nothing shows).
pub fn use_banner_output() -> ReadSignal<BannerOutput> {
    let link = use_hook(|| try_consume_context::<BannerLink>().unwrap_or_default());
    use_watch(link.subscribe())
}
