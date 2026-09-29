//! The lock surface (design/20 §1.9) on `LockHarness`: the clock on
//! every output, the prompt only on the output the `LockLink` names, the prompt's state from the
//! auth service, the unlock fade's mark on `Accepted`, and the blurred backdrop as the stage's
//! picture. Outside a lock document (no `LockHandle`), as every test is.

use std::time::Duration;

use shell_host::OutputId;
use sill_model::auth::{Account, Auth, AuthPhase, RealName, UserName};
use sill_model::ids::OutputName;
use sill_model::wallpaper::{WallpaperImage, Wallpapers};
use sill_ui_kit::Lock;
use sill_ui_kit::harness::{LockHarness, fake};
use sill_ui_kit::links::{LockLink, PromptOutput};
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
    let link = LockLink::new();
    link.set(prompt);
    // The senders outlive the render; the services only hold the receivers.
    let (_auth_tx, auth_rx) = watch::channel(auth);
    let (_wallpaper_tx, wallpaper_rx) = watch::channel(wallpapers);
    let mut run = LockHarness::<Lock>::setup(link)
        .services(move |services| {
            services.auth.state = auth_rx;
            services.wallpaper.state = wallpaper_rx;
        })
        .start();
    run.harness.advance(Duration::ZERO);
    markup_only(&run.harness.html())
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
    PromptOutput::On(fake::output().id)
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
