//! The lock surface's SSR render (design/20 §1.9): the clock on
//! every output, the prompt only on the output the `LockLink` names, the prompt's state from the
//! auth service, the unlock fade's mark on `Accepted`, and the blurred backdrop as the stage's
//! picture. Outside a lock document (no `LockHandle`), as every test is.

mod support;

use dioxus::prelude::*;
use shell_host::OutputId;
use sill_services::auth::{Account, RealName, UserName};
use sill_services::{Auth, AuthPhase, OutputName, WallpaperImage, Wallpapers};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::LockScreen;
use sill_surfaces::surfaces::lock::{LockLink, LockScreenProps, PromptOutput};
use tokio::sync::watch;

fn ada(phase: AuthPhase) -> Auth {
    Auth {
        account: Account {
            user: UserName("ada".to_owned()),
            name: RealName("Ada Lovelace".to_owned()),
            ..Account::default()
        },
        phase,
        ..Auth::default()
    }
}

fn render(auth: Auth, prompt: PromptOutput, wallpapers: Wallpapers) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (mut services, _feeds) = support::fake::services();
    let (_auth_tx, auth_rx) = watch::channel(auth);
    services.auth.state = auth_rx;
    let (_wallpaper_tx, wallpaper_rx) = watch::channel(wallpapers);
    services.wallpaper.state = wallpaper_rx;
    let link = LockLink::new();
    link.set(prompt);
    let mut dom = VirtualDom::new_with_props(
        LockScreen,
        LockScreenProps {
            output: support::fake::output(),
        },
    );
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.provide_root_context(link);
    dom.rebuild_in_place();
    markup_only(&dioxus_ssr::render(&dom))
}

/// The markup without its `<style>` blocks (quire's stylesheet names every class).
fn markup_only(html: &str) -> String {
    let mut rest = html;
    let mut kept = String::new();
    while let Some((before, after)) = rest.split_once("<style>") {
        kept.push_str(before);
        rest = after.split_once("</style>").map_or("", |(_, tail)| tail);
    }
    kept.push_str(rest);
    kept
}

fn here() -> PromptOutput {
    PromptOutput::On(support::fake::output().id)
}

#[test]
fn the_prompt_shows_on_its_output_with_the_clock() {
    let html = render(ada(AuthPhase::Idle), here(), Wallpapers::default());
    assert!(html.contains("ds-lock-clock"), "{html}");
    assert!(html.contains("ds-lock-prompt"), "{html}");
    assert!(html.contains("Ada Lovelace"), "{html}");
    assert!(html.contains("data-state=\"idle\""), "{html}");
    assert!(!html.contains("data-leaving"), "{html}");
}

#[test]
fn another_output_shows_the_clock_only() {
    for elsewhere in [PromptOutput::On(OutputId(9)), PromptOutput::Nowhere] {
        let html = render(ada(AuthPhase::Idle), elsewhere, Wallpapers::default());
        assert!(html.contains("ds-lock-clock"), "{html}");
        assert!(!html.contains("ds-lock-prompt"), "{html}");
    }
}

#[test]
fn the_prompt_follows_the_check() {
    let cases = [
        (AuthPhase::Checking, "data-state=\"checking\""),
        (AuthPhase::Wrong, "data-state=\"wrong\""),
        (AuthPhase::Accepted, "data-leaving=\"yes\""),
    ];
    for (phase, want) in cases {
        let html = render(ada(phase.clone()), here(), Wallpapers::default());
        assert!(html.contains(want), "{phase:?}: {html}");
    }
}

#[test]
fn a_written_backdrop_is_the_stages_picture() {
    let dir = std::env::temp_dir().join(format!("sill-lock-render-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
    let image = dir.join("aa-1920x1080.png");
    std::fs::write(dir.join("aa-1920x1080-backdrop.png"), b"png").unwrap_or_else(|e| panic!("{e}"));
    let wallpapers = Wallpapers {
        outputs: vec![WallpaperImage {
            output: OutputName("DP-1".to_owned()),
            light: image.clone(),
            dark: image,
        }],
    };
    let html = render(ada(AuthPhase::Idle), here(), wallpapers);
    assert!(html.contains("aa-1920x1080-backdrop.png"), "{html}");
    let missing = Wallpapers {
        outputs: vec![WallpaperImage {
            output: OutputName("DP-1".to_owned()),
            light: dir.join("bb-1920x1080.png"),
            dark: dir.join("bb-1920x1080.png"),
        }],
    };
    let html = render(ada(AuthPhase::Idle), here(), missing);
    assert!(
        !html.contains("background-image"),
        "no backdrop yet: {html}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
