//! Animated emoji as markup (design/25-EMOJI.md): a pick at each size, with and
//! without its disc, in both schemes, match goldens under `tests/snapshots/emoji/`, lint clean,
//! and use only `ds-` classes the stylesheet styles. The sheet's `data:` URI is replaced in the
//! goldens by its length, so a golden stays readable and still notices a different sheet.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test it emoji_ssr` rewrites the goldens.

use crate::support::specimens::Specimen;

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::prelude::*;
use sill_shell_kit::emoji::disc::{DiscHue, EmojiDisc};
use sill_shell_kit::emoji::id::EmojiId;
use sill_shell_kit::prelude::*;
use sill_shell_kit::user_picture::size::PictureSize;

#[derive(Props, Clone, PartialEq)]
struct SpecimenProps {
    emoji: EmojiId,
    size: PictureSize,
    disc: EmojiDisc,
    theme: Theme,
}

#[allow(non_snake_case)]
fn Specimen(props: SpecimenProps) -> Element {
    rsx! {
        Ds { appearance: Appearance { theme: props.theme, ..Appearance::default() }, material: Material::Window, stylesheet: Inject::Host,
            AnimatedEmoji { emoji: props.emoji, size: props.size, disc: props.disc }
        }
    }
}

fn render(props: SpecimenProps) -> String {
    let mut dom = VirtualDom::new_with_props(Specimen, props);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// `html` with every `data:` URI replaced by its length.
fn redacted(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(at) = rest.find("data:image/png;base64,") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail.find(['"', '&', ')']).unwrap_or(tail.len());
        out.push_str(&format!("data:image/png;base64,({} chars)", end));
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

fn large(emoji: EmojiId) -> SpecimenProps {
    SpecimenProps {
        emoji,
        size: PictureSize::Large,
        disc: EmojiDisc::None,
        theme: Theme::Light,
    }
}

fn cases() -> Vec<(String, SpecimenProps)> {
    let mut all: Vec<(String, SpecimenProps)> =
        vec![("default-large".into(), large(EmojiId::default()))];
    all.push((
        "wink-small".into(),
        SpecimenProps {
            size: PictureSize::Small,
            ..large(EmojiId::Wink)
        },
    ));
    all.push((
        "heart-eyes-medium-disc".into(),
        SpecimenProps {
            size: PictureSize::Medium,
            disc: EmojiDisc::Tinted(DiscHue::Teal),
            ..large(EmojiId::HeartEyes)
        },
    ));
    all.push((
        "fox-disc-dark".into(),
        SpecimenProps {
            disc: EmojiDisc::Tinted(DiscHue::Plum),
            theme: Theme::Dark,
            ..large(EmojiId::Fox)
        },
    ));
    all
}

pub(crate) fn specimens() -> Vec<Specimen> {
    cases()
        .into_iter()
        .map(|(name, props)| {
            let html = render(props);
            let stored = redacted(&html);
            Specimen::of(&format!("emoji/{name}.html"), html).stored_as(stored)
        })
        .collect()
}

/// The first render, before any timer, draws the pick at its rest frame.
#[test]
fn a_pick_is_drawn_at_its_rest_frame() {
    let html = render(large(EmojiId::Wink));
    assert!(html.contains("data-emoji=\"wink\""), "{}", redacted(&html));
    assert!(html.contains("data-frame=\"0\""), "{}", redacted(&html));
    assert!(
        html.contains("background-position:0% 0%"),
        "{}",
        redacted(&html)
    );
}
