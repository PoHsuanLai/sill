//! The user's picture picker as markup (design/25-EMOJI.md section 7): the picker matches its golden under `tests/snapshots/user_picture/`, lint clean, and use
//! only `ds-` classes the stylesheet styles. A sheet's `data:` URI is replaced in the goldens by
//! its length, so a golden stays readable and still notices a different sheet. The stored
//! choice round-trips through serde by its stable names, and `resolve_picture` follows its table.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test user_picture_ssr` rewrites the goldens.

#[path = "support/golden.rs"]
mod golden;

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::components::content::avatar::{
    AvatarFace, AvatarShape, AvatarSize, AvatarTone, person_hue,
};
use ds::prelude::*;
use ds_lint::{LintConfig, markup};
use sill_shell_kit::emoji::id::EmojiId;
use sill_shell_kit::prelude::*;
use sill_shell_kit::user_picture::choice::{FaceFile, PictureChoice, resolve_picture};
use sill_shell_kit::user_picture::picture::UserPicture;

fn letter() -> AvatarFace {
    AvatarFace {
        initial: 'D',
        size: AvatarSize::Size64,
        tone: AvatarTone::Person(person_hue("dana")),
        shape: AvatarShape::Round,
    }
}

fn photo() -> ImageSource {
    ImageSource("data:image/png;base64,AAAA".to_owned())
}

fn root(body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance { theme: Theme::Light, ..Appearance::default() }, material: Material::Window, stylesheet: Inject::Host,
            {body}
        }
    }
}

/// A named specimen and how to build it.
type Specimen = (&'static str, fn() -> Element);

const SPECIMENS: &[Specimen] = &[("picker", || {
    root(rsx! {
        UserPicturePicker {
            letter: letter(),
            choice: PictureChoice::Emoji(EmojiId::Fox),
            onpick: |_| {},
        }
    })
})];

fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(make);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// `html` with every `data:` URI but the stand-in photo's replaced by its length.
fn redacted(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(at) = rest.find("data:image/png;base64,") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail.find(['"', '&', ')']).unwrap_or(tail.len());
        match &tail[..end] {
            "data:image/png;base64,AAAA" => out.push_str(&tail[..end]),
            _ => out.push_str(&format!("data:image/png;base64,({} chars)", end)),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

fn by_name(name: &str) -> String {
    let make = SPECIMENS
        .iter()
        .find(|(named, _)| *named == name)
        .unwrap_or_else(|| panic!("a specimen {name}"))
        .1;
    render(make)
}

#[test]
fn every_specimen_matches_its_golden() {
    let failures: Vec<String> = SPECIMENS
        .iter()
        .filter_map(|(name, make)| {
            golden::check(
                &format!("user_picture/{name}.html"),
                &redacted(&render(*make)),
            )
            .err()
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

/// The picker offers the letter and the whole set, still, with the current choice checked.
#[test]
fn the_picker_offers_the_letter_and_every_emoji_still() {
    let picker = by_name("picker");
    assert_eq!(
        picker.matches("role=\"radio\"").count(),
        EmojiId::ALL.len() + 1
    );
    assert_eq!(picker.matches("aria-checked=\"true\"").count(), 1);
    assert!(
        picker.contains("aria-checked=\"true\" data-selected=\"selected\" aria-label=\"Fox\""),
        "{}",
        redacted(&picker)
    );
    assert!(picker.contains("aria-label=\"Letter D\""));
    assert_eq!(
        picker.matches("data-playback=\"still\"").count(),
        EmojiId::ALL.len()
    );
}

#[test]
fn the_choice_is_stored_by_stable_names() {
    let cases = [
        (PictureChoice::Auto, r#"{"kind":"auto"}"#),
        (PictureChoice::Letter, r#"{"kind":"letter"}"#),
        (
            PictureChoice::Emoji(EmojiId::HeartEyes),
            r#"{"kind":"emoji","v":"heart-eyes"}"#,
        ),
        (PictureChoice::Photo, r#"{"kind":"photo"}"#),
    ];
    for (choice, json) in cases {
        assert_eq!(
            serde_json::to_string(&choice).expect("serialise"),
            json,
            "{choice:?}"
        );
        let back: PictureChoice = serde_json::from_str(json).expect("deserialise");
        assert_eq!(back, choice, "{json}");
    }
    for emoji in EmojiId::ALL.iter().copied() {
        let choice = PictureChoice::Emoji(emoji);
        let json = serde_json::to_string(&choice).expect("serialise");
        assert!(json.contains(emoji.slug()), "{json}");
        let back: PictureChoice = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, choice);
    }
    assert_eq!(PictureChoice::default(), PictureChoice::Auto);
}

#[test]
fn resolve_follows_its_table() {
    let found = || FaceFile::Found(photo());
    let cases = [
        (PictureChoice::Auto, found(), UserPicture::Photo(photo())),
        (PictureChoice::Auto, FaceFile::Missing, letter().into()),
        (PictureChoice::Letter, found(), letter().into()),
        (PictureChoice::Letter, FaceFile::Missing, letter().into()),
        (
            PictureChoice::Emoji(EmojiId::Wink),
            found(),
            UserPicture::Emoji(EmojiId::Wink),
        ),
        (
            PictureChoice::Emoji(EmojiId::Wink),
            FaceFile::Missing,
            UserPicture::Emoji(EmojiId::Wink),
        ),
        (PictureChoice::Photo, found(), UserPicture::Photo(photo())),
        (PictureChoice::Photo, FaceFile::Missing, letter().into()),
    ];
    for (choice, face, want) in cases {
        let label = format!("{choice:?} {face:?}");
        assert_eq!(resolve_picture(choice, face, letter()), want, "{label}");
    }
}
