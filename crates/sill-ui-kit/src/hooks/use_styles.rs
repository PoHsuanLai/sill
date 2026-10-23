//! `use_styles`: the styles every `Ds` root draws, live, and whether the person's are drawn.

use dioxus::prelude::{ReadSignal, try_consume_context, use_hook};
use ds::style::kit::{CustomStyles, Styles};
use ds_blitz::StylesGuard;
use ds_blitz::styles_guard::Load;
use shell_host::use_watch;
use tokio::sync::watch;

use crate::hooks::use_appearance;
use crate::settings::SettingsFeed;

/// What a `Ds` root takes for its styles: the `styles` prop and the `custom_styles` prop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StyleSource {
    /// The three sheets, live; empty when the document was built without a [`SettingsFeed`].
    pub styles: ReadSignal<Styles>,
    /// `appearance.custom_styles`, forced to `Off` while the crash guard has the styles off.
    pub custom_styles: CustomStyles,
}

/// The [`StyleSource`] of this document. Call it before the component's first early return.
pub fn use_styles() -> StyleSource {
    let feed = use_hook(try_consume_context::<SettingsFeed>);
    let (styles, guard) = use_hook(|| match feed {
        Some(feed) => (feed.styles, feed.guard),
        None => (watch::channel(Styles::default()).1, watch::channel(None).1),
    });
    let styles = use_watch(styles);
    let guard = use_watch(guard);
    let appearance = use_appearance();
    let withheld = guard().as_ref().map(StylesGuard::load) == Some(Load::Without);
    let custom_styles = match withheld {
        true => CustomStyles::Off,
        false => appearance().settings.appearance.custom_styles,
    };
    StyleSource {
        styles,
        custom_styles,
    }
}
