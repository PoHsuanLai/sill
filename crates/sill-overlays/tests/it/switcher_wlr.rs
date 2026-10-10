//! The switcher on a scripted ext/wlr session (desktop-client's fake): the apps it lists come
//! from the windows the standard protocols report, in the order the windows were last active.

use std::time::Duration;

use desktop_client::fake::FakeDesktop;
use desktop_client::toplevel::{ToplevelChange, ToplevelEvent, ToplevelFlag, ToplevelFlags};
use desktop_client::{
    Desktop, DesktopEvent, Protocol, ProtocolSet, ToplevelIdentifier, ToplevelKey,
};
use ds_harness::Viewport;
use sill_model::cosmic_wl::Toplevels;
use sill_overlays::Switcher;
use sill_services::desktop_wl::project_toplevels;
use sill_settings::ShellFile;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{Dir, SwitcherAsk, SwitcherLink};

fn tk(n: u64) -> ToplevelKey {
    ToplevelKey::new(n)
}

fn opens(key: u64, app: &str) -> Vec<DesktopEvent> {
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
        DesktopEvent::Toplevel(ToplevelEvent::Done(tk(key))),
    ]
}

fn activates(key: u64) -> DesktopEvent {
    DesktopEvent::Toplevel(ToplevelEvent::Changed {
        key: tk(key),
        change: ToplevelChange::Flags(ToplevelFlags::of([ToplevelFlag::Activated])),
    })
}

/// The tiles' names in order (`aria-label`), and the selected one's.
fn tiles(html: &str) -> (Vec<String>, Option<String>) {
    let attr = |tag: &str, name: &str| {
        let needle = format!("{name}=\"");
        let rest = &tag[tag.find(&needle)? + needle.len()..];
        Some(rest[..rest.find('"')?].to_owned())
    };
    let mut names = Vec::new();
    let mut selected = None;
    let mut rest = html;
    while let Some(at) = rest.find("role=\"option\"") {
        rest = &rest[at..];
        let tag = &rest[..rest.find('>').unwrap_or(rest.len())];
        let name = attr(tag, "aria-label").unwrap_or_default();
        if attr(tag, "aria-selected").as_deref() == Some("true") {
            selected = Some(name.clone());
        }
        names.push(name);
        rest = &rest["role=\"option\"".len()..];
    }
    (names, selected)
}

#[test]
fn the_list_is_the_apps_of_the_reported_windows_most_recently_active_first() {
    let mut desktop = FakeDesktop::new(
        ProtocolSet::empty()
            .with(Protocol::ExtForeignToplevelList)
            .with(Protocol::WlrForeignToplevelManagement),
    );
    for (key, app) in [(1, "term"), (2, "calc"), (3, "edit"), (4, "calc")] {
        opens(key, app)
            .into_iter()
            .for_each(|event| desktop.script(event));
    }
    desktop.advance_all();
    let first = project_toplevels(Toplevels::default(), desktop.state());
    // The compositor activates calc's second window: its app moves to the front.
    desktop.script(activates(4));
    desktop.advance_all();
    let toplevels = project_toplevels(first, desktop.state());

    let mut running = SurfaceHarness::<Switcher>::setup(SwitcherLink::new(), ())
        .settings(ShellFile::default())
        .viewport(Viewport {
            width: 1280,
            height: 720,
            scale_percent: 100,
        })
        .start();
    running
        .feeds
        .cosmic_wl
        .send_modify(|wl| wl.toplevels = toplevels);
    running.harness.advance(Duration::from_millis(40));
    running
        .link
        .ask(fake::output().id, SwitcherAsk::Chord(Dir::Next));
    running.harness.advance(Duration::from_millis(400));

    let names = |list: &[&str]| list.iter().map(|&s| s.to_owned()).collect::<Vec<_>>();
    assert_eq!(
        tiles(&running.harness.html()),
        (names(&["calc", "term", "edit"]), Some("term".to_owned())),
        "calc first, and the chord selects the previous app"
    );
}
