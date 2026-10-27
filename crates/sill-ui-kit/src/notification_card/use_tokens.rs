//! The notification surfaces' tokens as the cascade resolves them, read by the surface (the
//! banner column, the notification center) and shared with the cards drawn under it.

use dioxus::prelude::*;

use super::metrics::NotificationTokens;
use crate::hooks::{SurfaceTokens, use_surface_tokens};
use crate::root::use_root_style;

/// The surface's resolved tokens (the shipped values until its element mounts: give the reader
/// to that element's `onmounted`), also provided to the cards below as a context.
pub fn use_notification_tokens() -> (SurfaceTokens<NotificationTokens>, NotificationTokens) {
    let resolved = use_surface_tokens(use_root_style().styles, |length| {
        NotificationTokens::read(length)
    });
    let tokens = resolved.get();
    let mut shared = use_context_provider(|| Signal::new(tokens));
    if *shared.peek() != tokens {
        shared.set(tokens);
    }
    (resolved, tokens)
}

/// The tokens the surface above shares: the shipped values outside one.
pub(super) fn shared_tokens() -> NotificationTokens {
    try_use_context::<Signal<NotificationTokens>>()
        .map_or_else(NotificationTokens::default, |tokens| tokens())
}
