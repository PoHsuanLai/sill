//! `use_user_style`: the person's `style.css`, live.

use dioxus::prelude::{ReadSignal, try_consume_context, use_hook};
use ds_settings::UserStyle;
use shell_host::use_watch;
use tokio::sync::watch;

use crate::settings::SettingsFeed;

/// The person's stylesheet as last settled; empty when the document was built without a
/// [`SettingsFeed`] (a popup its opener gave none) or there is no file.
pub fn use_user_style() -> ReadSignal<UserStyle> {
    let receiver = use_hook(|| match try_consume_context::<SettingsFeed>() {
        Some(feed) => feed.user_style,
        None => watch::channel(UserStyle::default()).1,
    });
    use_watch(receiver)
}
