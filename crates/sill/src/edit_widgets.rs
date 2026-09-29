//! Edit Widgets on the daemon's side: the writer its link holds, which saves the widget keys
//! into `settings.toml` (`widgets.center`, `widgets.desktop_widgets`, and `widgets.desktop`,
//! which an add to the desktop turns on) (publishing them at once, as `toggle-desktop` does, so
//! the desktop layer and the center follow before the watch fires) and the output's places into
//! `desktop-widgets.json`.

use std::sync::Arc;

use ds_settings::Store;
use sill_settings::ShellFile;
use sill_ui_kit::links::{PlacementFile, WidgetsWrite, WidgetsWriter};
use tokio::sync::watch;

/// The writer the Edit Widgets link is given: [`write`] into `store` and the default places' file.
pub fn writer(
    store: Store,
    places: PlacementFile,
    settings: watch::Sender<ShellFile>,
) -> WidgetsWriter {
    WidgetsWriter(Arc::new(move |change: WidgetsWrite| {
        if let Err(error) = write(&store, &settings, Some(&places), change) {
            eprintln!("sill: Edit Widgets: {error}");
        }
    }))
}

/// Save `change`'s keys into `store`'s `settings.toml`, keeping every other key, publish the
/// file on `settings`, and save its places in `places` when there is a file.
pub fn write(
    store: &Store,
    settings: &watch::Sender<ShellFile>,
    places: Option<&PlacementFile>,
    change: WidgetsWrite,
) -> Result<(), String> {
    let mut file = crate::daemon::settings::load(store);
    file.widgets.center = change.center;
    file.widgets.desktop_widgets = change.desktop;
    file.widgets.desktop = change.shown;
    crate::daemon::settings::save(store, &file)
        .map_err(|error| format!("saving the settings: {error}"))?;
    settings.send_replace(file);
    if let Some(places) = places {
        places
            .save(&change.output, &change.places)
            .map_err(|error| format!("saving {}: {error}", places.0.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sill_settings::{DesktopWidgets, ShellFile, WidgetKind};
    use sill_ui_kit::links::PlacementFile;
    use sill_ui_kit::links::WidgetsWrite;
    use sill_ui_kit::widgets::grid::{GridSlot, OutputKey, Placement};
    use tokio::sync::watch;

    use super::write;

    #[test]
    fn an_edit_writes_the_widget_keys_and_the_outputs_places() {
        let dir = std::env::temp_dir().join(format!("sill-edit-widgets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
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
        let change = WidgetsWrite {
            center: vec![WidgetKind::UpNext],
            desktop: vec![WidgetKind::Calendar, WidgetKind::WorldClock],
            shown: DesktopWidgets::On,
            output: OutputKey::Named("DP-1".to_owned()),
            places: vec![clock],
        };
        write(&store, &tx, Some(&places), change).unwrap_or_else(|e| panic!("{e}"));
        let saved = crate::daemon::settings::load(&store).widgets;
        assert_eq!(saved.center, vec![WidgetKind::UpNext]);
        assert_eq!(
            saved.desktop_widgets,
            vec![WidgetKind::Calendar, WidgetKind::WorldClock]
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
        assert_eq!(
            places.load(&OutputKey::Named("DP-1".to_owned())),
            vec![clock]
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
