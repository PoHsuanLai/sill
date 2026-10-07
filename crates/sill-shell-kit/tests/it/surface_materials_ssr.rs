//! The materials as markup (FINDINGS "Materials, blur and colour"): a plate, a squircle surface and
//! root, the bar item, the workspace pills, the dock's dot and floor, the palette's squircle
//! card. Each golden is `tests/snapshots/materials/<name>.html`, and every `ds-` class in it must
//! be styled by the stylesheet.
//!
//! `DS_BLESS=1 cargo test -p sill-shell-kit --test it surface_materials_ssr` rewrites the goldens.

use crate::support::golden;

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::base::press::Press;
use ds::base::vocab::Activity;
use ds::components::controls::badge::BadgeContent;
use ds::components::controls::button_model::Bezel;
use ds::components::menus::palette::palette_group::PaletteGroups;
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::icon::family::PlateFamily;
use ds::style::icon::plate_tint::PlateTint;
use ds::style::icon::retint::IconStyle;
use ds::style::icon::retint::Tint;
use ds::style::space::presets::PRESETS;
use ds::style::tokens::shape::Corner;
use sill_shell_kit::prelude::*;

/// The Work Space's Monochrome plate tint.
fn work() -> Option<PlateTint> {
    PlateTint::of(IconStyle::Monochrome, Tint::space(PRESETS[0].dots))
}

/// A root in `theme`, for the tinted plates' two schemes.
fn scheme(theme: Theme) -> Appearance {
    Appearance {
        theme,
        ..Appearance::default()
    }
}

#[derive(Props, Clone)]
struct HostProps {
    make: fn() -> Element,
}

impl PartialEq for HostProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

fn host(props: HostProps) -> Element {
    (props.make)()
}

fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new_with_props(host, HostProps { make });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// A specimen's golden name and how it renders.
type Case = (&'static str, fn() -> Element);

const CASES: &[Case] = &[
    ("plate-blue", || {
        rsx! {
            IconView { source: IconSource::Glyph(Icon::Folder), size: IconSize::Tile48, plate: Some(PlateFamily::Blue) }
        }
    }),
    ("plate-neutral-symbolic", || {
        rsx! {
            IconView { source: IconSource::Glyph(Icon::Window), size: IconSize::Tile96, plate: Some(PlateFamily::Neutral) }
        }
    }),
    // Both schemes' stops are written on the plate; the root's data-theme picks them.
    ("plate-neutral-monochrome-light", || {
        rsx! {
            Ds { appearance: scheme(Theme::Light), material: Material::Dock, stylesheet: Inject::Host,
                IconView { source: IconSource::Glyph(Icon::Window), size: IconSize::Tile48, plate: Some(PlateFamily::Neutral), plate_tint: work() }
            }
        }
    }),
    ("plate-neutral-monochrome-dark", || {
        rsx! {
            Ds { appearance: scheme(Theme::Dark), material: Material::Dock, stylesheet: Inject::Host,
                IconView { source: IconSource::Glyph(Icon::Window), size: IconSize::Tile48, plate: Some(PlateFamily::Neutral), plate_tint: work() }
            }
        }
    }),
    ("plate-blue-muted", || {
        rsx! {
            IconView { source: IconSource::Glyph(Icon::Folder), size: IconSize::Tile48, plate: Some(PlateFamily::Blue), plate_tint: Some(PlateTint::Muted) }
        }
    }),
    ("surface-squircle", || {
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Window, stylesheet: Inject::Host,
                Surface { material: Material::Sheet, radius: Some(Corner::Squircle(Px(14.0))), p { "inside" } }
            }
        }
    }),
    ("root-dock-squircle", || {
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Dock, stylesheet: Inject::Host, radius: Some(Corner::Squircle(Px(18.0))),
                DockFloor {}
                div { style: "position:relative", RunningDot {} }
            }
        }
    }),
    ("dock-tile", || {
        rsx! {
            DockTile { icon: IconSource::Glyph(Icon::Folder), plate: Some(PlateFamily::Blue), label: "Files", onclick: |_| {} }
        }
    }),
    ("dock-tile-running-badge-label", || {
        rsx! {
            DockTile {
                icon: IconSource::Glyph(Icon::Mail),
                plate: Some(PlateFamily::Blue),
                running: Activity::Active,
                badge: Some(BadgeContent::Number(3)),
                label: "Mail",
                label_shown: Some(Shown::Visible),
                onclick: |_| {},
            }
        }
    }),
    ("dock-tile-progress-magnified", || {
        rsx! {
            DockTile {
                icon: IconSource::Glyph(Icon::Download),
                plate: Some(PlateFamily::Green),
                progress: Some(Fraction(400)),
                side: Px(80.0),
                label: "Downloads",
                onclick: |_| {},
            }
        }
    }),
    ("dock-tile-muted-dot-badge", || {
        rsx! {
            DockTile {
                icon: IconSource::Glyph(Icon::Folder),
                plate: Some(PlateFamily::Neutral),
                plate_tint: Some(PlateTint::Muted),
                badge: Some(BadgeContent::Dot),
                label: "Files",
                onclick: |_| {},
            }
        }
    }),
    ("dock-tile-bounce", || {
        rsx! {
            DockTile {
                icon: IconSource::Glyph(Icon::Mail),
                plate: Some(PlateFamily::Blue),
                running: Activity::Active,
                lift: Px(12.0),
                label: "Mail",
                onclick: |_| {},
            }
        }
    }),
    ("pane-header-back-and-switch", || {
        rsx! {
            PaneHeader {
                title: "Wi-Fi",
                back: Some(EventHandler::new(|_: Press| {})),
                trailing: rsx! { span { "switch" } },
            }
        }
    }),
    ("pane-header-plain", || {
        rsx! {
            PaneHeader { title: "Bluetooth" }
        }
    }),
    ("pane-footer", || {
        rsx! {
            PaneFooter {
                Button { bezel: Bezel::Inline, label: "Wi-Fi Settings…", onclick: |_| {} }
            }
        }
    }),
    ("bar-items", || {
        rsx! {
            MenuBarItem { emphasis: Emphasis::Strong, label: "Files", onclick: |_| {} }
            MenuBarItem { shown: Shown::Visible, common: Common { id: Some("file".to_string()), ..Common::default() }, label: "File", onclick: |_| {} }
            MenuBarItem { label: "Edit", onclick: |_| {} }
        }
    }),
    ("workspace-pills", || {
        rsx! {
            WorkspacePills { label: "Workspaces",
                WorkspacePill { label: "1", current: Selection::Selected, onclick: |_| {} }
                WorkspacePill { label: "2", onclick: |_| {} }
            }
        }
    }),
    ("palette-squircle", || {
        rsx! {
            Ds { appearance: Appearance::default(), material: Material::Sheet, stylesheet: Inject::Host,
            CommandPalette::<u8> {
                label: "Launch",
                placeholder: "Search",
                query: "",
                tokens: Vec::new(),
                groups: PaletteGroups::default(),
                empty: "Nothing",
                oninput: |_| {},
                onpick: |_| {},
                onclose: |_| {},
                host: CommandPaletteHost::Surface,
                corner: Some(Corner::Squircle(Px(14.0))),
            }
            }
        }
    }),
];

#[test]
fn every_material_specimen_matches_its_golden() {
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|(name, make)| {
            golden::check(&format!("materials/{name}.html"), &render(*make)).err()
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_class_is_styled() {
    let sheet = sill_shell_kit::stylesheet();
    for (name, make) in CASES {
        let html = render(*make);
        for class in html
            .split("class=\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .flat_map(str::split_whitespace)
            .filter(|class| class.starts_with("ds-"))
        {
            let needle = format!(".{class}");
            let styled = sheet.match_indices(&needle).any(|(at, _)| {
                !sheet[at + needle.len()..]
                    .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            });
            assert!(styled, "{name}: .{class} is not styled");
        }
    }
}
