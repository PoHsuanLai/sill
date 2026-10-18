//! Coherence rules 1 and 2 for every surface, one loop over one table: its own stylesheet is
//! clean under `Profile::Strict`, and the markup it draws in each state worth checking is clean
//! against quire's classes and its own stylesheet. A surface's row names it; a failure names the
//! row and the state.

use ds_harness::Input;
use std::time::Duration;

use ds_lint::{LintConfig, Profile, markup, stylesheet};
use sill_bar::Bar;
use sill_dock::Dock;
use sill_launcher::WindowKey;
use sill_launcher_ui::{Launcher, LauncherCatcher};
use sill_model::auth::UserName;
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
use sill_model::windows::{Activity, Sizing, Toplevel, WindowState};
use sill_notify_ui::{Banners, NotificationCenter, ScreenshotThumb};
use sill_overlays::{
    AccountsSheet, CaptureSheet, ColorSampler, ConfirmSheet, DisplayTrialSheet, ForceQuit,
    HotCorners, IdleDim, Lock, Osd, PermissionSheet, PolkitPrompt, PowerMenu, SpacesBar,
    SplitPicker, Switcher, VoiceConsentSheet, Wallpaper,
};
use sill_settings::{AutoHide, ShellFile};
use sill_ui_kit::harness::{LockHarness, SurfaceHarness, fake};
use sill_ui_kit::links::{
    BannerLink, BannerOutput, CenterLink, ControlCenterLink, Corner, CornerLink, Dir,
    DisplayTrialLink, EditWidgetsLink, EyedropperLink, ForceQuitLink, LauncherLink, LevelSource,
    LockLink, OsdKind, OsdLink, PlacementFile, PowerMenuLink, Preselect, PromptOutput,
    SpacesBarLink, SplitPickerLink, SwitcherAsk, SwitcherLink,
};
use sill_ui_kit::surface::{LockSurface, Surface};
use sill_widgets_ui::{DesktopWidgets, EditWidgets};

