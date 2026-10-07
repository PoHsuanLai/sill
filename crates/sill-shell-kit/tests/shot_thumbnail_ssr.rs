//! The screenshot thumbnail as markup: the card shown, the card with its actions, a
//! portrait picture pillarboxed in a dark root, and the drag ghost each match their golden under
//! `tests/snapshots/shot_thumbnail/`, lint clean, and use only `ds-` classes the stylesheet
//! styles.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test shot_thumbnail_ssr` rewrites the goldens.

#[path = "support/golden.rs"]
mod golden;

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::components::content::image_source::ImageSize;
use ds::prelude::*;
use ds::root::common::Common;
use ds_lint::{LintConfig, markup};
use sill_shell_kit::notifications::banner_stack::{Banner, BannerKey, BannerPosition};
use sill_shell_kit::notifications::swipe::NotificationSwipe;
use sill_shell_kit::prelude::*;
use sill_shell_kit::thumbs::shot_thumbnail::ThumbAction;

/// A picture's source: a stand-in URI, since only the markup is compared.
fn picture() -> ImageSource {
    ImageSource("data:image/png;base64,AAAA".to_owned())
}

const WIDE: ImageSize = ImageSize {
    width: 1920,
    height: 1080,
};

fn root(theme: Theme, body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance { theme, ..Appearance::default() }, material: Material::Window, stylesheet: Inject::Host,
            {body}
        }
    }
}

fn shown() -> Element {
    root(
        Theme::Light,
        rsx! { ShotThumbnail { image: picture(), size: WIDE, shown: Shown::Visible, common: Common { id: Some("thumb".to_string()), ..Common::default() } } },
    )
}

fn with_actions() -> Element {
    let actions = vec![
        ThumbAction {
            icon: Icon::Trash,
            label: "Delete".into(),
            onpress: EventHandler::new(|()| {}),
        },
        ThumbAction {
            icon: Icon::Copy,
            label: "Copy Text".into(),
            onpress: EventHandler::new(|()| {}),
        },
    ];
    root(
        Theme::Light,
        rsx! { ShotThumbnail { image: picture(), size: WIDE, shown: Shown::Visible, actions, onopen: |()| {} } },
    )
}

fn portrait_dark() -> Element {
    root(
        Theme::Dark,
        rsx! { ShotThumbnail { image: picture(), size: ImageSize { width: 900, height: 1800 }, width: Px(320.0), shown: Shown::Visible } },
    )
}

fn hidden() -> Element {
    root(
        Theme::Light,
        rsx! { ShotThumbnail { image: picture(), size: WIDE, shown: Shown::Hidden } },
    )
}

/// Swipe to dismiss on, inside a `BannerStack` at the bottom right as sill hosts it.
fn in_stack() -> Element {
    let card = rsx! {
        ShotThumbnail {
            image: picture(),
            size: WIDE,
            shown: Shown::Visible,
            swipe: NotificationSwipe::Dismiss(EventHandler::new(|()| {})),
        }
    };
    root(
        Theme::Light,
        rsx! { BannerStack { banners: vec![Banner { key: BannerKey(1), card }], position: BannerPosition::BottomRight } },
    )
}

fn ghost() -> Element {
    root(
        Theme::Light,
        rsx! { ShotGhost { image: picture(), size: WIDE } },
    )
}

/// A specimen: its golden name and how it is made.
type Specimen = (&'static str, fn() -> Element);

const SPECIMENS: &[Specimen] = &[
    ("shown", shown),
    ("with-actions", with_actions),
    ("portrait-dark", portrait_dark),
    ("hidden", hidden),
    ("in-stack", in_stack),
    ("ghost", ghost),
];

fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(make);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn every_specimen_matches_its_golden() {
    let failures: Vec<String> = SPECIMENS
        .iter()
        .filter_map(|(name, make)| {
            golden::check(&format!("shot_thumbnail/{name}.html"), &render(*make)).err()
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_specimen_lints_clean_and_every_class_is_styled() {
    let sheet = sill_shell_kit::stylesheet();
    let mut failures = Vec::new();
    for (name, make) in SPECIMENS {
        let html = render(*make);
        for offence in markup(&html, sheet, &LintConfig::new(&sill_shell_kit::kits())) {
            failures.push(format!("{name}: {:?} {}", offence.rule, offence.text));
        }
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
            if !styled {
                failures.push(format!("{name}: .{class} is not styled"));
            }
        }
    }
    failures.dedup();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The card draws what its props say: entering, its picture letterboxed at its width, and one
/// strip button per action with its label; hidden, nothing is laid out.
#[test]
fn the_markup_carries_the_props() {
    let html = render(with_actions);
    for want in [
        "data-presence=\"entering\"",
        "data-hover=\"off\"",
        "style=\"width:240px\"",
        "style=\"height:138.5px\"",
        "left:4px;top:4px;width:232px;height:130.5px",
        "aria-label=\"Delete\"",
        "aria-label=\"Copy Text\"",
    ] {
        assert!(html.contains(want), "{want} in {html}");
    }
    let gone = render(hidden);
    assert!(gone.contains("data-shown=\"hidden\""), "{gone}");
    assert!(!gone.contains("data-presence"), "{gone}");
    let portrait = render(portrait_dark);
    assert!(
        portrait.contains("left:111.25px;top:4px;width:97.5px;height:195px"),
        "{portrait}"
    );
}
