use std::path::PathBuf;

use super::{ExcerptLimit, PageIndex, Preview, PreviewKind, excerpt_of, preview_for};
use crate::ids::{AppId, ClipKey, ItemId, ProviderKind};
use crate::item::{IconRef, Item};
use crate::rank::Score;
use crate::subject::{
    CharCount, ClipFacts, ClipKind, EmojiFacts, Excerpt, FileFacts, Mime, SkinTone, Subject,
    WebFacts,
};
use crate::time::UnixTime;

fn row(provider: ProviderKind, title: &str, subtitle: &str, subject: Subject) -> Item {
    Item {
        id: ItemId(title.to_owned()),
        provider,
        title: title.to_owned(),
        subtitle: subtitle.to_owned(),
        icon: IconRef::None,
        score: Score(0),
        actions: Vec::new(),
        subject,
    }
}

fn file(path: &str, mime: &str) -> Subject {
    Subject::File(FileFacts {
        path: PathBuf::from(path),
        mime: Mime(mime.to_owned()),
        modified: UnixTime(0),
    })
}

#[test]
fn a_preview_kind_is_its_variant() {
    assert_eq!(Preview::None.kind(), PreviewKind::None);
    assert_eq!(
        Preview::Pdf {
            path: PathBuf::from("/a.pdf"),
            page: PageIndex::FIRST
        }
        .kind(),
        PreviewKind::Pdf
    );
}

#[test]
fn every_subject_has_its_preview() {
    let clip = ClipFacts {
        key: ClipKey(3),
        kind: ClipKind::Text {
            excerpt: Excerpt("hello".to_owned()),
            chars: CharCount(5),
        },
        copied: UnixTime(0),
    };
    let emoji = EmojiFacts {
        glyph: "😀".to_owned(),
        name: "grinning face".to_owned(),
        tone: SkinTone::Default,
    };
    let web = WebFacts {
        engine: "duckduckgo.com".to_owned(),
        url: "https://duckduckgo.com/?q=x".to_owned(),
    };
    let cases: Vec<(Item, Preview)> = vec![
        (
            row(ProviderKind::System, "Lock", "", Subject::None),
            Preview::None,
        ),
        (
            row(
                ProviderKind::Apps,
                "Files",
                "File manager",
                Subject::App(AppId("com.system76.CosmicFiles".to_owned())),
            ),
            Preview::App {
                app: AppId("com.system76.CosmicFiles".to_owned()),
                name: "Files".to_owned(),
                detail: "File manager".to_owned(),
            },
        ),
        (
            row(
                ProviderKind::Files,
                "a.pdf",
                "",
                file("/h/a.pdf", "application/pdf"),
            ),
            Preview::Pdf {
                path: PathBuf::from("/h/a.pdf"),
                page: PageIndex::FIRST,
            },
        ),
        (
            row(
                ProviderKind::Files,
                "p.png",
                "",
                file("/h/p.png", "image/png"),
            ),
            Preview::Image(PathBuf::from("/h/p.png")),
        ),
        (
            row(
                ProviderKind::Files,
                "n.md",
                "",
                file("/h/n.md", "text/markdown"),
            ),
            Preview::TextFile(PathBuf::from("/h/n.md")),
        ),
        (
            row(
                ProviderKind::Files,
                "c.rs",
                "",
                file("/h/c.rs", "text/rust"),
            ),
            Preview::TextFile(PathBuf::from("/h/c.rs")),
        ),
        (
            row(
                ProviderKind::Files,
                "Documents",
                "",
                file("/h/Documents", Mime::FOLDER),
            ),
            Preview::File(FileFacts {
                path: PathBuf::from("/h/Documents"),
                mime: Mime(Mime::FOLDER.to_owned()),
                modified: UnixTime(0),
            }),
        ),
        (
            row(
                ProviderKind::Files,
                "z.zip",
                "",
                file("/h/z.zip", "application/zip"),
            ),
            Preview::File(FileFacts {
                path: PathBuf::from("/h/z.zip"),
                mime: Mime("application/zip".to_owned()),
                modified: UnixTime(0),
            }),
        ),
        (
            row(
                ProviderKind::Clipboard,
                "hello",
                "",
                Subject::Clip(clip.clone()),
            ),
            Preview::Clipboard(clip),
        ),
        (
            row(ProviderKind::Emoji, "😀", "", Subject::Emoji(emoji.clone())),
            Preview::Emoji(emoji),
        ),
        (
            row(
                ProviderKind::Web,
                "Search the web",
                "",
                Subject::Web(web.clone()),
            ),
            Preview::Web(web),
        ),
    ];
    for (item, want) in cases {
        assert_eq!(preview_for(&item), want, "{}", item.title);
    }
}

#[test]
fn an_excerpt_is_the_head_of_the_text_within_its_limit() {
    let small = ExcerptLimit { lines: 2, chars: 8 };
    let cases: Vec<(&[u8], ExcerptLimit, &str)> = vec![
        (b"one\ntwo\nthree\n", small, "one\ntwo"),
        (b"abcdefghijkl", small, "abcdefgh"),
        (
            "naïve café".as_bytes(),
            ExcerptLimit { lines: 1, chars: 3 },
            "naï",
        ),
        (b"trailing   \n\n", ExcerptLimit::PANE, "trailing"),
        (b"\x00\x01binary", ExcerptLimit::PANE, ""),
        (b"bad \xff byte", ExcerptLimit::PANE, "bad \u{fffd} byte"),
    ];
    for (bytes, limit, want) in cases {
        assert_eq!(
            excerpt_of(bytes, limit),
            Excerpt(want.to_owned()),
            "{bytes:?}"
        );
    }
}
