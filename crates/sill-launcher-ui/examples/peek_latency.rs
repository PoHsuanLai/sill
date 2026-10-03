//! How long the preview pane's peek of each file takes, off the compositor: `Look::at` (probe,
//! then peek) run once cold, then `N` times warm, printed as milliseconds. It is what a recording's
//! preview costs the launcher, with and without anyview-peek's `media` feature.
//!
//! `cargo run --release -p sill-launcher-ui --example peek_latency -- [-n N] FILE...`

use std::path::Path;
use std::time::Instant;

use sill_launcher_ui::launcher::preview::load::{Look, ViewerLook};

fn millis(path: &Path) -> f64 {
    let start = Instant::now();
    let _ = ViewerLook.at(path);
    start.elapsed().as_secs_f64() * 1000.0
}

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    let mut samples = 30_usize;
    if args.peek().map(String::as_str) == Some("-n") {
        args.next();
        samples = args.next().and_then(|n| n.parse().ok()).unwrap_or(samples);
    }
    for file in args {
        let path = Path::new(&file);
        let cold = millis(path);
        let mut warm: Vec<f64> = (0..samples).map(|_| millis(path)).collect();
        warm.sort_by(f64::total_cmp);
        let at = |p: f64| warm[((warm.len() - 1) as f64 * p).round() as usize];
        println!(
            "{file}: cold {cold:.2} ms, warm p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms (n={})",
            at(0.5),
            at(0.95),
            warm[warm.len() - 1],
            warm.len()
        );
    }
}
