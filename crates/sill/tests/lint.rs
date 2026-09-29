//! Coherence rules 1 and 2 for every surface, one loop over one table: its own stylesheet is
//! clean under `Profile::Strict`, and the markup it draws in each state worth checking is clean
//! against quire's classes and its own stylesheet. A surface's row names it; a failure names the
//! row and the state.

use std::time::Duration;

use ds::lint::{LintConfig, Profile, markup, stylesheet};
use sill_launcher::WindowKey;
use sill_model::auth::UserName;
use sill_model::cosmic_wl::{Activity, Sizing, Toplevel, WindowState};
use sill_model::idle::{DimLevel, DimShown, Idle, IdleMode, IdlePhase};
use sill_model::ids::{OutputName, WorkspaceKey};
use sill_model::polkit_agent::{
    ActionId, Cookie, Echo, LastTry, Polkit, PromptPhase, PromptView, Registration,
};
use sill_model::session::Session;
use sill_model::wallpaper::{WallpaperImage, Wallpapers};
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{
    BannerLink, BannerOutput, Dir, LevelSource, OsdKind, OsdLink, PowerMenuLink, Preselect,
    SwitcherAsk, SwitcherLink,
};
use sill_ui_kit::{IdleDim, Osd, PolkitPrompt, PowerMenu, Switcher, Wallpaper};

/// One surface's row: its stylesheet and a way to draw each state to lint.
struct SurfaceCase {
    name: &'static str,
    /// The surface's own stylesheet; empty when quire's alone styles it.
    css: &'static str,
    /// The document the surface draws in each state, named: what the harness's `html` says.
    states: fn() -> Vec<(&'static str, String)>,
}

const SURFACES: &[SurfaceCase] = &[
    SurfaceCase {
        name: "osd",
        css: sill_ui_kit::surfaces::osd::OSD_CSS,
        states: osd_states,
    },
    SurfaceCase {
        name: "idle_dim",
        css: "",
        states: idle_dim_states,
    },
    SurfaceCase {
        name: "hot_corners",
        css: sill_ui_kit::surfaces::hot_corners::HOT_CORNERS_CSS,
        states: no_states,
    },
    SurfaceCase {
        name: "wallpaper",
        css: sill_ui_kit::surfaces::wallpaper::WALLPAPER_CSS,
        states: wallpaper_states,
    },
    SurfaceCase {
        name: "power_menu",
        css: sill_ui_kit::surfaces::power_menu::POWER_MENU_CSS,
        states: power_menu_states,
    },
    SurfaceCase {
        name: "switcher",
        css: sill_ui_kit::surfaces::switcher::SWITCHER_CSS,
        states: switcher_states,
    },
    SurfaceCase {
        name: "polkit_prompt",
        css: "",
        states: polkit_prompt_states,
    },
];

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&ds::kits())
    }
}

/// A surface with no markup of its own to lint: a bare element the stylesheet sizes.
fn no_states() -> Vec<(&'static str, String)> {
    Vec::new()
}

/// The indicator hidden, then shown for a volume change.
fn osd_states() -> Vec<(&'static str, String)> {
    let mut osd = SurfaceHarness::<Osd>::setup(OsdLink::new(), ()).start();
    osd.harness.advance(Duration::from_millis(40));
    let hidden = osd.harness.html();
    assert!(hidden.contains("data-shown=\"hidden\""), "{hidden}");
    osd.link
        .show(OsdKind::Volume, vec![fake::output().id], LevelSource::Live);
    osd.harness.advance(Duration::from_millis(60));
    let shown = osd.harness.html();
    assert!(shown.contains("data-presence=\"entering\""), "{shown}");
    vec![("hidden", hidden), ("shown", shown)]
}

/// The scrim awake, then dimmed and settled.
fn idle_dim_states() -> Vec<(&'static str, String)> {
    let mut dim = SurfaceHarness::<IdleDim>::setup((), ()).start();
    dim.harness.advance(Duration::ZERO);
    let awake = dim.harness.html();
    dim.feeds.idle.send_replace(Idle {
        mode: IdleMode::Own,
        phase: IdlePhase::Dimmed,
        dim: DimShown::Dimmed(DimLevel(30)),
        ..Idle::default()
    });
    dim.harness.advance(Duration::from_millis(2_500));
    let dimmed = dim.harness.html();
    assert!(dimmed.contains("opacity:0.7"), "{dimmed}");
    vec![("awake", awake), ("dimmed", dimmed)]
}

