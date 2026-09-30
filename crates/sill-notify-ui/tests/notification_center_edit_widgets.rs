//! The notification center's "Edit Widgets" button closes the center and opens the gallery on
//! its output: the real `NotificationCenter` on `SurfaceHarness` on the virtual clock and
//! the fake services, with the `EditWidgetsLink` the daemon shares between the two surfaces.

use std::time::Duration;

use dioxus::prelude::*;
use ds_harness::Viewport;
use ds_harness::harness::{assert_settles_to_zero_frames, settle_until};
use sill_notify_ui::NotificationCenter;
use sill_settings::{ShellFile, WidgetKind};
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{CenterLink, CenterShown, EditShown, EditWidgetsLink};

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

fn click(running: &mut SurfaceHarness<NotificationCenter>, selector: &str) {
    let at = running
        .harness
        .centre(selector)
        .unwrap_or_else(|| panic!("{selector} is laid out: {}", running.harness.html()));
    running.harness.click(at);
    running.harness.advance(Duration::from_millis(50));
}

#[test]
fn the_centers_edit_widgets_opens_the_gallery_on_its_output() {
    // One tile, so the button is in view without scrolling the column.
    let mut file = ShellFile::default();
    file.widgets.center = vec![WidgetKind::Battery];
    let edit = EditWidgetsLink::new();
    let shared = edit.clone();
    let mut run = SurfaceHarness::<NotificationCenter>::setup(CenterLink::new(), ())
        .settings(file)
        .viewport(VIEWPORT)
        .in_root(move || {
            let edit = shared.clone();
            use_context_provider(|| edit);
        })
        .start();
    let output = fake::output().id;
    run.link.open(output);
    settle_until(&mut run.harness, |h| {
        h.count(".sill-nc-edit .ds-button") == 1
    });
    assert_eq!(
        run.harness.text_of(".sill-nc-edit .ds-button").as_deref(),
        Some("Edit Widgets")
    );
    let widgets = run.harness.rect(".sill-nc-widgets").expect("the widgets");
    let button = run.harness.rect(".sill-nc-edit").expect("the button");
    assert!(
        button.origin.y.0
            >= widgets.origin.y.0 + widgets.size.height.0 - button.size.height.0 - 0.5,
        "at the bottom of the widgets: {widgets:?} {button:?}"
    );
    assert_settles_to_zero_frames(&mut run.harness);
    click(&mut run, ".sill-nc-edit .ds-button");
    assert_eq!(run.link.now(), CenterShown::Closed);
    assert_eq!(edit.now(), EditShown::Open(output));
    settle_until(&mut run.harness, |h| h.count(".ds-side-panel") == 0);
    assert_settles_to_zero_frames(&mut run.harness);
}
