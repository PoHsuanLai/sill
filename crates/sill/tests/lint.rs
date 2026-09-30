//! Coherence rules 1 and 2 for every surface, one loop over one table: its own stylesheet is
//! clean under `Profile::Strict`, and the markup it draws in each state worth checking is clean
//! against quire's classes and its own stylesheet. A surface's row names it; a failure names the
//! row and the state.

use std::time::Duration;

use ds_lint::{LintConfig, Profile, markup, stylesheet};
use sill_dock::Dock;
use sill_launcher::WindowKey;
use sill_model::auth::UserName;
use sill_model::cosmic_wl::{Activity, Sizing, Toplevel, WindowState};
use sill_model::idle::{DimLevel, DimShown, Idle, IdleMode, IdlePhase};
use sill_model::ids::{OutputName, WorkspaceKey};
use sill_model::notifications::{
    Action, ActionKey, AppGroup, AppKey, AppRef, BannerLife, Body, Expiry, Hints, History, IconRef,
    LiveBanner, Notification, NotificationId, Notifications, UnixMs, parse_body,
};
use sill_model::polkit_agent::{
    ActionId, Cookie, Echo, LastTry, Polkit, PromptPhase, PromptView, Registration,
};
use sill_model::screenshots::{Shot, ShotId, ShotPreview, Shots};
use sill_model::session::Session;
use sill_model::wallpaper::{WallpaperImage, Wallpapers};
use sill_notify_ui::{Banners, NotificationCenter, ScreenshotThumb};
use sill_overlays::{IdleDim, Lock, Osd, PolkitPrompt, PowerMenu, Switcher, Wallpaper};
use sill_settings::{AutoHide, ShellFile};
use sill_ui_kit::harness::{LockHarness, SurfaceHarness, fake};
use sill_ui_kit::links::{
    BannerLink, BannerOutput, CenterLink, ControlCenterLink, Dir, EditWidgetsLink, LauncherLink,
    LevelSource, LockLink, OsdKind, OsdLink, PlacementFile, PowerMenuLink, Preselect, PromptOutput,
    SwitcherAsk, SwitcherLink,
};
use sill_ui_kit::{Bar, Launcher, LauncherCatcher};
use sill_widgets_ui::{DesktopWidgets, EditWidgets};

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
        name: "bar",
        css: sill_ui_kit::surfaces::bar::BAR_CSS,
        states: bar_states,
    },
    SurfaceCase {
        name: "dock",
        css: sill_dock::dock::DOCK_CSS,
        states: dock_states,
    },
    SurfaceCase {
        name: "launcher",
        css: sill_ui_kit::surfaces::launcher::LAUNCHER_CSS,
        states: launcher_states,
    },
    SurfaceCase {
        name: "launcher_catcher",
        css: sill_ui_kit::surfaces::launcher::LAUNCHER_CSS,
        states: launcher_catcher_states,
    },
    SurfaceCase {
        name: "desktop_widgets",
        css: sill_widgets_ui::desktop::DESKTOP_WIDGETS_CSS,
        states: desktop_widgets_states,
    },
    SurfaceCase {
        name: "edit_widgets",
        css: sill_widgets_ui::edit::EDIT_WIDGETS_CSS,
        states: edit_widgets_states,
    },
    SurfaceCase {
        name: "osd",
        css: sill_overlays::osd::OSD_CSS,
        states: osd_states,
    },
    SurfaceCase {
        name: "idle_dim",
        css: "",
        states: idle_dim_states,
    },
    SurfaceCase {
        name: "lock",
        css: sill_overlays::lock::LOCK_CSS,
        states: lock_states,
    },
    SurfaceCase {
        name: "banners",
        css: sill_notify_ui::banners::BANNERS_CSS,
        states: banners_states,
    },
    SurfaceCase {
        name: "notification_center",
        css: sill_notify_ui::notification_center::NOTIFICATION_CENTER_CSS,
        states: notification_center_states,
    },
    SurfaceCase {
        name: "screenshot_thumb",
        css: sill_notify_ui::screenshot_thumb::SCREENSHOT_THUMB_CSS,
        states: screenshot_thumb_states,
    },
    SurfaceCase {
        name: "hot_corners",
        css: sill_overlays::hot_corners::HOT_CORNERS_CSS,
        states: no_states,
    },
    SurfaceCase {
        name: "wallpaper",
        css: sill_overlays::wallpaper::WALLPAPER_CSS,
        states: wallpaper_states,
    },
    SurfaceCase {
        name: "power_menu",
        css: sill_overlays::power_menu::POWER_MENU_CSS,
        states: power_menu_states,
    },
    SurfaceCase {
        name: "switcher",
        css: sill_overlays::switcher::SWITCHER_CSS,
        states: switcher_states,
    },
    SurfaceCase {
        name: "polkit_prompt",
        css: "",
        states: polkit_prompt_states,
    },
];

