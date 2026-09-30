//! The catcher closes the launcher on a press anywhere on it (design/06 §20.2), in a
//! `ds_harness::Harness`.

mod launcher_harness;

use std::time::Duration;

use ds_harness::Viewport;
use launcher_harness::launcher_support::fake;
use sill_launcher_ui::LauncherCatcher;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::LauncherLink;

#[test]
fn a_press_on_the_catcher_closes_the_launcher() {
    let link = LauncherLink::new();
    link.toggle(fake::output().id);
    let mut catcher = SurfaceHarness::<LauncherCatcher>::setup(link.clone(), ())
        .viewport(Viewport {
            width: 960,
            height: 540,
            scale_percent: 100,
        })
        .start();
    catcher.harness.advance(Duration::from_millis(20));
    assert!(link.now().opening().is_some());
    catcher.harness.click(ds::Point {
        x: ds::Px(20.0),
        y: ds::Px(500.0),
    });
    catcher.harness.advance(Duration::from_millis(20));
    assert!(link.now().opening().is_none(), "{}", catcher.harness.html());
}
