//! Making a re-tinted raster (effects: the filesystem). Blocking: the cache runs it on
//! tokio's blocking pool, never on the UI thread.
//!
//! The source is read, rasterised at the key's size (an SVG through resvg, a PNG or JPEG
//! decoded and fitted), put through `ds::icon::retint` and written as a PNG, atomically.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use ds::icon::{IconStyle, Tint, retint};
use image::imageops::FilterType;
use image::{ImageFormat, RgbaImage};

use crate::app_icons::size::AssetPx;

/// Why an icon could not be made; the cache then keeps showing the untinted file.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("icon file: {0}")]
    Io(#[from] std::io::Error),
    #[error("icon raster: {0}")]
    Image(#[from] image::ImageError),
    #[error("icon svg: {0}")]
    Svg(String),
}

/// `source` at `px`, re-tinted for `style` and `tint`, as PNG bytes.
pub fn render(
    source: &Path,
    px: AssetPx,
    style: IconStyle,
    tint: Tint,
) -> Result<Vec<u8>, RenderError> {
    let bytes = std::fs::read(source)?;
    let mut raster = match source.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("svg") => rasterize_svg(&bytes, px)?,
        _ => fit(image::load_from_memory(&bytes)?.to_rgba8(), px),
    };
    retint(raster.as_mut(), style, tint);
    let mut png = Vec::new();
    raster.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
    Ok(png)
}

/// An SVG drawn into a `px` square, fitted and centred, as straight-alpha RGBA.
fn rasterize_svg(bytes: &[u8], px: AssetPx) -> Result<RgbaImage, RenderError> {
    use resvg::{tiny_skia, usvg};
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default())
        .map_err(|e| RenderError::Svg(e.to_string()))?;
    let side = u32::from(px.0);
    let mut pixmap = tiny_skia::Pixmap::new(side, side)
        .ok_or_else(|| RenderError::Svg(format!("no {side} px canvas")))?;
    let size = tree.size();
    let scale = f64::from(side) / f64::from(size.width().max(size.height()));
    let dx = (f64::from(side) - f64::from(size.width()) * scale) / 2.0;
    let dy = (f64::from(side) - f64::from(size.height()) * scale) / 2.0;
    let transform =
        tiny_skia::Transform::from_row(scale as f32, 0.0, 0.0, scale as f32, dx as f32, dy as f32);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let straight: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let c = pixel.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    RgbaImage::from_raw(side, side, straight)
        .ok_or_else(|| RenderError::Svg("pixmap size".to_owned()))
}

/// `raster` scaled to fit a `px` square and centred on a transparent one.
fn fit(raster: RgbaImage, px: AssetPx) -> RgbaImage {
    let side = u32::from(px.0);
    if raster.width() == side && raster.height() == side {
        return raster;
    }
    let longest = raster.width().max(raster.height()).max(1);
    let w = (raster.width() * side / longest).max(1);
    let h = (raster.height() * side / longest).max(1);
    let scaled = image::imageops::resize(&raster, w, h, FilterType::Lanczos3);
    let mut canvas = RgbaImage::new(side, side);
    image::imageops::overlay(
        &mut canvas,
        &scaled,
        i64::from((side - w) / 2),
        i64::from((side - h) / 2),
    );
    canvas
}

/// `bytes` written to `dir/name` through a temporary file and a rename, so a reader (Blitz, or
/// another sill process) never sees half a PNG.
pub fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::fs::create_dir_all(dir)?;
    let target = dir.join(name);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let temp = dir.join(format!(".{name}.{}-{n}.tmp", std::process::id()));
    std::fs::write(&temp, bytes)?;
    std::fs::rename(&temp, &target).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })?;
    Ok(target)
}

/// Whether `target` was written after `source` last changed (a theme update re-renders).
pub fn fresh(target: &Path, source: &Path) -> bool {
    let modified = |path: &Path| std::fs::metadata(path).and_then(|m| m.modified()).ok();
    match (modified(target), modified(source)) {
        (Some(made), Some(changed)) => made >= changed,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{fresh, render, write_atomic};
    use crate::app_icons::size::AssetPx;
    use ds::icon::{IconStyle, Tint};
    use std::path::PathBuf;

    fn temp(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sill-surfaces-icons-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        dir
    }

    fn decode(png: &[u8]) -> image::RgbaImage {
        image::load_from_memory(png)
            .unwrap_or_else(|e| panic!("{e}"))
            .to_rgba8()
    }

    /// A red SVG square, rendered at 48 in Monochrome with a green tint: the right size, the
    /// square's pixels green-hued, the transparent margin still transparent.
    #[test]
    fn an_svg_is_rasterised_at_the_key_size_and_retinted() {
        let dir = temp("svg");
        let svg = dir.join("app.svg");
        std::fs::write(
            &svg,
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'>\
             <rect x='4' y='4' width='16' height='16' fill='#e8483c'/></svg>",
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let tint = Tint {
            hue: 150.0,
            chroma: 0.07,
        };
        let png = render(&svg, AssetPx(48), IconStyle::Monochrome, tint)
            .unwrap_or_else(|e| panic!("{e}"));
        let raster = decode(&png);
        assert_eq!(raster.dimensions(), (48, 48));
        let [r, g, b, a] = raster.get_pixel(24, 24).0;
        assert_eq!(a, 255);
        assert!(g > r && g > b, "green-hued: {r} {g} {b}");
        assert_eq!(raster.get_pixel(2, 2).0[3], 0, "the margin stays clear");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_png_is_fitted_and_colour_leaves_it_alone() {
        let dir = temp("png");
        let src = dir.join("app.png");
        image::RgbaImage::from_pixel(64, 32, image::Rgba([10, 120, 240, 255]))
            .save(&src)
            .unwrap_or_else(|e| panic!("{e}"));
        let png = render(&src, AssetPx(32), IconStyle::Colour, Tint::NEUTRAL)
            .unwrap_or_else(|e| panic!("{e}"));
        let raster = decode(&png);
        assert_eq!(raster.dimensions(), (32, 32));
        assert_eq!(raster.get_pixel(16, 16).0, [10, 120, 240, 255]);
        assert_eq!(raster.get_pixel(16, 2).0[3], 0, "letterboxed, centred");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_atomic_write_leaves_only_the_file_and_it_is_fresh() {
        let dir = temp("write");
        let src = dir.join("source.png");
        std::fs::write(&src, b"s").unwrap_or_else(|e| panic!("{e}"));
        let out = dir.join("out");
        let path = write_atomic(&out, "k.png", b"png").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(std::fs::read(&path).ok(), Some(b"png".to_vec()));
        let names: Vec<String> = std::fs::read_dir(&out)
            .unwrap_or_else(|e| panic!("{e}"))
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["k.png".to_owned()]);
        assert!(fresh(&path, &src));
        assert!(!fresh(&out.join("missing.png"), &src));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unreadable_source_is_an_error_not_a_panic() {
        let missing = std::path::Path::new("/nonexistent/sill/icon.svg");
        assert!(render(missing, AssetPx(48), IconStyle::Muted, Tint::NEUTRAL).is_err());
    }
}
