//! The lock screen, the polkit prompt and the app switcher as markup (M11): every part in every
//! state matches its golden under `tests/snapshots/lock_switcher/`, lints clean, and uses only
//! `ds-` classes the stylesheet styles; the markup carries what the props say, and no state
//! writes a password field's `value`.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test it lock_switcher_ssr` rewrites the goldens.

use crate::support::golden;

use dioxus::core::NoOpMutations;
use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::components::content::avatar::{
    AvatarFace, AvatarShape, AvatarSize, AvatarTone, person_hue,
};
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::icon::family::PlateFamily;
use ds_lint::{LintConfig, markup};
use sill_shell_kit::emoji::id::EmojiId;
use sill_shell_kit::lock::vocab::{CapsLock, LockLook, LockUser, PromptState};
use sill_shell_kit::prelude::*;
use sill_shell_kit::switcher::app_switcher::{AppKey, SwitcherApp};

fn user() -> LockUser {
    LockUser::new(
        "Dana Reyes",
        AvatarFace {
            initial: 'D',
            size: AvatarSize::Size20,
            tone: AvatarTone::Person(person_hue("dana")),
            shape: AvatarShape::Round,
        },
    )
}

/// Dana with her photo, a portrait-shaped PNG the disc crops.
fn photo_user() -> LockUser {
    LockUser::new(
        "Dana Reyes",
        ImageSource("data:image/png;base64,AAAA".to_owned()),
    )
}

/// Dana as her emoji.
fn emoji_user() -> LockUser {
    LockUser::new("Dana Reyes", EmojiId::HeartEyes)
}

fn root(material: Material, body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance { theme: Theme::Light, ..Appearance::default() }, material, stylesheet: Inject::Host,
            {body}
        }
    }
}

fn clock(look: LockLook) -> Element {
    root(
        Material::Window,
        rsx! { LockClock { time: "9:41", date: "Saturday 26 September", look } },
    )
}

fn prompt(state: PromptState, caps: CapsLock, look: LockLook) -> Element {
    root(
        Material::Window,
        rsx! {
            LockPrompt { user: user(), state, caps, look, hint: Some(TextLine::from("Press Enter to unlock")),
                oninput: |_| {}, onsubmit: |_| {} }
        },
    )
}

fn prompt_for(user: LockUser, state: PromptState) -> Element {
    root(
        Material::Window,
        rsx! {
            LockPrompt { user, state, hint: Some(TextLine::from("Press Enter to unlock")),
                oninput: |_| {}, onsubmit: |_| {} }
        },
    )
}

fn screen() -> Element {
    root(
        Material::Window,
        rsx! {
            LockScreen {
                wallpaper: ImageSource("data:image/png;base64,AAAA".to_owned()),
                clock: rsx! { LockClock { time: "9:41", date: "Saturday 26 September" } },
                prompt: rsx! { LockPrompt { user: user(), oninput: |_| {}, onsubmit: |_| {} } },
            }
        },
    )
}

fn polkit(state: PromptState, caps: CapsLock) -> Element {
    polkit_with(user(), state, caps)
}

fn polkit_for(user: LockUser, state: PromptState) -> Element {
    polkit_with(user, state, CapsLock::Off)
}

fn polkit_with(user: LockUser, state: PromptState, caps: CapsLock) -> Element {
    root(
        Material::Sheet,
        rsx! {
            PolkitPrompt {
                action: "Authentication is required to change the system's time zone.",
                detail: Some(TextLine::from("org.freedesktop.timedate1.set-timezone")),
                user,
                state,
                caps,
                oninput: |_| {},
                onsubmit: |_| {},
                oncancel: |_| {},
            }
        },
    )
}

const APPS: [(&str, &str, Icon, PlateFamily); 14] = [
    ("mail", "Mail", Icon::Mail, PlateFamily::Blue),
    ("files", "Files", Icon::Folder, PlateFamily::Blue),
    ("terminal", "Terminal", Icon::Terminal, PlateFamily::Neutral),
    ("notes", "Notes", Icon::StickyNote, PlateFamily::Amber),
    ("photos", "Photos", Icon::Image, PlateFamily::Violet),
    ("settings", "Settings", Icon::Settings, PlateFamily::Neutral),
    ("calendar", "Calendar", Icon::Clock, PlateFamily::Red),
    ("music", "Music", Icon::Headphones, PlateFamily::Red),
    ("camera", "Camera", Icon::Camera, PlateFamily::Neutral),
    ("downloads", "Downloads", Icon::Download, PlateFamily::Green),
    ("monitor", "System Monitor", Icon::Gauge, PlateFamily::Green),
    ("printer", "Printers", Icon::Printer, PlateFamily::Neutral),
    ("keyboard", "Keyboard", Icon::Keyboard, PlateFamily::Violet),
    ("display", "Displays", Icon::Monitor, PlateFamily::Amber),
];

