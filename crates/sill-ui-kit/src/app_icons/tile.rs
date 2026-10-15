//! A tile's icon, as the switcher and the Split View picker pick one (design/08 §4): the entry's own icon file in the
//! icon style (re-tinted under Muted and Monochrome), a `*-symbolic` file in the text colour,
//! and quire's `Window` glyph for an app with no icon (or no entry). Every entry takes this one
//! path. quire's `AppSwitcher` stretches a file to the cell's icon square, so it is loaded at
//! `switcher.icon_size_px` times the scale.

use std::path::Path;

use ds::prelude::*;
use ds::style::icon::render::IconPx;
use ds::style::icon::url::IconUrl;
use sill_launcher::AppId;
use sill_model::apps::{AppIcon, AppIndex};
use sill_settings::SwitcherSettings;
use sill_shell_kit::switcher::switcher_fit::SwitcherMetrics;

use super::{IconContext, LogicalPx, pick, theme_art};

/// What a surface's tile icons depend on.
#[derive(Debug, Clone)]
pub struct TileIcons {
    pub cx: IconContext,
    pub index: AppIndex,
    /// The tile's icon side (`switcher.icon_size_px` for the switcher).
    pub side: LogicalPx,
}

impl TileIcons {
    /// `app`'s icon.
    pub fn source(&self, app: &AppId) -> IconSource {
        let px = pick(self.side, self.cx.scale);
        let resolver = self.cx.resolver;
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

/// The icon and cell sizes sill draws when `switcher.*` leaves them at quire's defaults (96 and
/// 112): design/20 section 1.11 plates the icons at 64, so the cell is 80. Until quire's
/// `SwitcherSettings::default` moves there, a pair at quire's defaults reads as not set.
const ICON: ds_settings::Px = ds_settings::Px(64);
const CELL: ds_settings::Px = ds_settings::Px(80);

/// The icon and cell sides to draw, in `switcher.*`'s units.
fn sides(settings: &SwitcherSettings) -> (ds_settings::Px, ds_settings::Px) {
    let quire = SwitcherSettings::default();
    if settings.icon_size_px == quire.icon_size_px && settings.cell_size_px == quire.cell_size_px {
        (ICON, CELL)
    } else {
        (settings.icon_size_px, settings.cell_size_px)
    }
}

/// The icon's side to resolve art at (`switcher.icon_size_px`, [`sides`]).
pub fn icon_side(settings: &SwitcherSettings) -> ds_settings::Px {
    sides(settings).0
}

/// quire's metrics from `switcher.*` (`icon_size_px`, `cell_size_px`, `cell_gap_px`,
/// `overflow_min_icon_px`).
pub fn metrics_of(settings: &SwitcherSettings) -> SwitcherMetrics {
    let (icon, cell) = sides(settings);
    SwitcherMetrics {
        icon: px(icon),
        cell: px(cell),
        gap: px(settings.cell_gap_px),
        min_icon: px(settings.overflow_min_icon_px),
    }
}

fn px(value: ds_settings::Px) -> Px {
    Px(f32::from(value.0))
}

#[cfg(test)]
mod tests {
    use ds::prelude::*;
    use sill_settings::SwitcherSettings;
    use sill_shell_kit::switcher::switcher_fit::SwitcherMetrics;

    use super::metrics_of;

    #[test]
    fn metrics_follow_the_settings_and_default_to_the_designed_sizes() {
        assert_eq!(
            metrics_of(&SwitcherSettings::default()),
            SwitcherMetrics {
                icon: Px(64.0),
                cell: Px(80.0),
                ..SwitcherMetrics::default()
            }
        );
        let mut set = SwitcherSettings::default();
        set.icon_size_px = ds_settings::Px(96);
        set.cell_size_px = ds_settings::Px(100);
        assert_eq!(metrics_of(&set).icon, Px(96.0), "a set size is kept");
    }
}