/// The custom properties surfaces write on themselves.
const OWN_VARS: &[&str] = &sill_notify_ui::banners::BANNER_VARS;

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        own_vars: OWN_VARS.iter().map(|var| (*var).to_owned()).collect(),
        ..LintConfig::new(&ds_shell::kits())
    }
}

/// A surface with no markup of its own to lint: a bare element the stylesheet sizes.
fn no_states() -> Vec<(&'static str, String)> {
    Vec::new()
}

/// The bar with its default items, settled.
fn bar_states() -> Vec<(&'static str, String)> {
    let mut bar = SurfaceHarness::<Bar>::setup(ControlCenterLink::new(), ()).start();
    bar.harness.advance(Duration::ZERO);
    vec![("idle", bar.harness.html())]
}

/// The dock on the dock fixtures, pinned, running and badged; then hiding, with its strip.
fn dock_states() -> Vec<(&'static str, String)> {
    let draw = |file: ShellFile| {
        let mut dock = SurfaceHarness::<Dock>::setup((), ()).settings(file).start();
        dock.feeds.apps.send_replace(fake::dock::index());
        dock.feeds.pins.send_replace(fake::dock::pins());
        dock.feeds.badges.send_replace(fake::dock::badges());
        dock.feeds
            .cosmic_wl
            .send_modify(|wl| wl.toplevels = fake::dock::toplevels_on("DP-1"));
        dock.harness.advance(Duration::from_millis(40));
        dock.harness.html()
    };
    let mut hiding = ShellFile::default();
    hiding.dock.autohide = AutoHide::On;
    let fixed = draw(ShellFile::default());
    let hidden = draw(hiding);
    assert!(hidden.contains("id=\"dock-strip\""), "{hidden}");
    vec![("fixed", fixed), ("hiding", hidden)]
}

/// The panel shut, then opened on the empty query (results over fake providers are
/// `sill-ui-kit`'s `launcher_lint`).
fn launcher_states() -> Vec<(&'static str, String)> {
    let mut panel = SurfaceHarness::<Launcher>::setup(LauncherLink::new(), ()).start();
    panel.harness.advance(Duration::from_millis(40));
    let shut = panel.harness.html();
    panel.link.open(fake::output().id, String::new());
    panel.harness.advance(Duration::from_millis(60));
    let open = panel.harness.html();
    assert!(open.contains("class=\"ds-palette\""), "{open}");
    vec![("shut", shut), ("open", open)]
}

/// The catcher shut, then shown with the launcher.
fn launcher_catcher_states() -> Vec<(&'static str, String)> {
    let mut catcher = SurfaceHarness::<LauncherCatcher>::setup(LauncherLink::new(), ()).start();
    catcher.harness.advance(Duration::from_millis(40));
    let shut = catcher.harness.html();
    catcher.link.open(fake::output().id, String::new());
    catcher.harness.advance(Duration::from_millis(60));
    let shown = catcher.harness.html();
    assert!(shown.contains("sill-launcher-catcher"), "{shown}");
    vec![("shut", shut), ("shown", shown)]
}

/// The desktop layer with its default widgets placed, its places kept in a scratch file.
fn desktop_widgets_states() -> Vec<(&'static str, String)> {
    let dir =
        std::env::temp_dir().join(format!("sill-lint-desktop-widgets-{}", std::process::id()));
    let places = PlacementFile(dir.join("desktop-widgets.json"));
    let mut layer = SurfaceHarness::<DesktopWidgets>::setup(places, ()).start();
    for _ in 0..10 {
        layer.harness.advance(Duration::from_millis(40));
    }
    let placed = layer.harness.html();
    let _ = std::fs::remove_dir_all(dir);
    assert!(placed.contains("widget-battery"), "{placed}");
    vec![("placed", placed)]
}

