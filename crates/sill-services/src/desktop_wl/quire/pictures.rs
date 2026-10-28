//! A captured frame as the picture the shell shows or reads. Pure.

use std::io::Cursor;
use std::sync::Arc;

use casement_protocols::vocab::Transform;
use desktop_client::Frame;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, ImageFormat, RgbaImage};
use sill_model::desktop::{OutputPicture, OutputShot, SpacePicture};

/// `frame` as PNG bytes, or [`SpacePicture::Failed`] when its format or size is wrong. A Space
/// picture comes upright, so the frame's transform is not applied.
pub fn picture_of(frame: &Frame) -> SpacePicture {
    let Ok(rgba) = frame.to_rgba8() else {
        return SpacePicture::Failed;
    };
    let Some(image) = RgbaImage::from_raw(frame.size.w, frame.size.h, rgba) else {
        return SpacePicture::Failed;
    };
    let mut out = Cursor::new(Vec::new());
    match image.write_to(&mut out, ImageFormat::Png) {
        Ok(()) => SpacePicture::Png(out.into_inner()),
        Err(_) => SpacePicture::Failed,
    }
}

/// `frame`, a whole output, as the pixels the eyedropper reads and the PNG it draws, or
/// [`OutputPicture::Failed`] when its format or size is wrong or it does not come upright (the
/// transform is not applied, so a pixel read from a turned frame would be the wrong one). The
/// PNG is encoded fast and unfiltered: the picture is shown once and thrown away.
pub fn shot_of(frame: &Frame) -> OutputPicture {
    let upright = frame.transform == Transform::Normal;
    let Ok(rgba) = frame.to_rgba8() else {
        return OutputPicture::Failed;
    };
    let (width, height) = (frame.size.w, frame.size.h);
    let sized = rgba.len() == width as usize * height as usize * 4;
    if !upright || !sized || width == 0 || height == 0 {
        return OutputPicture::Failed;
    }
    let mut png = Vec::new();
    let encoder =
        PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::NoFilter);
    match encoder.write_image(&rgba, width, height, ExtendedColorType::Rgba8) {
        Ok(()) => OutputPicture::Shot(OutputShot {
            size: (width, height),
            rgba: Arc::from(rgba),
            png: Arc::from(png),
        }),
        Err(_) => OutputPicture::Failed,
    }
}
