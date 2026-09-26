//! The emoji grid's open risk, timed (M9 lane c, FINDINGS "M9 freeze" F656): "a scrolling grid
//! of a few hundred colour glyphs at 60 Hz on vello_hybrid is untimed". Lane d's grid
//! (`sill_launcher::rank::sections`, `sill-surfaces` launcher panel) leans on these numbers
//! before it relies on the emoji provider's rows at scale.
//!
//! Draws 300 real `.ds-emoji-text` cells (quire's colour-emoji face, `--font-emoji`; never
//! `font-family` written here, which quire's stylesheet lint refuses) in a `ds_native::Harness`
//! — a real Blitz document, real layout, real Noto Color Emoji COLRv1 painting — scrolled at a
//! simulated 60 Hz, and reports each frame's paint time as p50, p95 and max.
//!
//! It times vello_cpu and vello_hybrid (quire Q330: `HarnessConfig::with_backend`, on the
//! discrete and the integrated GPU) at 100 %, 150 % and 200 % scale, the frame's whole paint
//! (`Harness::paint_timed`, GPU included), with the grid's real cell (56 px) and glyph size
//! (`--fs-emoji-cell`). A backend with no GPU is skipped and said so.
//!

//! `cargo run -p sill-surfaces --example emoji_paint_bench`

use std::time::Duration;

use dioxus::prelude::*;
use ds::{Appearance, Ds, Material, Point, Px};
use ds_native::{AdapterPref, Backend, Harness, HarnessConfig, Viewport};

/// The grid's cell size (quire `EmojiGrid` default: 56 px).
const CELL_PX: u32 = 56;
/// Columns (`EmojiGrid` default: 8).
const COLUMNS: u32 = 8;
/// Cells drawn: the open risk names "a few hundred".
const CELL_COUNT: usize = 300;
/// The visible grid, in logical px: `COLUMNS` cells wide, several rows tall.
const WIDTH: u32 = CELL_PX * COLUMNS;
const HEIGHT: u32 = 620;
/// Three seconds of continuous scroll at 60 Hz.
const FRAMES: usize = 180;
/// A 60 Hz frame.
const FRAME: Duration = Duration::from_millis(16);
/// Wheel delta per simulated frame.
const SCROLL_PX_PER_FRAME: f32 = 18.0;
/// The scales timed: the user's 27" 4K runs at 200 %.
const SCALES: [u16; 3] = [100, 150, 200];

/// A representative spread of colour emoji (faces, hands in several tones, animals, food,
/// flags, symbols): real COLRv1 glyphs, cycled to fill [`CELL_COUNT`] cells.
const GLYPHS: &[&str] = &[
    "😀", "😃", "😄", "😁", "😆", "😅", "🤣", "😂", "🙂", "😉", "😊", "😍", "🥰", "😘", "😜", "🤔",
    "😴", "🥳", "😎", "🤯", "👋", "👋🏻", "👋🏼", "👋🏽", "👋🏾", "👋🏿", "👍", "👍🏻", "👍🏼", "👍🏽", "🤝", "🙏",
    "👏", "💪", "🫶", "🐶", "🐱", "🦊", "🐼", "🐸", "🍎", "🍕", "🍔", "🍣", "☕", "🍰", "🎉", "🎈",
    "⚽", "🚀", "🌈", "⭐", "🔥", "❤️", "✨", "🏳️", "🏴", "🎌", "🇹🇼", "🇯🇵",
];

/// One renderer to time.
struct Run {
    label: &'static str,
    backend: Backend,
    adapter: AdapterPref,
}

fn runs() -> [Run; 3] {
    [
        Run {
            label: "vello_cpu",
            backend: Backend::Cpu,
            adapter: AdapterPref::Auto,
        },
        Run {
            label: "hybrid discrete",
            backend: Backend::Hybrid,
            adapter: AdapterPref::Discrete,
        },
        Run {
            label: "hybrid integrated",
            backend: Backend::Hybrid,
            adapter: AdapterPref::Integrated,
        },
    ]
}

fn main() {
    for run in runs() {
        for scale in SCALES {
            time(&run, scale);
        }
    }
}

/// Scroll the grid for [`FRAMES`] frames on `run`'s renderer at `scale` percent and print
/// the paint times.
fn time(run: &Run, scale: u16) {
    let viewport = Viewport {
        width: WIDTH,
        height: HEIGHT,
        scale_percent: scale,
    };
    let config = HarnessConfig::new(viewport)
        .with_backend(run.backend)
        .with_adapter(run.adapter.clone());
    let mut harness = match Harness::try_with_config(GridApp, config) {
        Ok(harness) => harness,
        Err(error) => {
            println!("{} @{scale}%: skipped ({error})", run.label);
            return;
        }
    };
    let adapter = harness.adapter().unwrap_or("cpu").to_owned();
    let point = Point {
        x: Px(WIDTH as f32 / 2.0),
        y: Px(HEIGHT as f32 / 2.0),
    };
    let mut times = Vec::with_capacity(FRAMES);
    for _ in 0..FRAMES {
        harness.wheel(point, Px(0.0), Px(SCROLL_PX_PER_FRAME));
        harness.advance(FRAME);
        match harness.paint_timed() {
            Ok(paint) => times.push(paint.total),
            Err(error) => {
                println!("{} @{scale}%: paint failed ({error})", run.label);
                return;
            }
        }
    }
    report(&format!("{} @{scale}% [{adapter}]", run.label), &times);
}

/// `label`'s frame times as p50, p95 and max.
fn report(label: &str, frame_times: &[Duration]) {
    let mut sorted = frame_times.to_vec();
    sorted.sort();
    let at = |fraction: f64| -> Duration {
        let last = sorted.len().saturating_sub(1);
        let index = ((last as f64) * fraction).round() as usize;
        sorted[index.min(last)]
    };
    let max = sorted.last().copied().unwrap_or_default();
    println!(
        "{label}: frames={} p50={:>6.2}ms p95={:>6.2}ms max={:>6.2}ms",
        sorted.len(),
        at(0.50).as_secs_f64() * 1000.0,
        at(0.95).as_secs_f64() * 1000.0,
        max.as_secs_f64() * 1000.0,
    );
}

/// A scrollable grid of [`CELL_COUNT`] emoji, each carrying quire's `ds-emoji-text` class at
/// the grid's glyph size token.
#[allow(non_snake_case)]
fn GridApp() -> Element {
    let scroll_style = format!("width:{WIDTH}px;height:{HEIGHT}px;overflow-y:auto;");
    let grid_style = format!("display:flex;flex-wrap:wrap;width:{WIDTH}px;");
    let cell_style = format!(
        "width:{CELL_PX}px;height:{CELL_PX}px;display:flex;align-items:center;justify-content:center;font-size:var(--fs-emoji-cell);"
    );
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Sheet,
            div { style: scroll_style,
                div { style: grid_style,
                    for i in 0..CELL_COUNT {
                        div {
                            key: "{i}",
                            class: "ds-emoji-text",
                            style: "{cell_style}",
                            "{GLYPHS[i % GLYPHS.len()]}"
                        }
                    }
                }
            }
        }
    }
}
