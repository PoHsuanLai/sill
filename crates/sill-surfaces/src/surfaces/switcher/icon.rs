//! A tile's icon, as the dock picks one (design/08 §4): quire's shipped icon for an app it
//! stands for (`dock.shipped_icons`), else the entry's own icon file in the icon style
//! (re-tinted under Muted and Monochrome), a `*-symbolic` file in the text colour, and quire's
//! `Window` glyph for an app with no icon (or no entry). quire's `AppSwitcher` stretches a file
//! to the cell's icon square, so it is loaded at `switcher.icon_size_px` times the scale.

use std::path::Path;

use ds::{ExternalIcon, Icon, IconPx, IconSize, IconSource, IconUrl};
use sill_launcher::AppId;
use sill_services::{AppIcon, AppIndex};
use sill_settings::ShippedIcons;

use crate::app_icons::{
    DEFAULT_TABLE, IconContext, LogicalPx, pick, shipped_app, shipped_art, theme_art,
};

/// What a switcher surface's icons depend on.
#[derive(Debug, Clone)]
pub struct SwitcherIcons {
    pub cx: IconContext,
    pub index: AppIndex,
    pub shipped: ShippedIcons,
    /// `switcher.icon_size_px`.
    pub side: LogicalPx,
}

impl SwitcherIcons {
    /// `app`'s icon.
    pub fn source(&self, app: &AppId) -> IconSource {
        let px = pick(self.side, self.cx.scale);
        let resolver = self.cx.resolver;
        if let Some(ours) = shipped_app(app, self.shipped, DEFAULT_TABLE)
            && let Some(source) = resolver.shipped_path(ours, px, self.cx.look.style)
        {
            let file = resolver.path(&shipped_art(source, ours, px, self.cx.look));
            return self.external(&file).map_or(window(), IconSource::Image);
        }
        match self.index.get(app).map(|entry| &entry.icon) {
            Some(AppIcon::File(path)) if path.is_absolute() && symbolic(path) => {
                self.external(path).map_or(window(), IconSource::Symbolic)
            }
            Some(AppIcon::File(path)) if path.is_absolute() => {
                let file = resolver.path(&theme_art(path, px, self.cx.look));
                self.external(&file).map_or(window(), IconSource::Image)
            }
            Some(AppIcon::File(_) | AppIcon::None) | None => window(),
        }
    }

    fn external(&self, path: &Path) -> Option<ExternalIcon> {
        Some(ExternalIcon {
            url: IconUrl::file(path).ok()?,
            size: IconSize::Px(IconPx(u8::try_from(self.side.0).unwrap_or(u8::MAX))),
        })
    }
}

fn window() -> IconSource {
    IconSource::Glyph(Icon::Window)
}

/// Whether a file is a freedesktop `*-symbolic` icon.
fn symbolic(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.ends_with("-symbolic"))
}
