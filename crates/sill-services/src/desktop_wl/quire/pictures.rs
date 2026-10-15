//! A captured frame of a Space as the PNG the shell shows. Pure.

use std::io::Cursor;

use desktop_client::Frame;
use image::{ImageFormat, RgbaImage};
use sill_model::desktop::SpacePicture;

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
