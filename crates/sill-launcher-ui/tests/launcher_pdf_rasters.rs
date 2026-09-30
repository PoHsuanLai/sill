//! Holding Down through several PDFs with the preview pane shown rasterises few of them: quire's
//! `PdfFileThumb` runs one worker with a latest-wins queue (m9-parts), and the pane hands it
//! only the selected row's path. Its own test binary: `ds_blitz::pdf_thumb_rasters` counts for
//! the whole process.

mod launcher_harness;

use ds_harness::Driver;
use std::time::Duration;

use launcher_harness::{Press, open};
use sill_launcher::PreviewKind;

#[test]
fn holding_down_through_pdfs_rasterises_only_a_few() {
    let mut open = open("m9");
    // The Top Hit is a PDF; the pane shows it (and rasterises it once).
    open.press(Press::QuickLook);
    for _ in 0..10 {
        open.harness.advance(Duration::from_millis(20));
    }
    let before = ds_blitz::pdf_thumb_rasters();
    // Past the PNG and the Markdown file through three more PDFs (the rest are behind Show More),
    // each press rendered at once, as a held key repeats faster than a page rasterises (the
    // fixtures are tiny: a real page takes longer, so this is the kind case).
    for _ in 0..5 {
        open.press(Press::Down);
        open.harness.advance(Duration::from_millis(1));
    }
    let shown = open.link.rows().expect("the open panel publishes its rows");
    assert_eq!(
        shown.preview,
        Some(PreviewKind::Pdf),
        "the last PDF's preview"
    );
    // Let the queue drain.
    for _ in 0..50 {
        open.harness.advance(Duration::from_millis(20));
    }
    let rasters = ds_blitz::pdf_thumb_rasters() - before;
    eprintln!("pdf rasters while holding Down through 3 PDFs: {rasters}");
    assert!(rasters <= 2, "{rasters} rasters for 3 PDFs passed over");
}