/// The wallpaper with no image, then with this output's image.
fn wallpaper_states() -> Vec<(&'static str, String)> {
    let mut wallpaper = SurfaceHarness::<Wallpaper>::setup((), ()).start();
    wallpaper.harness.advance(Duration::ZERO);
    let bare = wallpaper.harness.html();
    wallpaper.feeds.wallpaper.send_replace(Wallpapers {
        outputs: vec![WallpaperImage {
            output: OutputName("DP-1".to_owned()),
            light: "/tmp/sill-wallpaper-lint/light.png".into(),
            dark: "/tmp/sill-wallpaper-lint/dark.png".into(),
        }],
    });
    wallpaper.harness.advance(Duration::ZERO);
    let pictured = wallpaper.harness.html();
    assert!(pictured.contains("<img"), "{pictured}");
    vec![("no image", bare), ("with an image", pictured)]
}

/// The menu hidden, then open on the default button once its sheet has settled.
fn power_menu_states() -> Vec<(&'static str, String)> {
    let mut menu = SurfaceHarness::<PowerMenu>::setup(PowerMenuLink::new(), ()).start();
    menu.feeds.session.send_replace(Session::everything());
    menu.harness.advance(Duration::from_millis(40));
    let hidden = menu.harness.html();
    menu.link.open(fake::output().id, Preselect::Default);
    menu.harness.advance(Duration::from_millis(700));
    let open = menu.harness.html();
    assert!(open.contains("sill-power-menu-panel"), "{open}");
    vec![("hidden", hidden), ("open", open)]
}

/// The switcher hidden, then shown on three apps past the show delay.
fn switcher_states() -> Vec<(&'static str, String)> {
    let mut switcher = SurfaceHarness::<Switcher>::setup(SwitcherLink::new(), ()).start();
    let window = |key: u64, app: &str| Toplevel {
        key: WindowKey(key),
        app_id: app.to_owned(),
        title: format!("{app} {key}"),
        activity: Activity::Inactive,
        window: WindowState::Normal,
        sizing: Sizing::Floating,
        workspaces: vec![WorkspaceKey(2)],
        outputs: vec![OutputName("DP-1".to_owned())],
    };
    switcher.feeds.cosmic_wl.send_modify(|wl| {
        wl.toplevels.windows = vec![window(1, "term"), window(2, "calc"), window(3, "edit")];
    });
    switcher.harness.advance(Duration::from_millis(40));
    let hidden = switcher.harness.html();
    switcher
        .link
        .ask(fake::output().id, SwitcherAsk::Chord(Dir::Next));
    switcher.harness.advance(Duration::from_millis(260));
    assert_eq!(switcher.harness.count(".ds-switcher"), 1);
    vec![("hidden", hidden), ("shown", switcher.harness.html())]
}

/// The prompt hidden, then showing a request for a password once its sheet has settled.
fn polkit_prompt_states() -> Vec<(&'static str, String)> {
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    let mut prompt = SurfaceHarness::<PolkitPrompt>::setup(link, ()).start();
    prompt.harness.advance(Duration::from_millis(40));
    let hidden = prompt.harness.html();
    prompt.feeds.polkit.send_replace(Polkit {
        registration: Registration::Registered,
        current: Some(PromptView {
            cookie: Cookie("c1".to_owned()),
            action_id: ActionId("org.freedesktop.systemd1.manage-units".to_owned()),
            message: "Authentication is required to start 'sshd.service'.".to_owned(),
            icon: String::new(),
            details: vec![("unit".to_owned(), "sshd.service".to_owned())],
            user: UserName("ada".to_owned()),
            phase: PromptPhase::Asking {
                prompt: "Password: ".to_owned(),
                echo: Echo::Hidden,
            },
            last: LastTry::None,
            notes: Vec::new(),
        }),
        waiting: 0,
    });
    prompt.harness.advance(Duration::from_millis(700));
    let shown = prompt.harness.html();
    assert!(shown.contains("ds-polkit"), "{shown}");
    vec![("hidden", hidden), ("shown", shown)]
}

#[test]
fn every_surface_stylesheet_is_clean_under_strict() {
    for case in SURFACES {
        let offences = stylesheet(case.css, &strict());
        assert!(offences.is_empty(), "{}: {offences:#?}", case.name);
    }
}

#[test]
fn every_surface_draws_clean_markup_in_each_state() {
    for case in SURFACES {
        let css = format!("{}\n{}", ds::stylesheet(), case.css);
        for (state, html) in (case.states)() {
            let offences = markup(&html, &css, &strict());
            assert!(
                offences.is_empty(),
                "{} ({state}): {offences:#?}",
                case.name
            );
        }
    }
}
