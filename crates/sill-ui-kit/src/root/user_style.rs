//! What every `Ds` root takes from the person's desktop beyond the look: its own name, so a user
//! stylesheet can target it (`[data-surface=bar]`), and the person's stylesheet itself. Both are
//! public selector surface (ARCHITECTURE.md section 8): a surface's name is `Surface::NAME`,
//! a popup's is the name its opener gives its [`crate::menu::PopupFrame`].

use dioxus::prelude::{ReadSignal, try_consume_context, use_hook};
use ds_settings::UserStyle;

use crate::hooks::use_user_style;

/// The name of the surface a document is, provided by `mount`, `mount_lock` and `PopupFrame`;
/// the value of `data-surface`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceName(pub &'static str);

/// The two `Ds` props that come from the person: give both to every `Ds` root.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootStyle {
    /// The document's surface name; none in a document no surface mounted.
    pub surface: Option<&'static str>,
    /// The person's stylesheet, live.
    pub user_style: ReadSignal<UserStyle>,
}

impl RootStyle {
    /// The style of a popup's document, named `surface` by its opener: a popup is a document of
    /// its own and holds no [`SurfaceName`]. `user_style` is [`use_user_style`], called before
    /// the popup's first early return; the opener gives the popup the
    /// [`crate::settings::SettingsFeed`] as a root context.
    pub fn popup(surface: &'static str, user_style: ReadSignal<UserStyle>) -> RootStyle {
        RootStyle {
            surface: Some(surface),
            user_style,
        }
    }
}

/// The [`RootStyle`] of the document this component is in.
pub fn use_root_style() -> RootStyle {
    let surface = use_hook(try_consume_context::<SurfaceName>).map(|name| name.0);
    RootStyle {
        surface,
        user_style: use_user_style(),
    }
}