fn apps(count: usize) -> Vec<SwitcherApp> {
    APPS.iter()
        .take(count)
        .map(|(key, name, icon, family)| SwitcherApp {
            plate: Some(*family),
            ..SwitcherApp::new(*key, *name, IconSource::Glyph(*icon))
        })
        .collect()
}

fn switcher(count: usize, selected: &str, output: f32) -> Element {
    let body = rsx! {
        AppSwitcher {
            apps: apps(count),
            selected: AppKey(selected.to_owned()),
            output: Some(Px(output)),
            onhover: |_| {},
            onactivate: |_| {},
        }
    };
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Osd, chrome: Some(RootChrome::Transparent), stylesheet: Inject::Host,
            {body}
        }
    }
}

type Specimen = (&'static str, fn() -> Element);

const SPECIMENS: &[Specimen] = &[
    ("clock-clear", || clock(LockLook::Clear)),
    ("clock-space", || clock(LockLook::Space)),
    ("prompt-idle", || {
        prompt(PromptState::Idle, CapsLock::Off, LockLook::Clear)
    }),
    ("prompt-checking", || {
        prompt(PromptState::Checking, CapsLock::Off, LockLook::Clear)
    }),
    ("prompt-wrong-caps", || {
        prompt(PromptState::Wrong, CapsLock::On, LockLook::Clear)
    }),
    ("prompt-locked-out", || {
        prompt(
            PromptState::LockedOut {
                until: "9:52".to_owned(),
            },
            CapsLock::Off,
            LockLook::Clear,
        )
    }),
    ("prompt-space", || {
        prompt(PromptState::Idle, CapsLock::Off, LockLook::Space)
    }),
    ("screen", screen),
    ("polkit-idle", || polkit(PromptState::Idle, CapsLock::Off)),
    ("polkit-wrong-caps", || {
        polkit(PromptState::Wrong, CapsLock::On)
    }),
    ("polkit-checking", || {
        polkit(PromptState::Checking, CapsLock::Off)
    }),
    ("polkit-locked-out", || {
        polkit(
            PromptState::LockedOut {
                until: "9:52".to_owned(),
            },
            CapsLock::Off,
        )
    }),
    ("switcher-5", || switcher(5, "files", 1440.0)),
    ("switcher-14-shrunk", || switcher(14, "files", 1216.0)),
    ("switcher-14-scrolled", || switcher(14, "downloads", 800.0)),
    ("prompt-photo", || {
        prompt_for(photo_user(), PromptState::Idle)
    }),
    ("prompt-emoji", || {
        prompt_for(emoji_user(), PromptState::Idle)
    }),
    ("prompt-emoji-accepted", || {
        prompt_for(emoji_user(), PromptState::Accepted)
    }),
    ("polkit-photo", || {
        polkit_for(photo_user(), PromptState::Idle)
    }),
    ("polkit-emoji", || {
        polkit_for(emoji_user(), PromptState::Idle)
    }),
];

/// The specimen called `name`.
fn specimen(name: &str) -> fn() -> Element {
    SPECIMENS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, make)| *make)
        .unwrap_or_else(|| panic!("no specimen {name}"))
}

/// Build the dom and flush the effects that register overlays, so a sheet is in the markup.
fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(make);
    dom.rebuild_in_place();
    for _ in 0..3 {
        dom.render_immediate(&mut NoOpMutations);
    }
    dioxus_ssr::render(&dom)
}

