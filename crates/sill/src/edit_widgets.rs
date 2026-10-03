//! The desktop's writes on the daemon's side: the writer the Edit Widgets link holds. An edit
//! saves the notification center's widgets (`widgets.center`) and `widgets.desktop`, which an add
//! to the desktop turns on, into `settings.toml` (publishing them at once, as `toggle-desktop`
//! does, so the desktop layer and the center follow before the watch fires), and the editing
//! output's own widgets and places into `desktop-widgets.json`. The desktop menu's View Options
//! save the grid's cell and gap the same way.

use std::sync::Arc;

use ds_settings::Store;
use sill_settings::ShellFile;
use sill_ui_kit::links::{DesktopView, PlacementFile, WidgetsChange, WidgetsWrite, WidgetsWriter};
use tokio::sync::watch;

/// The writer the Edit Widgets link is given: [`write`] and [`write_view`] into `store` and the
/// default places' file.
pub fn writer(
    store: Store,
    places: PlacementFile,
    settings: watch::Sender<ShellFile>,
) -> WidgetsWriter {
    WidgetsWriter(Arc::new(move |change: WidgetsChange| {
        let done = match change {
            WidgetsChange::Edit(edit) => write(&store, &settings, Some(&places), edit),
            WidgetsChange::View(view) => write_view(&store, &settings, view),
        };
        if let Err(error) = done {
            eprintln!("sill: the desktop's widgets: {error}");
        }
    }))
}

/// Save `change`'s keys into `store`'s `settings.toml`, keeping every other key, publish the
/// file on `settings`, and save its output's widgets and places in `places` when there is a
/// file.
pub fn write(
    store: &Store,
    settings: &watch::Sender<ShellFile>,
    places: Option<&PlacementFile>,
    change: WidgetsWrite,
) -> Result<(), String> {
    let mut file = crate::daemon::settings::load(store);
    file.widgets.center = change.center;
    file.widgets.desktop = change.shown;
    crate::daemon::settings::save(store, &file)
        .map_err(|error| format!("saving the settings: {error}"))?;
    settings.send_replace(file);
    if let Some(places) = places {
        places
            .save_layout(&change.output, &change.desktop, &change.places)
            .map_err(|error| format!("saving {}: {error}", places.0.display()))?;
    }
    Ok(())
}

/// Save the grid's `view` into `store`'s `settings.toml` and publish it on `settings`.
pub fn write_view(
    store: &Store,
    settings: &watch::Sender<ShellFile>,
    view: DesktopView,
) -> Result<(), String> {
    let mut file = crate::daemon::settings::load(store);
    file.widgets.desktop_cell_px = view.cell;
    file.widgets.desktop_gap_px = view.gap;
    crate::daemon::settings::save(store, &file)
        .map_err(|error| format!("saving the settings: {error}"))?;
    settings.send_replace(file);
    Ok(())
}

#[cfg(test)]
mod tests {
    use sill_settings::{DesktopWidgets, Px, ShellFile, WidgetKind};
    use sill_ui_kit::links::DesktopView;
    use sill_ui_kit::links::PlacementFile;
    use sill_ui_kit::links::WidgetsWrite;
    use sill_ui_kit::widgets::grid::{GridSlot, OutputKey, Placement};
    use tokio::sync::watch;

    use super::{write, write_view};

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sill-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn an_edit_writes_the_center_key_and_the_outputs_widgets_and_places() {
        let dir = scratch("edit-widgets");
        let places = PlacementFile(dir.join("state").join("desktop-widgets.json"));
        let (tx, rx) = watch::channel(ShellFile::default());
        let mut before = ShellFile::default();
        before.widgets.world_clocks = "Europe/London".to_owned();
        let store = crate::daemon::settings::store(&dir);
        crate::daemon::settings::save(&store, &before).unwrap_or_else(|e| panic!("{e}"));
        let clock = Placement {
            kind: WidgetKind::WorldClock,
            slot: GridSlot { column: 2, row: 0 },
        };
        let dp1 = OutputKey::Named("DP-1".to_owned());
        let change = WidgetsWrite {
            center: vec![WidgetKind::UpNext],
            desktop: vec![WidgetKind::Calendar, WidgetKind::WorldClock],
            shown: DesktopWidgets::On,
            output: dp1.clone(),
            places: vec![clock],
        };
        write(&store, &tx, Some(&places), change).unwrap_or_else(|e| panic!("{e}"));
        let saved = crate::daemon::settings::load(&store).widgets;
        assert_eq!(saved.center, vec![WidgetKind::UpNext]);
        assert_eq!(
            saved.desktop_widgets, before.widgets.desktop_widgets,
            "the setting is the default for outputs with no list; an edit does not rewrite it"
        );
        assert_eq!(
            saved.desktop,
            DesktopWidgets::On,
            "an add to the desktop shows it"
        );
        assert_eq!(
            saved.world_clocks, "Europe/London",
            "every other key is kept"
        );
        assert_eq!(rx.borrow().widgets.desktop, DesktopWidgets::On);
        assert_eq!(places.load(&dp1), vec![clock]);
        assert_eq!(
            places.kinds(&dp1),
            Some(vec![WidgetKind::Calendar, WidgetKind::WorldClock])
        );
        assert_eq!(
            places.kinds(&OutputKey::Named("HDMI-A-1".to_owned())),
            None,
            "another output keeps following the setting"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn view_options_write_only_the_grids_measures() {
        let dir = scratch("view-options");
        let (tx, rx) = watch::channel(ShellFile::default());
        let mut before = ShellFile::default();
        before.widgets.world_clocks = "Europe/London".to_owned();
        let store = crate::daemon::settings::store(&dir);
        crate::daemon::settings::save(&store, &before).unwrap_or_else(|e| panic!("{e}"));
        let view = DesktopView {
            cell: Px(196),
            gap: Px(8),
        };
        write_view(&store, &tx, view).unwrap_or_else(|e| panic!("{e}"));
        let saved = crate::daemon::settings::load(&store).widgets;
        assert_eq!(
            (saved.desktop_cell_px, saved.desktop_gap_px),
            (Px(196), Px(8))
        );
        assert_eq!(
            saved.world_clocks, "Europe/London",
            "every other key is kept"
        );
        assert_eq!(saved.center, before.widgets.center);
        let published = rx.borrow().widgets.clone();
        assert_eq!(
            (published.desktop_cell_px, published.desktop_gap_px),
            (Px(196), Px(8))
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
