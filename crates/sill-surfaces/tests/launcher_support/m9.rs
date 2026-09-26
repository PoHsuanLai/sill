//! The M9 rows the fakes know (lane d): documents that exist in a scratch folder (a one-page
//! PDF, a PNG, a Markdown file and five more PDFs, enough to put Documents over its cap), a
//! clipboard entry, emoji and the web fallback, each with the subject the real providers give
//! it and their chords. Nothing here reads the person's home, index or clipboard.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use sill_launcher::{
    Action, ActionId, ActionKeys, Activation, CharCount, Choice, ClipFacts, ClipKey, ClipKind,
    EmojiFacts, Excerpt, FileFacts, IconRef, Item, ItemId, Mime, OpenTarget, ProviderKind, Score,
    SkinTone, Subject, UnixTime, WebFacts, chords,
};

/// The scratch folder the documents are written to, once per test binary.
pub fn folder() -> &'static Path {
    static FOLDER: OnceLock<PathBuf> = OnceLock::new();
    FOLDER.get_or_init(|| {
        // A test binary's own scratch space under target/; the demo's temp directory.
        let base =
            option_env!("CARGO_TARGET_TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from);
        let dir = base.join(format!("sill-launcher-m9-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a scratch folder"); // test-only
        write(&dir.join("m9-report.pdf"), &pdf());
        write(&dir.join("m9-shot.png"), &png());
        write(
            &dir.join("m9-notes.md"),
            b"# M9 notes\n\nThe preview pane reads this head.\n",
        );
        for n in 1..=5 {
            write(&dir.join(format!("m9-extra-{n}.pdf")), &pdf());
        }
        dir
    })
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).expect("a fixture file"); // test-only
}

/// A one-page PDF of many small stroked circles, written by hand (offsets counted below): heavy
/// enough that rasterising it takes a while, as a real page does.
pub fn pdf() -> Vec<u8> {
    let rings: String = (0..6000)
        .map(|n| {
            let (x, y) = (20 + (n * 7) % 160, 20 + (n * 13) % 220);
            format!("{x} {y} m {} {y} {x} {} {x} {y} c S\n", x + 9, y + 9)
        })
        .collect();
    let stream = format!("0.2 0.4 0.8 RG 0.3 w\n0.9 0.9 0.9 rg 20 20 160 220 re f\n{rings}");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 260] /Contents 4 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (n, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{body}\nendobj\n", n + 1).as_bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        out.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// A small PNG, two colours.
pub fn png() -> Vec<u8> {
    let image = image::RgbaImage::from_fn(64, 48, |x, _| {
        if x < 32 {
            image::Rgba([40, 120, 220, 255])
        } else {
            image::Rgba([240, 180, 40, 255])
        }
    });
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("a PNG encodes"); // test-only
    bytes.into_inner()
}

fn action(id: &str, label: &str, keys: ActionKeys) -> Action {
    Action {
        id: ActionId(id.to_owned()),
        label: label.to_owned(),
        keys,
    }
}

fn document(name: &str, mime: &str, score: u32) -> Item {
    let path = folder().join(name);
    Item {
        id: ItemId(path.display().to_string()),
        provider: ProviderKind::Files,
        title: name.to_owned(),
        subtitle: "~/scratch".to_owned(),
        icon: IconRef::None,
        score: Score(score),
        actions: vec![
            action(
                "reveal",
                "Reveal in Files",
                ActionKeys::Chord(chords::reveal()),
            ),
            action("copy-path", "Copy Path", ActionKeys::Chord(chords::copy())),
        ],
        subject: Subject::File(FileFacts {
            path,
            mime: Mime(mime.to_owned()),
            modified: UnixTime(1_790_000_000),
        }),
    }
}

/// Every M9 row, in no order (the fake runner sorts).
pub fn rows() -> Vec<Item> {
    let mut rows = vec![
        document("m9-report.pdf", "application/pdf", 900),
        document("m9-shot.png", "image/png", 880),
        document("m9-notes.md", "text/markdown", 860),
    ];
    rows.extend(
        (1..=5).map(|n| document(&format!("m9-extra-{n}.pdf"), "application/pdf", 700 - n)),
    );
    rows.push(Item {
        id: ItemId("7".to_owned()),
        provider: ProviderKind::Clipboard,
        title: "m9 copied words".to_owned(),
        subtitle: "Text".to_owned(),
        icon: IconRef::None,
        score: Score(800),
        actions: vec![action(
            "forget",
            "Forget",
            ActionKeys::Chord(chords::forget()),
        )],
        subject: Subject::Clip(ClipFacts {
            key: ClipKey(7),
            kind: ClipKind::Text {
                excerpt: Excerpt("m9 copied words".to_owned()),
                chars: CharCount(15),
            },
            copied: UnixTime(1_790_000_000),
        }),
    });
    for (glyph, name) in [("🎉", "party popper"), ("👍", "thumbs up"), ("🦊", "fox")] {
        rows.push(Item {
            id: ItemId(glyph.to_owned()),
            provider: ProviderKind::Emoji,
            title: glyph.to_owned(),
            subtitle: name.to_owned(),
            icon: IconRef::None,
            score: Score(300),
            actions: vec![action("copy", "Copy", ActionKeys::Chord(chords::copy()))],
            subject: Subject::Emoji(EmojiFacts {
                glyph: glyph.to_owned(),
                name: name.to_owned(),
                tone: SkinTone::Default,
            }),
        });
    }
    rows
}

/// The web fallback's row for `query` (typed text only).
pub fn web(query: &str) -> Item {
    Item {
        id: ItemId("web:search".to_owned()),
        provider: ProviderKind::Web,
        title: format!("Search the web for “{query}”"),
        subtitle: "duckduckgo.com".to_owned(),
        icon: IconRef::None,
        score: Score(1),
        actions: Vec::new(),
        subject: Subject::Web(WebFacts {
            engine: "duckduckgo.com".to_owned(),
            url: format!("https://duckduckgo.com/?q={query}"),
        }),
    }
}

/// What a fake M9 provider's row does.
pub fn activate(item: &Item, choice: &Choice) -> Activation {
    match (&item.subject, choice) {
        (Subject::File(file), Choice::Action(id)) if id.0 == "reveal" => Activation::Open {
            target: OpenTarget::Reveal(file.path.clone()),
        },
        (Subject::File(file), Choice::Action(_)) => Activation::Copy {
            text: file.path.display().to_string(),
        },
        (Subject::File(file), Choice::Primary) => Activation::Open {
            target: OpenTarget::File(file.path.clone()),
        },
        (Subject::Clip(clip), Choice::Action(_)) => Activation::ForgetClipboard { entry: clip.key },
        (Subject::Clip(clip), Choice::Primary) => Activation::SetClipboard { entry: clip.key },
        (Subject::Emoji(emoji), _) => Activation::Copy {
            text: emoji.glyph.clone(),
        },
        (Subject::Web(web), _) => Activation::Open {
            target: OpenTarget::Url(web.url.clone()),
        },
        _ => Activation::Nothing,
    }
}
