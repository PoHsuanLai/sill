//! The wallpaper's SSR render reads the frozen `wallpapers` channel: an entry for its own
//! output shows that entry's light-scheme path as a `file://` URL (the fake `use_appearance`
//! environment resolves to the light scheme, `ds-settings`' own default), an entry for another
//! output is ignored, and no entry paints no image at all (`--paper` shows through instead).

mod support;

use dioxus::prelude::*;
use sill_model::ids::OutputName;
use sill_services::{WallpaperImage, Wallpapers};
use sill_surfaces::Wallpaper;
use sill_surfaces::surfaces::wallpaper::WallpaperProps;
use tokio::sync::watch;

fn render(fed: Wallpapers) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (mut services, _feeds) = support::fake::services();
    let (_tx, rx) = watch::channel(fed);
    services.wallpaper.state = rx;
    let mut dom = VirtualDom::new_with_props(
        Wallpaper,
        WallpaperProps {
            output: support::fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn no_entry_for_this_output_paints_no_image() {
    let html = render(Wallpapers::default());
    assert!(!html.contains("<img"), "{html}");
}

#[test]
fn an_entry_for_a_different_output_is_ignored() {
    let fed = Wallpapers {
        outputs: vec![WallpaperImage {
            output: OutputName("HDMI-A-1".to_owned()),
            light: "/tmp/sill-wallpaper-render/other-light.png".into(),
            dark: "/tmp/sill-wallpaper-render/other-dark.png".into(),
        }],
    };
    let html = render(fed);
    assert!(!html.contains("<img"), "{html}");
}

#[test]
fn this_outputs_entry_shows_its_light_path_as_a_file_url() {
    let fed = Wallpapers {
        outputs: vec![WallpaperImage {
            output: OutputName("DP-1".to_owned()),
            light: "/tmp/sill-wallpaper-render/light.png".into(),
            dark: "/tmp/sill-wallpaper-render/dark.png".into(),
        }],
    };
    let html = render(fed);
    assert!(
        html.contains("src=\"file:///tmp/sill-wallpaper-render/light.png\""),
        "{html}"
    );
    assert!(!html.contains("dark.png"), "{html}");
}