/// `html` with every emoji sheet's `data:` URI replaced by its length (as `emoji_ssr`), so a
/// golden stays readable and still notices a different sheet; the stand-in pictures' short URIs
/// are kept.
fn redacted(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(at) = rest.find("data:image/png;base64,") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail.find(['"', '&', ')']).unwrap_or(tail.len());
        match end {
            0..=64 => out.push_str(&tail[..end]),
            _ => out.push_str(&format!("data:image/png;base64,({} chars)", end)),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

#[test]
fn every_specimen_matches_its_golden() {
    let failures: Vec<String> = SPECIMENS
        .iter()
        .filter_map(|(name, make)| {
            golden::check(
                &format!("lock_switcher/{name}.html"),
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

/// Every password field is a secret: `data-kind="secret"` and no `value` attribute, whatever
/// the state.
#[test]
fn no_password_field_writes_a_value() {
    for (name, make) in SPECIMENS
        .iter()
        .filter(|(name, _)| name.starts_with("prompt") || name.starts_with("polkit"))
    {
        let html = render(*make);
        let field = html
            .split("<input")
            .nth(1)
            .and_then(|rest| rest.split('>').next())
            .unwrap_or_else(|| panic!("{name}: a field in {html}"));
        assert!(field.contains("data-kind=\"secure\""), "{name}: {field}");
        assert!(!field.contains("value="), "{name}: {field}");
    }
}

/// The markup says what the props say: the state, the caps mark, the closed field and the
/// lock-out line; the switcher's fit and selection. A shake plays on arriving at `Wrong`, not on
/// mounting in it (`lock_switcher` in ds-native drives that).
#[test]
fn the_markup_carries_the_props() {
    let wrong = render(specimen("prompt-wrong-caps"));
    for want in [
        "data-state=\"wrong\"",
        "class=\"ds-password-field\"",
        "data-form=\"pill\"",
        "aria-label=\"Caps Lock is on\"",
        "data-size=\"64\"",
    ] {
        assert!(wrong.contains(want), "{want} in {wrong}");
    }
    let idle = render(specimen("prompt-idle"));
    assert!(!idle.contains("a-shake-x"), "{idle}");
    assert!(!idle.contains("Caps Lock"), "{idle}");
    let checking = render(specimen("prompt-checking"));
    for want in [
        "aria-busy=\"true\"",
        "ds-progress",
        "aria-disabled=\"true\"",
    ] {
        assert!(checking.contains(want), "{want} in {checking}");
    }
    let out = render(specimen("prompt-locked-out"));
    assert!(out.contains("Try again at 9:52"), "{out}");
    let polkit = render(specimen("polkit-locked-out"));
    assert!(
        polkit.contains("Too many tries. Try again at 9:52."),
        "{polkit}"
    );
    assert!(polkit.contains("data-width=\"narrow\""), "{polkit}");

    let scrolled = render(specimen("switcher-14-scrolled"));
    assert!(
        scrolled.contains(
            "--switcher-cell:64px;--switcher-icon:48px;--switcher-gap:8px;--switcher-view:704px;--switcher-shift:296px;--switcher-at:9"
        ),
        "{scrolled}"
    );
    assert_eq!(scrolled.matches("aria-selected=\"true\"").count(), 1);
    let shrunk = render(specimen("switcher-14-shrunk"));
    assert!(
        shrunk.contains("--switcher-cell:72px;--switcher-icon:56px"),
        "{shrunk}"
    );
}

/// A photo is cropped round at the face's size (64 at the lock, 48 in the polkit sheet), with
/// no letter; an emoji is drawn at Medium, idle at rest and happy once accepted.
#[test]
fn the_picture_is_the_kind_the_user_carries() {
    let by_name = |name: &str| {
        let make = SPECIMENS
            .iter()
            .find(|(named, _)| *named == name)
            .unwrap_or_else(|| panic!("a specimen {name}"))
            .1;
        render(make)
    };
    let photo = by_name("prompt-photo");
    for want in [
        "class=\"ds-user-photo\" data-size=\"64\"",
        "class=\"ds-user-photo-image\"",
        "src=\"data:image/png;base64,AAAA\"",
    ] {
        assert!(photo.contains(want), "{want} in {photo}");
    }
    assert!(!photo.contains("ds-avatar"), "{photo}");
    assert!(
        by_name("polkit-photo").contains("class=\"ds-user-photo\" data-size=\"48\""),
        "the polkit photo at 48"
    );
    let emoji = by_name("prompt-emoji");
    assert!(
        emoji.contains("class=\"ds-emoji\" data-size=\"64\""),
        "{emoji}"
    );
    assert!(!emoji.contains("ds-avatar"), "{emoji}");
    assert!(
        !emoji.contains("data-mood"),
        "the picture has no mood: {emoji}"
    );
    let accepted = by_name("prompt-emoji-accepted");
    assert!(accepted.contains("data-state=\"accepted\""), "{accepted}");
}

fn polkit_named() -> Element {
    root(
        Material::Sheet,
        rsx! {
            PolkitPrompt {
                action: "Authentication is required to change the system's time zone.",
                user: user(),
                oninput: |_| {},
                onsubmit: |_| {},
                oncancel: |_| {},
                common: Common { id: Some("polkit-panel".to_owned()), ..Common::default() },
            }
        },
    )
}

/// The shell resolves the polkit sheet's blur region by the panel's id.
#[test]
fn the_polkit_sheet_carries_the_panel_id() {
    let html = render(polkit_named);
    assert!(
        html.contains(r#"class="ds-sheet" id="polkit-panel""#)
            || html.contains(r#"id="polkit-panel""#) && html.contains("ds-sheet"),
        "{html}"
    );
    let plain = render(|| polkit(PromptState::Idle, CapsLock::Off));
    assert!(
        !plain.contains(r#"id=""#),
        "no empty id without a common id"
    );
}