/// One surface's row: its stylesheet and a way to draw each state to lint.
struct SurfaceCase {
    name: &'static str,
    /// The surface's name, the `data-surface` of its root: public selector surface.
    surface: &'static str,
    /// The surface's own stylesheet; empty when quire's alone styles it.
    css: &'static str,
    /// The document the surface draws in each state, named: what the harness's `html` says.
    states: fn() -> Vec<(&'static str, String)>,
}

const SURFACES: &[SurfaceCase] = &[
    SurfaceCase {
        name: "bar",
        surface: <Bar as Surface>::NAME,
        css: sill_bar::bar::BAR_CSS,
        states: bar_states,
    },
    SurfaceCase {
        name: "dock",
        surface: <Dock as Surface>::NAME,
        css: sill_dock::dock::DOCK_CSS,
        states: dock_states,
    },
    SurfaceCase {
        name: "launcher",
        surface: <Launcher as Surface>::NAME,
        css: sill_launcher_ui::launcher::LAUNCHER_CSS,
        states: launcher_states,
    },
    SurfaceCase {
        name: "launcher_catcher",
        surface: <LauncherCatcher as Surface>::NAME,
        css: sill_launcher_ui::launcher::LAUNCHER_CSS,
        states: launcher_catcher_states,
    },
    SurfaceCase {
        name: "desktop_widgets",
        surface: <DesktopWidgets as Surface>::NAME,
        css: sill_widgets_ui::desktop::DESKTOP_WIDGETS_CSS,
        states: desktop_widgets_states,
    },
    SurfaceCase {
        name: "edit_widgets",
        surface: <EditWidgets as Surface>::NAME,
        css: sill_widgets_ui::edit::EDIT_WIDGETS_CSS,
        states: edit_widgets_states,
    },
    SurfaceCase {
        name: "osd",
        surface: <Osd as Surface>::NAME,
        css: sill_overlays::osd::OSD_CSS,
        states: osd_states,
    },
    SurfaceCase {
        name: "idle_dim",
        surface: <IdleDim as Surface>::NAME,
        css: "",
        states: idle_dim_states,
    },
    SurfaceCase {
        name: "lock",
        surface: <Lock as LockSurface>::NAME,
        css: sill_overlays::lock::LOCK_CSS,
        states: lock_states,
    },
    SurfaceCase {
        name: "banners",
        surface: <Banners as Surface>::NAME,
        css: sill_notify_ui::banners::BANNERS_CSS,
        states: banners_states,
    },
    SurfaceCase {
        name: "notification_center",
        surface: <NotificationCenter as Surface>::NAME,
        css: sill_notify_ui::notification_center::NOTIFICATION_CENTER_CSS,
        states: notification_center_states,
    },
    SurfaceCase {
        name: "screenshot_thumb",
        surface: <ScreenshotThumb as Surface>::NAME,
        css: sill_notify_ui::screenshot_thumb::SCREENSHOT_THUMB_CSS,
        states: screenshot_thumb_states,
    },
    SurfaceCase {
        name: "hot_corners",
        surface: <HotCorners as Surface>::NAME,
        css: sill_overlays::hot_corners::HOT_CORNERS_CSS,
        states: hot_corners_states,
    },
    SurfaceCase {
        name: "wallpaper",
        surface: <Wallpaper as Surface>::NAME,
        css: sill_overlays::wallpaper::WALLPAPER_CSS,
        states: wallpaper_states,
    },
    SurfaceCase {
        name: "power_menu",
        surface: <PowerMenu as Surface>::NAME,
        css: sill_overlays::power_menu::POWER_MENU_CSS,
        states: power_menu_states,
    },
    SurfaceCase {
        name: "switcher",
        surface: <Switcher as Surface>::NAME,
        css: sill_overlays::switcher::SWITCHER_CSS,
        states: switcher_states,
    },
    SurfaceCase {
        name: "spaces_bar",
        surface: <SpacesBar as Surface>::NAME,
        css: "",
        states: spaces_bar_states,
    },
    SurfaceCase {
        name: "split_picker",
        surface: <SplitPicker as Surface>::NAME,
        css: sill_overlays::split_picker::SPLIT_PICKER_CSS,
        states: split_picker_states,
    },
    SurfaceCase {
        name: "force_quit",
        surface: <ForceQuit as Surface>::NAME,
        css: sill_overlays::force_quit::FORCE_QUIT_CSS,
        states: force_quit_states,
    },
    SurfaceCase {
        name: "polkit_prompt",
        surface: <PolkitPrompt as Surface>::NAME,
        css: "",
        states: polkit_prompt_states,
    },
    SurfaceCase {
        name: "confirm_sheet",
        surface: <ConfirmSheet as Surface>::NAME,
        css: sill_overlays::confirm_sheet::CONFIRM_SHEET_CSS,
        states: confirm_sheet_states,
    },
    SurfaceCase {
        name: "voice_consent",
        surface: <VoiceConsentSheet as Surface>::NAME,
        css: sill_overlays::voice_consent::VOICE_CONSENT_CSS,
        states: voice_consent_states,
    },
    SurfaceCase {
        name: "accounts_sheet",
        surface: <AccountsSheet as Surface>::NAME,
        css: sill_overlays::accounts_sheet::ACCOUNTS_SHEET_CSS,
        states: accounts_sheet_states,
    },
    SurfaceCase {
        name: "capture_sheet",
        surface: <CaptureSheet as Surface>::NAME,
        css: sill_overlays::capture_sheet::CAPTURE_SHEET_CSS,
        states: capture_sheet_states,
    },
    SurfaceCase {
        name: "permission_sheet",
        surface: <PermissionSheet as Surface>::NAME,
        css: "",
        states: permission_sheet_states,
    },
    SurfaceCase {
        name: "display_trial",
        surface: <DisplayTrialSheet as Surface>::NAME,
        css: "",
        states: display_trial_states,
    },
    SurfaceCase {
        name: "eyedropper",
        surface: <ColorSampler as Surface>::NAME,
        css: sill_overlays::eyedropper::EYEDROPPER_CSS,
        states: eyedropper_states,
    },
];

/// The custom properties surfaces write on themselves.
const OWN_VARS: &[&str] = &sill_notify_ui::banners::BANNER_VARS;

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        own_vars: OWN_VARS.iter().map(|var| (*var).to_owned()).collect(),
        ..LintConfig::new(&sill_shell_kit::kits())
    }
}

