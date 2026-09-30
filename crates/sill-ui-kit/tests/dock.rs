//! The dock in a `ds_native::Harness` on the fake services: pinned apps then running ones
//! grouped by app, the Trash, running dots, the badge, the ring, the regions' elements, the
//! icon style on each plate; a value fed after the first frame changes what it shows.

use std::time::Duration;

use ds_lint::{LintConfig, Profile, assert_clean};
use ds_settings::{Environment, IconStyle};
use sill_launcher::AppId;
use sill_model::badges::Badges;
use sill_model::cosmic_wl::Toplevels;
use sill_model::pins::DockPins;
use sill_settings::{AutoHide, ShellFile, TrashTile};
use sill_ui_kit::Dock;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::hooks::PinnedAppearance;

type Running = SurfaceHarness<Dock>;

/// The dock on `file` and `appearance`, fed the dock fixtures and drawn.
fn start_with(file: ShellFile, appearance: PinnedAppearance) -> Running {
    let mut run = SurfaceHarness::<Dock>::setup((), ())
        .settings(file)
        .appearance(appearance)
        .start();
    run.feeds.apps.send_replace(fake::dock::index());
    run.feeds.pins.send_replace(fake::dock::pins());
    run.feeds.badges.send_replace(fake::dock::badges());
    run.feeds
        .cosmic_wl
        .send_modify(|wl| wl.toplevels = fake::dock::toplevels_on("DP-1"));
    run.harness.advance(Duration::from_millis(40));
    run
}

fn start(file: ShellFile) -> Running {
    start_with(file, PinnedAppearance::default())
}

/// How many times `needle` occurs in `html`.
fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

/// The hover labels in document order: each tile's name.
fn labels(html: &str) -> Vec<String> {
    html.split("class=\"ds-fly\" role=\"tooltip\">")
        .skip(1)
        .filter_map(|rest| rest.split('<').next().map(str::to_owned))
        .collect()
}

#[test]
fn pinned_then_running_then_trash_with_dots_and_a_badge() {
    let run = start(ShellFile::default());
    let html = run.harness.html();
    assert_eq!(
        labels(&html),
        ["COSMIC Files", "COSMIC Terminal", "Firefox", "Trash"],
        "{html}"
    );
    let cases = [
        ("four tiles", "class=\"sill-dock-tile\"", 4),
        ("terminal and firefox run", "class=\"sill-dock-dot\"", 2),
        (
            "each with quire's running dot",
            "class=\"ds-running-dot\"",
            2,
        ),
        (
            "every tile on a neutral plate",
            "data-family=\"neutral\"",
            4,
        ),
        ("no bare art", "class=\"sill-dock-art\"", 0),
        (
            "the terminal draws its own hicolor icon",
            "CosmicTerm.svg",
            1,
        ),
        ("quire's floor in the pill", "class=\"ds-dock-floor\"", 1),
        ("a squircle pill", "data-corner=\"squircle\"", 1),
        (
            "quire's dock metrics",
            "--dock-gap-px:8px;--dock-pad-px:6px;",
            1,
        ),
        ("the material stack", "--m-vibrancy:1;", 1),
        ("firefox's count", ">7</span>", 1),
        ("one badge", "class=\"sill-dock-badge\"", 1),
        ("firefox's ring", "class=\"sill-dock-ring\"", 1),
        ("the hit element", "id=\"dock-hit\"", 1),
        ("the pill element", "id=\"dock-pill\"", 1),
        (
            "the separator before Trash",
            "class=\"sill-dock-separator\"",
            1,
        ),
        ("no strip while auto-hide is off", "id=\"dock-strip\"", 0),
        ("the Dock material", "data-material=\"dock\"", 1),
        ("the root draws the Space", "data-frame=\"tinted\"", 1),
        ("no tint layer of the dock's own", "sill-dock-tint", 0),
        ("plates on paper", "data-ground=\"frame\"", 0),
        (
            "Files and Terminal draw their own hicolor icons",
            "data-kind=\"image\"",
            2,
        ),
    ];
    for (name, needle, want) in cases {
        let got = count(&html, needle);
        assert_eq!(got, want, "{name}: {needle}\n{html}");
    }
}

