//! What the switcher hands quire's `AppSwitcher`: the tiles, the metrics, and the way between
//! quire's `AppKey` and the machine's indices. Pure.

use super::apps::SwitcherApp;
use ds::prelude::*;
use sill_launcher::AppId;
use sill_model::apps::AppIndex;
use sill_settings::SwitcherSettings;
use sill_shell_kit::switcher::app_switcher::AppKey;
use sill_shell_kit::switcher::app_switcher::SwitcherApp as Tile;
use sill_shell_kit::switcher::switcher_fit::SwitcherMetrics;

/// One tile per app, in order: the key is the app id, the name the desktop entry's `Name` (the
/// id when it has no entry), the icon from `icon` (the caller's resolver, at the cell's icon
/// size), no plate, `Present`.
pub fn tiles(
    apps: &[SwitcherApp],
    index: &AppIndex,
    icon: impl Fn(&AppId) -> IconSource,
) -> Vec<Tile> {
    apps.iter()
        .map(|app| Tile::new(app.app.0.clone(), name_of(&app.app, index), icon(&app.app)))
        .collect()
}

/// The entry's `Name`, else the id made readable ([`readable`]).
fn name_of(app: &AppId, index: &AppIndex) -> String {
    index
        .get(app)
        .map(|entry| entry.name.trim())
        .filter(|name| !name.is_empty())
        .map_or_else(|| readable(&app.0), str::to_owned)
}

/// An id with no desktop entry as a name: its last dotted part, with `-` and `_` as spaces and
/// each word capitalised (`org.mozilla.firefox` is `Firefox`, `gnome-terminal` is `Gnome Terminal`).
fn readable(id: &str) -> String {
    let last = id.rsplit('.').find(|part| !part.is_empty()).unwrap_or(id);
    last.split(['-', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut letters = word.chars();
            letters
                .next()
                .map(|first| first.to_uppercase().chain(letters).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
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

/// quire's key for `app`.
pub fn key_of(app: &AppId) -> AppKey {
    AppKey(app.0.clone())
}

/// The machine's index of `key` in `apps` (what `onhover` and `onactivate` become).
pub fn index_of(key: &AppKey, apps: &[SwitcherApp]) -> Option<usize> {
    apps.iter().position(|app| app.app.0 == key.0)
}

/// The output's width in logical pixels, for quire's overflow fit.
pub fn output_width(width: i32) -> Option<Px> {
    u16::try_from(width)
        .ok()
        .filter(|&width| width > 0)
        .map(|width| Px(f32::from(width)))
}

#[cfg(test)]
mod tests {
    use crate::switcher::apps::{AppShown, SwitcherApp};
    use ds::prelude::*;
    use sill_launcher::AppId;
    use sill_model::apps::AppIndex;
    use sill_settings::SwitcherSettings;
    use sill_shell_kit::switcher::app_switcher::AppKey;
    use sill_shell_kit::switcher::switcher_fit::SwitcherMetrics;

    use super::{index_of, key_of, metrics_of, output_width, tiles};

    fn app(id: &str) -> SwitcherApp {
        SwitcherApp {
            app: AppId(id.to_owned()),
            windows: Vec::new(),
            shown: AppShown::Visible,
        }
    }

    #[test]
    fn tiles_keys_and_metrics() {
        let apps = [app("foot"), app("org.mozilla.firefox")];
        let icon = |_: &AppId| IconSource::Glyph(Icon::ArrowRight);
        let row = tiles(&apps, &AppIndex::default(), icon);
        assert_eq!(
            row.iter().map(|t| t.key.clone()).collect::<Vec<_>>(),
            vec![key_of(&apps[0].app), key_of(&apps[1].app)]
        );
        assert_eq!(row[0].name, "Foot", "no entry: the id, readable");
        assert_eq!(row[1].name, "Firefox", "no entry: the id's last part");
        assert_eq!(index_of(&AppKey("foot".to_owned()), &apps), Some(0));
        assert_eq!(index_of(&AppKey("mail".to_owned()), &apps), None);
        assert_eq!(
            metrics_of(&SwitcherSettings::default()),
            SwitcherMetrics {
                icon: Px(64.0),
                cell: Px(80.0),
                ..SwitcherMetrics::default()
            }
        );
        let set = SwitcherSettings {
            icon_size_px: ds_settings::Px(96),
            cell_size_px: ds_settings::Px(100),
            ..SwitcherSettings::default()
        };
        assert_eq!(metrics_of(&set).icon, Px(96.0), "a set size is kept");
        assert_eq!(output_width(1920), Some(Px(1920.0)));
        assert_eq!(output_width(0), None);
    }
}