/// The top left corner's square.
fn hot_corners_states() -> Vec<(&'static str, String)> {
    let mut corner =
        SurfaceHarness::<HotCorners>::setup(CornerLink::new(), Corner::TopLeft).start();
    corner.harness.advance(Duration::ZERO);
    vec![("top_left", corner.harness.html())]
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
            .windows
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

/// The menu hidden, then open on the default action once its sheet has settled.
fn power_menu_states() -> Vec<(&'static str, String)> {
    let mut menu = SurfaceHarness::<PowerMenu>::setup(PowerMenuLink::new(), ()).start();
    menu.feeds.session.send_replace(Session::everything());
    menu.harness.advance(Duration::from_millis(40));
    let hidden = menu.harness.html();
    menu.link.open(fake::output().id, Preselect::Default);
    menu.harness.advance(Duration::from_millis(700));
    let open = menu.harness.html();
    assert!(open.contains("ds-alert"), "{open}");
    vec![("hidden", hidden), ("open", open)]
}

/// The Spaces bar down, then up (the strip and the plus; sill has no Space id of its own to
/// list, the pills are drawn in the overlay crate's tests).
fn spaces_bar_states() -> Vec<(&'static str, String)> {
    let mut bar = SurfaceHarness::<SpacesBar>::setup(SpacesBarLink::new(), ()).start();
    bar.harness.advance(Duration::from_millis(40));
    let down = bar.harness.html();
    bar.link.set_show(sill_model::desktop::BarShow::Show);
    bar.harness.advance(Duration::from_millis(400));
    let up = bar.harness.html();
    assert!(up.contains("ds-spaces-bar"), "{up}");
    vec![("down", down), ("up", up)]
}

/// The picker hidden, then offering two windows for the right half.
fn split_picker_states() -> Vec<(&'static str, String)> {
    let mut picker = SurfaceHarness::<SplitPicker>::setup(SplitPickerLink::new(), ()).start();
    picker.harness.advance(Duration::from_millis(40));
    let hidden = picker.harness.html();
    let card = |window: &str| sill_model::desktop::SplitCard {
        window: window.to_owned(),
        app: "term".to_owned(),
        title: format!("Title of {window}"),
    };
    picker
        .link
        .begin("w1", sill_model::desktop::SplitSide::Left, "");
    picker.link.add(card("w2"));
    picker.link.add(card("w3"));
    picker.harness.advance(Duration::from_millis(200));
    let offered = picker.harness.html();
    assert!(offered.contains("sill-split-scrim"), "{offered}");
    vec![("hidden", hidden), ("offered", offered)]
}

/// The eyedropper with nothing asked, then over a picture of its output with the pointer on it.
fn eyedropper_states() -> Vec<(&'static str, String)> {
    use sill_model::capture_pick::{AskId, EyedropIn};
    use sill_model::desktop::{OutputPicture, OutputShot};
    let mut sampler = SurfaceHarness::<ColorSampler>::setup(EyedropperLink::new(), ()).start();
    sampler.harness.advance(Duration::from_millis(40));
    let hidden = sampler.harness.html();
    let screens = vec!["DP-1".to_owned()];
    let _asked = sampler
        .link
        .take(EyedropIn::Asked(Some((AskId(1), screens))));
    let shot = OutputShot {
        size: (192, 108),
        rgba: vec![200_u8; 192 * 108 * 4].into(),
        png: vec![0x89, b'P'].into(),
    };
    let _held = sampler.link.take(EyedropIn::Picture(
        "DP-1".to_owned(),
        OutputPicture::Shot(shot),
    ));
    sampler.harness.advance(Duration::from_millis(100));
    let at = ds::prelude::Point {
        x: ds::prelude::Px(400.0),
        y: ds::prelude::Px(300.0),
    };
    sampler.harness.send(Input::pointer_move(at));
    sampler.harness.advance(Duration::from_millis(100));
    let pointed = sampler.harness.html();
    assert!(pointed.contains("sill-eyedrop-loupe"), "{pointed}");
    vec![("hidden", hidden), ("pointed", pointed)]
}

/// The panel hidden, then listing two apps (one hung), then asking about the hung one.
fn force_quit_states() -> Vec<(&'static str, String)> {
    use sill_model::desktop::{ForceQuitRow, Standing};

    let mut panel = SurfaceHarness::<ForceQuit>::setup(ForceQuitLink::new(), ()).start();
    panel.harness.advance(Duration::from_millis(40));
    let hidden = panel.harness.html();
    panel.link.set_rows(vec![
        ForceQuitRow {
            app: "mail".to_owned(),
            standing: Standing::Responding,
        },
        ForceQuitRow {
            app: "term".to_owned(),
            standing: Standing::NotResponding,
        },
    ]);
    panel.link.open(fake::output().id);
    panel.harness.advance(Duration::from_millis(600));
    let listed = panel.harness.html();
    assert!(listed.contains("sill-fq"), "{listed}");
    let button = panel
        .harness
        .centre("#force-quit-button")
        .expect("the Force Quit button is laid out"); // test-only
    panel.harness.send(Input::click(button));
    panel.harness.advance(Duration::from_millis(600));
    let asking = panel.harness.html();
    assert!(asking.contains("ds-alert"), "{asking}");
    vec![("hidden", hidden), ("listed", listed), ("asking", asking)]
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
    switcher.feeds.windows.send_modify(|wl| {
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

/// The sheet hidden, then asking a press (arming, then armed), the terminal's offer, and a hold.
/// The screen-sharing picker: hidden, then asking for a screen or a window (both tabs), and for
/// several screens with a choice of pointer.
fn capture_sheet_states() -> Vec<(&'static str, String)> {
    use sill_model::capture_pick::{
        Ask, AskId, Asking, Candidates, CaptureLink, CapturePicker, OutputCandidate,
        WindowCandidate,
    };
    let candidates = Candidates {
        outputs: vec![OutputCandidate {
            name: "DP-1".to_owned(),
            title: "Studio Display".to_owned(),
        }],
        windows: vec![WindowCandidate {
            identifier: "w-1".to_owned(),
            title: "Notes".to_owned(),
            app: "Notes".to_owned(),
        }],
    };
    let asking = |id: u64, kinds: u32, multiple: u32, cursors: &[u32]| CapturePicker {
        link: CaptureLink::Serving,
        candidates: candidates.clone(),
        asking: Some(Asking {
            id: AskId(id),
            ask: Ask::from_wire("Meet", kinds, multiple, cursors).expect("a valid ask"), // test-only
        }),
        ..CapturePicker::default()
    };
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    let mut sheet = SurfaceHarness::<CaptureSheet>::setup(link, ()).start();
    sheet.harness.advance(Duration::from_millis(40));
    let mut states = vec![("hidden", sheet.harness.html())];
    for (name, id, kinds, multiple, cursors) in [
        ("both", 1, 3, 0, &[1][..]),
        ("several_with_pointer", 2, 1, 1, &[1, 2][..]),
    ] {
        sheet
            .feeds
            .capture
            .send_replace(asking(id, kinds, multiple, cursors));
        sheet.harness.advance(Duration::from_millis(700));
        states.push((name, sheet.harness.html()));
        sheet.feeds.capture.send_replace(CapturePicker::default());
        sheet.harness.advance(Duration::from_millis(700));
    }
    assert!(states[1].1.contains("sill-capture"), "{}", states[1].1);
    states
}

/// The permission sheet: hidden, then asking an app's question.
fn permission_sheet_states() -> Vec<(&'static str, String)> {
    use sill_model::capture_pick::{
        AskId, AskedBy, CapturePicker, PermissionAsk, PermissionAsking,
    };
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    let mut sheet = SurfaceHarness::<PermissionSheet>::setup(link, ()).start();
    sheet.harness.advance(Duration::from_millis(40));
    let hidden = sheet.harness.html();
    sheet.feeds.capture.send_replace(CapturePicker {
        permission: Some(PermissionAsking {
            id: AskId(1),
            ask: PermissionAsk::from_wire(
                "Notes",
                "Share your screen?",
                "Notes wants to see it",
                "Only while you share.",
                "",
                "",
            ),
            by: AskedBy::Portal,
        }),
        ..CapturePicker::default()
    });
    sheet.harness.advance(Duration::from_millis(700));
    let shown = sheet.harness.html();
    assert!(shown.contains("ds-alert"), "{shown}");
    vec![("hidden", hidden), ("shown", shown)]
}

/// The display trial: hidden, then asking whether to keep the displays.
fn display_trial_states() -> Vec<(&'static str, String)> {
    use sill_model::desktop::{DisplayTrial, TrialId};
    let banners = BannerLink::new();
    banners.set(BannerOutput::On(fake::output().id));
    let link = DisplayTrialLink::new(banners);
    let mut sheet = SurfaceHarness::<DisplayTrialSheet>::setup(link, ()).start();
    sheet.harness.advance(Duration::from_millis(40));
    let hidden = sheet.harness.html();
    sheet.link.begin(DisplayTrial {
        trial: TrialId::new(1),
        seconds: 15,
    });
    sheet.harness.advance(Duration::from_millis(700));
    let shown = sheet.harness.html();
    assert!(shown.contains("ds-alert"), "{shown}");
    vec![("hidden", hidden), ("shown", shown)]
}

fn confirm_sheet_states() -> Vec<(&'static str, String)> {
    use sill_model::intents::sheet::{Pending, Serial, Sheets};
    use sill_model::intents::{Intents, IntentsLink};
    let wire = |offer: &str, gesture: &str, effect: &str| {
        format!(
            r#"{{"id":"c-1","space":"desktop","actor":{{"kind":"cli"}},"app":"org.quire.Mail",
            "action":"Archive a thread","effect":"{effect}","count":1,"detail":{{"kind":"plain"}},
            "lines":[{{"label":"Thread","value":{{"kind":"plain","v":"Digest"}}}},
                     {{"label":"Subject","value":{{"kind":"quoted","v":{{"text":"Invoice 7","from":{{"kind":"mail"}}}}}}}}],
            "why":[{{"kind":"from_terminal"}}],"taint":{{"kind":"clean"}},"offer":"{offer}",
            "gesture":"{gesture}","anchor":{{"kind":"centre"}},"expires":120}}"#
        )
    };
    let asking = |offer: &str, gesture: &str, effect: &str| Intents {
        link: IntentsLink::Serving,
        sheets: Sheets {
            queue: vec![Pending {
                serial: Serial(1),
                request: serde_json::from_str(&wire(offer, gesture, effect))
                    .unwrap_or_else(|e| panic!("{e}")),
            }],
        },
        ..Intents::default()
    };
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    let mut sheet = SurfaceHarness::<ConfirmSheet>::setup(link, ()).start();
    sheet.harness.advance(Duration::from_millis(40));
    let mut states = vec![("hidden", sheet.harness.html())];
    for (name, offer, gesture, effect, wait) in [
        ("arming", "once_only", "press", "undoable_write", 300),
        ("armed", "once_only", "press", "undoable_write", 700),
        (
            "from_the_terminal",
            "once_or_from_terminal",
            "press",
            "undoable_write",
            700,
        ),
        ("hold", "once_only", "hold_to_confirm", "destructive", 700),
    ] {
        sheet
            .feeds
            .intents
            .send_replace(asking(offer, gesture, effect));
        sheet.harness.advance(Duration::from_millis(wait));
        states.push((name, sheet.harness.html()));
        sheet.feeds.intents.send_replace(Intents::default());
        sheet.harness.advance(Duration::from_millis(700));
    }
    assert!(states[2].1.contains("sill-confirm"), "{}", states[2].1);
    states
}

/// The consent sheet hidden, then asking.
fn voice_consent_states() -> Vec<(&'static str, String)> {
    use sill_model::voice::{ConsentSheet, Voice};
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    let mut sheet = SurfaceHarness::<VoiceConsentSheet>::setup(link, ()).start();
    sheet.harness.advance(Duration::from_millis(40));
    let mut states = vec![("hidden", sheet.harness.html())];
    sheet.feeds.voice.send_replace(Voice {
        sheet: ConsentSheet::Asking,
        ..Voice::default()
    });
    sheet.harness.advance(Duration::from_millis(700));
    states.push(("asking", sheet.harness.html()));
    assert!(states[1].1.contains("sill-consent"), "{}", states[1].1);
    states
}

/// The sheet hidden, then each step accountd can show, and the consent question.
fn accounts_sheet_states() -> Vec<(&'static str, String)> {
    use sill_model::accounts::sheet::{AccountsSheets, SheetEvent, SheetHandle, SheetLink, step};
    const VIEWS: [(&str, &str); 7] = [
        (
            "providers",
            r#"{"kind":"providers","v":[{"id":"nextcloud","label":"Nextcloud","mark":"nextcloud"}]}"#,
        ),
        (
            "sign_in",
            r#"{"kind":"sign_in","v":{"provider":"nextcloud","fields":[
            {"kind":"address","entry":"plain","presence":"required","prefill":null},
            {"kind":"password","entry":"secret","presence":"required","prefill":null}],
            "problem":{"field":"password","problem":"refused"}}}"#,
        ),
        (
            "sign_in_manual",
            r#"{"kind":"sign_in","v":{"provider":"generic-imap","fields":[
            {"kind":"protocol","entry":"plain","presence":"required","prefill":"imap"},
            {"kind":"server","entry":"plain","presence":"required","prefill":"imap.example.org"},
            {"kind":"security","entry":"plain","presence":"required","prefill":"tls"},
            {"kind":"port","entry":"plain","presence":"optional","prefill":"993"},
            {"kind":"outgoing_server","entry":"plain","presence":"required","prefill":"smtp.example.org"},
            {"kind":"outgoing_security","entry":"plain","presence":"required","prefill":"starttls"},
            {"kind":"outgoing_port","entry":"plain","presence":"optional","prefill":"465"},
            {"kind":"username","entry":"plain","presence":"optional","prefill":null}],
            "problem":null}}"#,
        ),
        (
            "browser",
            r#"{"kind":"browser_wait","v":{"provider":"google","url":"https://accounts.example.org/a"}}"#,
        ),
        (
            "working",
            r#"{"kind":"working","v":{"provider":"nextcloud"}}"#,
        ),
        (
            "failed",
            r#"{"kind":"failed","v":{"provider":"nextcloud","fault":"refused"}}"#,
        ),
        (
            "consent",
            r#"{"kind":"consent","v":{"app":{"name":"org.quire.Photos","isolation":"flatpak"},
            "kind":"storage","class":"app_own","usage":"interactive","accounts":[
            {"account":"home","label":"ada@home.example","provider":"nextcloud"}]}}"#,
        ),
    ];
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    let mut sheet = SurfaceHarness::<AccountsSheet>::setup(link, ()).start();
    sheet.harness.advance(Duration::from_millis(40));
    let mut states = vec![("hidden", sheet.harness.html())];
    let handle = SheetHandle::parse("lint").unwrap_or_else(|| panic!("handle"));
    for (name, json) in VIEWS {
        let view = serde_json::from_str(json).unwrap_or_else(|e| panic!("{name}: {e}"));
        let (sheets, _) = step(
            AccountsSheets {
                link: SheetLink::Serving,
                ..AccountsSheets::default()
            },
            SheetEvent::Opened(handle.clone(), view),
        );
        sheet.feeds.accounts_sheet.send_replace(sheets);
        sheet.harness.advance(Duration::from_millis(700));
        states.push((name, sheet.harness.html()));
        sheet
            .feeds
            .accounts_sheet
            .send_replace(AccountsSheets::default());
        sheet.harness.advance(Duration::from_millis(700));
    }
    assert!(
        states[1].1.contains("sill-accounts-window"),
        "{}",
        states[1].1
    );
    states
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
    column.harness.send(Input::pointer_move(plate));
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
    assert!(open.contains("ds-notification"), "{open}");
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
    thumb.harness.send(Input::pointer_move(card));
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
            sill_shell_kit::stylesheet(),
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