/// The gallery hidden, then open.
fn edit_widgets_states() -> Vec<(&'static str, String)> {
    let mut edit = SurfaceHarness::<EditWidgets>::setup(EditWidgetsLink::new(), ()).start();
    edit.harness.advance(Duration::from_millis(60));
    let hidden = edit.harness.html();
    edit.link.open(fake::output().id);
    for _ in 0..10 {
        edit.harness.advance(Duration::from_millis(40));
    }
    let open = edit.harness.html();
    assert!(open.contains("ds-widget-gallery"), "{open}");
    vec![("hidden", hidden), ("open", open)]
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

/// The idle column, then a grouped card, hovered, with two actions and a body with a link.
fn banners_states() -> Vec<(&'static str, String)> {
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    let mut column = SurfaceHarness::<Banners>::setup(link, ()).start();
    column.harness.advance(Duration::from_millis(60));
    let idle = column.harness.html();
    let note = |id: u32, summary: &str, actions: &[(&str, &str)]| Notification {
        id: NotificationId(id),
        arrived: UnixMs(sill_ui_kit::notification_card::wall_now().0),
        app: AppRef {
            name: "mail".to_owned(),
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary: summary.to_owned(),
        body: Body::plain(""),
        actions: actions
            .iter()
            .map(|(key, label)| Action {
                key: ActionKey((*key).to_owned()),
                label: (*label).to_owned(),
            })
            .collect(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    };
    let live = |note| LiveBanner {
        note,
        life: BannerLife::Expires,
    };
    let mut newest = note(2, "Two", &[("reply", "Reply"), ("archive", "Archive")]);
    newest.body = parse_body("<b>Body</b> with <a href=\"https://x.org\">a link</a>");
    column.feeds.notifications.send_modify(|n| {
        n.live = Notifications {
            banners: vec![live(newest), live(note(1, "One", &[]))],
            ..Notifications::default()
        }
    });
    column.harness.advance(Duration::from_millis(700));
    let plate = column
        .harness
        .centre(".ds-notification-plate")
        .expect("a card"); // test-only
    column.harness.pointer_move(plate);
    column.harness.advance(Duration::from_millis(60));
    let hovered = column.harness.html();
    assert!(hovered.contains("data-hover=\"on\""), "{hovered}");
    vec![("idle", idle), ("hovered", hovered)]
}

/// The center closed, then open over a history of two apps.
fn notification_center_states() -> Vec<(&'static str, String)> {
    let note = |id: u32, app: &str, summary: &str| Notification {
        id: NotificationId(id),
        arrived: UnixMs(1_000 * i64::from(id)),
        app: AppRef {
            name: app.to_owned(),
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary: summary.to_owned(),
        body: Body::default(),
        actions: Vec::new(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    };
    let mut center = SurfaceHarness::<NotificationCenter>::setup(CenterLink::new(), ()).start();
    center.feeds.notifications.send_modify(|n| {
        n.history = History {
            groups: vec![
                AppGroup {
                    key: AppKey("mail".to_owned()),
                    notes: vec![note(3, "Mail", "Re: plans"), note(1, "Mail", "Hello")],
                },
                AppGroup {
                    key: AppKey("chat".to_owned()),
                    notes: vec![note(4, "Chat", "ping")],
                },
            ],
        }
    });
    center.harness.advance(Duration::from_millis(40));
    let closed = center.harness.html();
    center.link.open(fake::output().id);
    center.harness.advance(Duration::from_millis(700));
    let open = center.harness.html();
    assert!(open.contains("sill-nc-row"), "{open}");
    vec![("closed", closed), ("open", open)]
}

/// The thumbnail with no shot, then a shown card with the pointer on it.
fn screenshot_thumb_states() -> Vec<(&'static str, String)> {
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    let mut thumb = SurfaceHarness::<ScreenshotThumb>::setup(link, ()).start();
    thumb.harness.advance(Duration::from_millis(60));
    let idle = thumb.harness.html();
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../sill-services/tests/fixtures/shots/Screenshot_fixture.png"
    );
    thumb.feeds.shots.send_replace(Shots {
        current: Some(Shot {
            id: ShotId(1),
            path: "/tmp/shots/Screenshot_1.png".into(),
            preview: ShotPreview::Ready(fixture.into()),
        }),
        last_id: ShotId(1),
        ..Shots::default()
    });
    thumb.harness.advance(Duration::from_millis(1_300));
    let card = thumb.harness.centre(".ds-shot-picture").expect("a card"); // test-only
    thumb.harness.pointer_move(card);
    thumb.harness.advance(Duration::from_millis(300));
    let shown = thumb.harness.html();
    assert!(shown.contains("ds-shot-image"), "{shown}");
    vec![("idle", idle), ("shown", shown)]
}

/// The lock screen with the prompt on its output, then on another one (the clock alone).
fn lock_states() -> Vec<(&'static str, String)> {
    let draw = |prompt| {
        let link = LockLink::new();
        link.set(prompt);
        let mut lock = LockHarness::<Lock>::setup(link).start();
        lock.harness.advance(Duration::ZERO);
        lock.harness.html()
    };
    // The document carries quire's stylesheet, which names every class; the markup follows it.
    let drawn = |html: &str| {
        html.rsplit_once("</style>")
            .map_or(0, |(_, markup)| markup.matches("ds-lock-prompt").count())
    };
    let prompt = draw(PromptOutput::On(fake::output().id));
    assert!(drawn(&prompt) > 0, "{prompt}");
    let clock = draw(PromptOutput::Nowhere);
    assert_eq!(drawn(&clock), 0, "{clock}");
    vec![("prompt", prompt), ("clock", clock)]
}

/// What a surface injects beside its own sheet: the widgets' sheets under the center's history.
fn injected_css(name: &str) -> String {
    match name {
        "notification_center" => sill_ui_kit::widgets::widgets_css(),
        _ => String::new(),
    }
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
        let css = format!(
            "{}\n{}\n{}",
            ds_shell::stylesheet(),
            case.css,
            injected_css(case.name)
        );
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

#[test]
fn the_banner_stack_names_no_motion_of_its_own() {
    let banners = SURFACES
        .iter()
        .find(|case| case.name == "banners")
        .expect("a banners row");
    for motion in ["@keyframes", "transition", "animation"] {
        assert!(
            !banners.css.contains(motion),
            "{motion}: the stack's motion is quire's"
        );
    }
}
