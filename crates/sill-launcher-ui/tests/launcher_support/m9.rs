//! The preview rows the fakes know: documents that exist in a scratch folder (a one-page
//! PDF, a PNG, a Markdown file and five more PDFs, enough to put Documents over its cap), a
//! clipboard entry, emoji and the web fallback, each with the subject the real providers give
//! it and their chords. Nothing here reads the person's home, index or clipboard.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, mpsc};

use anyview_peek::AnyPeeked;
use sill_launcher_ui::launcher::preview::load::{Look, PreviewFault, ViewerLook};
use sill_search::providers::files::actions::{activation_of, row_actions};

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
        write(&dir.join("m9-report.pdf"), &pdf(LIGHT));
        write(&dir.join("m9-shot.png"), &png());
        write(
            &dir.join("m9-notes.md"),
            b"# M9 notes\n\nThe preview pane reads this head.\n",
        );
        for n in 1..=5 {
            write(&dir.join(format!("m9-extra-{n}.pdf")), &pdf(HEAVY));
        }
        dir
    })
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).expect("a fixture file"); // test-only
}

/// How many circles a PDF fixture strokes.
#[derive(Clone, Copy)]
struct Rings(u32);

/// The report (the Top Hit, the page the tests wait for): next to nothing to raster, however
/// loaded the machine is, since the tests wait on the wall clock for it.
const LIGHT: Rings = Rings(120);

/// The extra PDFs: heavy enough that rasterising one takes a while, as a real page does, so
/// holding Down through them (`launcher_pdf_rasters`) outpaces the worker.
const HEAVY: Rings = Rings(6000);

/// A one-page PDF of `rings` small stroked circles, written by hand (offsets counted below).
fn pdf(rings: Rings) -> Vec<u8> {
    let rings: String = (0..rings.0)
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

/// A file row as the files provider builds it: its actions are the real ones for its kind.
fn document(name: &str, mime: &str, score: u32) -> Item {
    let path = folder().join(name);
    let facts = FileFacts {
        path,
        mime: Mime(mime.to_owned()),
        modified: UnixTime(1_790_000_000),
    };
    Item {
        id: ItemId(facts.path.display().to_string()),
        provider: ProviderKind::Files,
        title: name.to_owned(),
        subtitle: "~/scratch".to_owned(),
        icon: IconRef::None,
        score: Score(score),
        actions: row_actions(&facts),
        subject: Subject::File(facts),
    }
}

/// The text file the pane is slow to look at: `slow-head.txt` in the scratch folder, found only by
/// the query `slow-head`. A test writes the file ([`slow_head_path`]) and holds the pane's look at
/// it open with a [`Gated`] look until it releases it.
pub fn slow_head() -> Item {
    document(SLOW_HEAD, "text/plain", 950)
}

/// Where [`slow_head`] points.
pub fn slow_head_path() -> PathBuf {
    folder().join(SLOW_HEAD)
}

/// A look that holds the file at `slow` until the test sends on the returned sender, and looks
/// at every other file as the viewer's parts do.
pub fn gated(slow: PathBuf) -> (Arc<Gated>, mpsc::Sender<()>) {
    let (release, held) = mpsc::channel();
    (
        Arc::new(Gated {
            slow,
            held: Mutex::new(held),
        }),
        release,
    )
}

/// See [`gated`].
pub struct Gated {
    slow: PathBuf,
    held: Mutex<mpsc::Receiver<()>>,
}

impl Look for Gated {
    fn at(&self, path: &Path) -> Result<Arc<AnyPeeked>, PreviewFault> {
        if path == self.slow {
            // A sender that went away releases the look too, so a failed test does not hang.
            let held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
            let _ = held.recv();
        }
        ViewerLook::without_frames().at(path)
    }
}

const SLOW_HEAD: &str = "slow-head.txt";

/// Every preview row, in no order (the fake runner sorts).
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

/// What a fake preview provider's row does.
pub fn activate(item: &Item, choice: &Choice) -> Activation {
    match (&item.subject, choice) {
        (Subject::File(file), Choice::Action(id)) => activation_of(id, file),
        (Subject::File(file), Choice::Primary) => activation_of(&ActionId("open".to_owned()), file),
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
