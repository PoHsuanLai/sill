//! A tile's icon, as the switcher and the Split View picker pick one (design/08 §4): the entry's own icon file in the
//! icon style (re-tinted under Muted and Monochrome), a `*-symbolic` file in the text colour,
//! and quire's `Window` glyph for an app with no icon (or no entry). Every entry takes this one
//! path. quire's `AppSwitcher` stretches a file to the cell's icon square, so it is loaded at
//! the drawn icon side times the scale.

use std::path::Path;

use ds::prelude::*;
use ds::style::icon::render::IconPx;
use ds::style::icon::url::IconUrl;
use sill_launcher::AppId;
use sill_model::apps::{AppIcon, AppIndex};

use super::{IconContext, LogicalPx, pick, theme_art};

/// What a surface's tile icons depend on.
#[derive(Debug, Clone)]
pub struct TileIcons {
    pub cx: IconContext,
    pub index: AppIndex,
    /// The tile's icon side ([`ICON_SIDE`] for the switcher).
    pub side: LogicalPx,
}

impl TileIcons {
    /// `app`'s icon.
    pub fn source(&self, app: &AppId) -> IconSource {
        let px = pick(self.side, self.cx.scale);
        let resolver = self.cx.resolver;
        match self.index.get(app).map(|entry| &entry.icon) {
            Some(AppIcon::File(path)) if path.is_absolute() && symbolic(path) => self
                .external(path)
                .map_or_else(window, IconSource::Symbolic),
            Some(AppIcon::File(path)) if path.is_absolute() => {
                let file = resolver.path(&theme_art(path, px, self.cx.look));
                self.external(&file).map_or_else(window, IconSource::Image)
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

/// The icon's side to resolve art at: the switcher's drawn icon, 64 (design/20 section 1.11,
/// as [`sill_shell_kit::switcher::switcher_fit::SwitcherMetrics`] draws it).
pub const ICON_SIDE: LogicalPx = LogicalPx(64);
