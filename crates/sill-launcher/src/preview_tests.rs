use std::path::PathBuf;

use super::{ExcerptLimit, Preview, PreviewKind, excerpt_of, preview_for};
use crate::ids::{AppId, ClipKey, ItemId, ProviderKind};
use crate::item::{IconRef, Item};
use crate::rank::Score;
use crate::subject::{
    CharCount, ClipFacts, ClipKind, EmojiFacts, Excerpt, FileFacts, HelperAttention, HelperFacts,
    Mime, SkinTone, Subject, WebFacts,
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

fn file_facts(path: &str, mime: &str) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        mime: Mime(mime.to_owned()),
        modified: UnixTime(0),
    }
}

fn file(path: &str, mime: &str) -> Subject {
    Subject::File(file_facts(path, mime))
}

#[test]
fn a_preview_kind_is_its_variant() {
    let file = FileFacts {
        path: PathBuf::from("/a.pdf"),
        mime: Mime("application/pdf".to_owned()),
        modified: UnixTime(0),
    };
    assert_eq!(Preview::None.kind(), PreviewKind::None);
    assert_eq!(Preview::File(file).kind(), PreviewKind::File);
}

#[test]
fn every_subject_has_its_preview() {
    let task = prov::TaskId::parse("t-1").unwrap_or_else(|e| panic!("{e}"));
    let run = prov::RunId::parse("r-1").unwrap_or_else(|e| panic!("{e}"));
    let memory = almanac_core::MemoryItem::Fact(almanac_core::FactId::mint(1, [3; 10]));
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
                Subject::App(AppId("org.gnome.Nautilus".to_owned())),
            ),
            Preview::App {
                app: AppId("org.gnome.Nautilus".to_owned()),
                name: "Files".to_owned(),
                detail: "File manager".to_owned(),
            },
        ),
        // Every file is the viewer's to sniff, whatever the index called it: the row decides
        // nothing about the format.
        (
            row(
                ProviderKind::Files,
                "a.pdf",
                "",
                file("/h/a.pdf", "application/pdf"),
            ),
            Preview::File(file_facts("/h/a.pdf", "application/pdf")),
        ),
        (
            row(
                ProviderKind::Files,
                "p.png",
                "",
                file("/h/p.png", "image/png"),
            ),
            Preview::File(file_facts("/h/p.png", "image/png")),
        ),
        (
            row(
                ProviderKind::Files,
                "c.rs",
                "",
                file("/h/c.rs", "text/rust"),
            ),
            Preview::File(file_facts("/h/c.rs", "text/rust")),
        ),
        (
            row(
                ProviderKind::Files,
                "Documents",
                "",
                file("/h/Documents", Mime::FOLDER),
            ),
            Preview::File(file_facts("/h/Documents", Mime::FOLDER)),
        ),
        (
            row(
                ProviderKind::Files,
                "z.zip",
                "",
                file("/h/z.zip", "application/zip"),
            ),
            Preview::File(file_facts("/h/z.zip", "application/zip")),
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
        (
            row(
                ProviderKind::Companion,
                "Forward the receipts",
                "",
                Subject::Answer(task.clone()),
            ),
            Preview::Answer(task),
        ),
        (
            row(
                ProviderKind::Helpers,
                "Research",
                "",
                Subject::Helper(HelperFacts {
                    id: prov::HelperId::parse("h-1").unwrap_or_else(|e| panic!("{e}")),
                    attention: HelperAttention::WaitingForYou,
                }),
            ),
            Preview::Helper(prov::HelperId::parse("h-1").unwrap_or_else(|e| panic!("{e}"))),
        ),
        (
            row(
                ProviderKind::Helpers,
                "New Helper\u{2026}",
                "",
                Subject::NewHelper,
            ),
            Preview::NewHelper,
        ),
        (
            row(
                ProviderKind::Runs,
                "Book a table",
                "",
                Subject::Run(run.clone()),
            ),
            Preview::Run(run),
        ),
        (
            row(
                ProviderKind::Memory,
                "Sam prefers mornings",
                "",
                Subject::Memory(memory.clone()),
            ),
            Preview::Memory(memory),
        ),
        (
            row(
                ProviderKind::Companion,
                "Archived 3 threads",
                "",
                Subject::Activity(docket_core::UndoId(4)),
            ),
            Preview::None,
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
