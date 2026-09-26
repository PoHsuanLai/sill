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
//! **vello_cpu only.** `ds_native::Harness::render` paints through `anyrender_vello_cpu`
//! (`ds-native/src/headless.rs`); it has no public path to paint through `anyrender_vello_hybrid`
//! instead (that backend is only reachable from a raw wgpu device and a hand-built scene, as
//! `ds-native/examples/emoji_probe_hybrid.rs` does, bypassing the DOM entirely — not "300
//! `ds-emoji` cells in a `ds_native::Harness`"). Reaching it for real DOM content needs either a
//! `Harness` method quire adds, or new dev-dependencies (`vello_hybrid`, `wgpu`, `wgpu_context`)
//! in `crates/sill-surfaces/Cargo.toml`, which this wave gives to lane d, not lane c. Reported as
//! a gap (FINDINGS "M9 lane c") rather than worked around here.
//!
//! `cargo run -p sill-surfaces --example emoji_paint_bench`

use std::time::{Duration, Instant};

use dioxus::prelude::*;
use ds::{Appearance, Ds, Material, Point, Px};
use ds_native::{Harness, Viewport};

/// The grid's cell size (quire gap Q291 proposes 56 px).
const CELL_PX: f32 = 56.0;
/// Columns (F656: `GRID_COLUMNS 8`).
const COLUMNS: usize = 8;
/// Cells drawn: the open risk names "a few hundred".
const CELL_COUNT: usize = 300;
/// The visible viewport: `COLUMNS` cells wide, tall enough to show several rows.
const VIEWPORT: Viewport = Viewport {
    width: (CELL_PX as u32) * (COLUMNS as u32),
    height: 620,
    scale_percent: 100,
};
/// A 60 Hz frame.
const FRAME: Duration = Duration::from_millis(16);
/// Three seconds of continuous scroll.
const FRAMES: usize = 180;
/// Wheel delta per simulated frame: the grid scrolls its full content over the run.
const SCROLL_PX_PER_FRAME: f32 = 18.0;

/// A representative spread of colour emoji (faces, hands in several tones, animals, food,
/// flags, symbols): real COLRv1 glyphs, cycled to fill [`CELL_COUNT`] cells.
const GLYPHS: &[&str] = &[
    "😀", "😃", "😄", "😁", "😆", "😅", "🤣", "😂", "🙂", "😉", "😊", "😍", "🥰", "😘", "😜", "🤔",
    "😴", "🥳", "😎", "🤯", "👋", "👋🏻", "👋🏼", "👋🏽", "👋🏾", "👋🏿", "👍", "👍🏻", "👍🏼", "👍🏽", "🤝", "🙏",
    "👏", "💪", "🫶", "🐶", "🐱", "🦊", "🐼", "🐸", "🍎", "🍕", "🍔", "🍣", "☕", "🍰", "🎉", "🎈",
    "⚽", "🚀", "🌈", "⭐", "🔥", "❤️", "✨", "🏳️", "🏴", "🎌", "🇹🇼", "🇯🇵",
];

fn main() {
    let mut harness = Harness::new(GridApp, VIEWPORT);
    let scroll_point = Point {
        x: Px(VIEWPORT.width as f32 / 2.0),
        y: Px(VIEWPORT.height as f32 / 2.0),
    };

    let mut frame_times = Vec::with_capacity(FRAMES);
    for _ in 0..FRAMES {
        harness.wheel(scroll_point, Px(0.0), Px(SCROLL_PX_PER_FRAME));
        harness.advance(FRAME);
        let started = Instant::now();
        if let Err(error) = harness.render() {
            eprintln!("emoji_paint_bench: vello_cpu render failed: {error}");
            std::process::exit(1);
        }
        frame_times.push(started.elapsed());
    }
    report("vello_cpu", &frame_times);

    eprintln!(
        "emoji_paint_bench: vello_hybrid not measured (see this file's module doc and \
         FINDINGS \"M9 lane c\" for why)."
    );
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

/// A scrollable grid of [`CELL_COUNT`] emoji, each carrying quire's `ds-emoji-text` class.
#[allow(non_snake_case)]
fn GridApp() -> Element {
    let scroll_style = format!(
        "width:{}px;height:{}px;overflow-y:auto;",
        VIEWPORT.width, VIEWPORT.height
    );
    let grid_style = format!(
        "display:flex;flex-wrap:wrap;width:{}px;",
        COLUMNS as f32 * CELL_PX
    );
    let cell_style = format!(
        "width:{CELL_PX}px;height:{CELL_PX}px;display:flex;align-items:center;justify-content:center;font-size:32px;"
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
