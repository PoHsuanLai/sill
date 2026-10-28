//! The colour picker: an app asks for one colour from the screen (`org.quire.Shell1.PickColor`,
//! the portal's `PickColor`), the person points at it with an eyedropper, and the answer is that
//! pixel's colour (`Some`), or nothing when they cancel (`None`). Pure.
//!
//! The picker is only open while an eyedropper is drawn (`CapturePicker::eyedropper`); until a
//! surface says it is there, a request is answered with a cancel at once and nothing is asked
//! of the person.

use super::step::AskId;

/// One colour as 8-bit sRGB, which is what a captured pixel is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb8 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// An app's request for one colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorAsk {
    /// The app's name as the person knows it.
    pub app_label: String,
}

/// The request on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorAsking {
    pub id: AskId,
    pub ask: ColorAsk,
}

/// The portal's `a(ddd)`: the colour as red, green and blue in 0..1, or an empty array when the
/// person cancelled.
pub fn color_wire(color: Option<Rgb8>) -> Vec<(f64, f64, f64)> {
    let unit = |channel: u8| f64::from(channel) / 255.0;
    color
        .map(|c| (unit(c.red), unit(c.green), unit(c.blue)))
        .into_iter()
        .collect()
}