#[test]
fn fed_values_replace_the_defaults() {
    let mut run = start(ShellFile::default());
    let before = run.harness.html();
    run.feeds.pins.send_replace(DockPins {
        pinned: vec![AppId(fake::dock::FIREFOX.to_owned())],
    });
    run.feeds
        .cosmic_wl
        .send_modify(|wl| wl.toplevels = Toplevels::default());
    run.feeds.badges.send_replace(Badges::default());
    run.harness.advance(Duration::from_millis(40));
    let after = run.harness.html();
    assert_eq!(labels(&after), ["Firefox", "Trash"]);
    let cases = [
        ("no window, no dot", "class=\"sill-dock-dot\"", 2, 0),
        ("no badge", "class=\"sill-dock-badge\"", 1, 0),
    ];
    for (name, needle, was, now) in cases {
        assert_eq!(
            (count(&before, needle), count(&after, needle)),
            (was, now),
            "{name}"
        );
    }
}

#[test]
fn settings_drop_the_trash_and_add_the_strip() {
    let mut file = ShellFile::default();
    file.dock.trash = TrashTile::Off;
    file.dock.autohide = AutoHide::On;
    let html = start(file).harness.html();
    assert_eq!(
        labels(&html),
        ["COSMIC Files", "COSMIC Terminal", "Firefox"]
    );
    assert_eq!(count(&html, "class=\"sill-dock-separator\""), 0);
    assert_eq!(count(&html, "id=\"dock-strip\""), 1);
}

/// The dock drawn with `style` as the icon style.
fn styled(style: IconStyle) -> String {
    let mut environment = Environment::default();
    environment.settings.icons.style = style;
    start_with(ShellFile::default(), PinnedAppearance(environment))
        .harness
        .html()
}

/// Under every style the terminal's own hicolor icon sits on the neutral plate, never bare.
#[test]
fn a_mapped_app_still_draws_its_own_icon_on_the_plate() {
    for style in [IconStyle::Colour, IconStyle::Muted, IconStyle::Monochrome] {
        let html = styled(style);
        assert!(
            html.contains("data-family=\"neutral\""),
            "{style:?}: {html}"
        );
        assert!(
            html.contains("data-kind=\"image\""),
            "{style:?}: an image, not the window glyph\n{html}"
        );
        assert!(
            !html.contains("sill-dock-art"),
            "{style:?}: no bare art\n{html}"
        );
    }
}

/// Under Muted or Monochrome the plate itself carries the icon style:
/// `IconView`'s `plate_tint` writes `data-icon-style` and the tinted stops and ink as custom
/// properties (`--plate-base-l` ... `--plate-ink-d`), so quire's own stylesheet re-colours the
/// plate the same hue as the icon on it, in both schemes at once.
#[test]
fn the_plate_carries_the_icon_style_under_muted_and_monochrome() {
    for (style, word) in [
        (IconStyle::Muted, "muted"),
        (IconStyle::Monochrome, "monochrome"),
    ] {
        let html = styled(style);
        assert!(
            html.contains(&format!("data-icon-style=\"{word}\"")),
            "{style:?}\n{html}"
        );
        for var in [
            "--plate-base-l:",
            "--plate-deep-l:",
            "--plate-ink-l:",
            "--plate-base-d:",
            "--plate-deep-d:",
            "--plate-ink-d:",
        ] {
            assert!(html.contains(var), "{style:?}: {var}\n{html}");
        }
    }
}

/// The popup frame's stylesheet (the dock's context menu, the bar's menus, the tray's): clean
/// under Strict with no exception, since quire's `RootExtent::Popup` lays the overlay in flow.
#[test]
fn the_popup_frame_stylesheet_is_clean_under_strict() {
    let strict = LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&ds_shell::kits())
    };
    assert_clean(sill_ui_kit::menu::POPUP_FRAME_CSS, &strict);
}
