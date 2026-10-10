//! The dock on a scripted ext/wlr session (desktop-client's fake): the windows the standard
//! protocols report become the running apps' tiles and dots, and a window that closes takes its
//! dot with it.

use std::time::Duration;

use desktop_client::fake::FakeDesktop;
use desktop_client::toplevel::{ToplevelChange, ToplevelEvent, ToplevelFlag, ToplevelFlags};
use desktop_client::{
    Desktop, DesktopEvent, Protocol, ProtocolSet, ToplevelIdentifier, ToplevelKey,
};
use sill_dock::Dock;
use sill_model::cosmic_wl::{CosmicWl, Toplevels};
use sill_services::desktop_wl::{project_toplevels, project_workspaces};
use sill_settings::ShellFile;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::hooks::PinnedAppearance;

fn tk(n: u64) -> ToplevelKey {
    ToplevelKey::new(n)
}

/// A window appearing complete, as the list and the manager describe it together.
fn opens(key: u64, app: &str, flags: &[ToplevelFlag]) -> Vec<DesktopEvent> {
    let change = |change| {
        DesktopEvent::Toplevel(ToplevelEvent::Changed {
            key: tk(key),
            change,
        })
    };
    vec![
        DesktopEvent::Toplevel(ToplevelEvent::Opened(tk(key))),
        change(ToplevelChange::Identifier(ToplevelIdentifier::new(
            format!("id-{key}"),
        ))),
        change(ToplevelChange::AppId(app.to_owned())),
        change(ToplevelChange::Title(format!("{app} {key}"))),
        change(ToplevelChange::Flags(ToplevelFlags::of(
            flags.iter().copied(),
        ))),
        DesktopEvent::Toplevel(ToplevelEvent::Done(tk(key))),
    ]
}

fn script(desktop: &mut FakeDesktop, events: Vec<DesktopEvent>) {
    for event in events {
        desktop.script(event);
    }
    desktop.advance_all();
}

/// What the dock reads after `desktop` said its piece, windows kept in recency by `recency`.
fn published(desktop: &FakeDesktop, recency: Toplevels) -> (CosmicWl, Toplevels) {
    let toplevels = project_toplevels(recency, desktop.state());
    let wl = CosmicWl {
        workspaces: project_workspaces(desktop.state()),
        toplevels: toplevels.clone(),
    };
    (wl, toplevels)
}

fn dots(html: &str) -> usize {
    html.matches("class=\"ds-running-dot\"").count()
}

#[test]
fn windows_from_the_standard_protocols_draw_running_dots_and_closing_removes_them() {
    let mut desktop = FakeDesktop::new(
        ProtocolSet::empty()
            .with(Protocol::ExtForeignToplevelList)
            .with(Protocol::WlrForeignToplevelManagement),
    );
    script(
        &mut desktop,
        opens(10, fake::dock::TERM, &[ToplevelFlag::Activated]),
    );
    script(&mut desktop, opens(12, fake::dock::FIREFOX, &[]));
    script(&mut desktop, opens(11, fake::dock::TERM, &[]));
    let (wl, recency) = published(&desktop, Toplevels::default());

    let mut run = SurfaceHarness::<Dock>::setup((), ())
        .settings(ShellFile::default())
        .appearance(PinnedAppearance::default())
        .start();
    run.feeds.apps.send_replace(fake::dock::index());
    run.feeds.pins.send_replace(fake::dock::pins());
    run.feeds.cosmic_wl.send_replace(wl);
    run.harness.advance(Duration::from_millis(40));
    let running = run.harness.html();
    assert_eq!(
        dots(&running),
        2,
        "Terminal (two windows, one dot) and Firefox run: {running}"
    );

    script(
        &mut desktop,
        vec![
            DesktopEvent::Toplevel(ToplevelEvent::Closed(tk(10))),
            DesktopEvent::Toplevel(ToplevelEvent::Closed(tk(11))),
        ],
    );
    let (wl, _) = published(&desktop, recency);
    run.feeds.cosmic_wl.send_replace(wl);
    run.harness.advance(Duration::from_millis(40));
    let after = run.harness.html();
    assert_eq!(dots(&after), 1, "only Firefox still runs: {after}");
}