#[test]
fn every_surface_root_names_itself_for_the_person_stylesheet() {
    for case in SURFACES {
        let attribute = format!("data-surface=\"{}\"", case.surface);
        for (state, html) in (case.states)() {
            assert!(
                html.matches(&attribute).count() == 1,
                "{} ({state}): one root carries {attribute}: {html}",
                case.name
            );
        }
    }
}

#[test]
fn the_surface_names_are_unique() {
    let mut names: Vec<&str> = SURFACES.iter().map(|case| case.surface).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(names.len(), before, "two surfaces share a name");
}

/// The popups' names, which the rows above do not mount: `crates/sill-ui-kit`'s `BAR_MENU`,
/// `CONTROL_CENTER`, `DOCK_MENU` and `DESKTOP_MENU`.
const POPUP_NAMES: &[&str] = &["bar-menu", "control-center", "desktop-menu", "dock-menu"];

#[test]
fn docs_selectors_lists_every_surface_and_popup_name() {
    let page = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/selectors.md"
    ))
    .expect("docs/selectors.md is in the repository"); // test-only
    let mut missing: Vec<&str> = SURFACES
        .iter()
        .map(|case| case.surface)
        .chain(POPUP_NAMES.iter().copied())
        .filter(|name| !page.contains(&format!("| `{name}` |")))
        .collect();
    missing.sort_unstable();
    assert_eq!(missing, Vec::<&str>::new(), "not in docs/selectors.md");
    let listed = page
        .lines()
        .filter(|line| line.starts_with("| `") && !line.starts_with("| `data-surface`"))
        .count();
    assert_eq!(
        listed,
        SURFACES.len() + POPUP_NAMES.len(),
        "docs/selectors.md lists a name nothing draws"
    );
}
