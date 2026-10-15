//! The colour picker: an app asks for one colour from the screen (`org.quire.Shell1.PickColor`,
//! the portal's `PickColor`), the person points at it with an eyedropper, and the answer is that
//! pixel's colour, or nothing when they cancel. Pure.
//!
//! The picker is only open while an eyedropper is drawn ([`Eyedropper::Present`]); until a
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

/// How the person ended the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorOutcome {
    /// Clicked this colour.
    Picked(Rgb8),
    /// Pressed Escape.
    Cancel,
}

/// Whether a surface draws the eyedropper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Eyedropper {
    /// Nothing draws it: a request cannot be put to the person.
    #[default]
    Absent,
    /// A surface draws it and answers with [`ColorOutcome`].
    Present,
}

/// The call's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorReply {
    /// The colour picked; none means the person cancelled.
    pub color: Option<Rgb8>,
}

impl ColorReply {
    /// Cancelled.
    pub fn cancelled() -> ColorReply {
        ColorReply { color: None }
    }

    /// The reply to how the person ended the request.
    pub fn of(outcome: ColorOutcome) -> ColorReply {
        match outcome {
            ColorOutcome::Picked(color) => ColorReply { color: Some(color) },
            ColorOutcome::Cancel => ColorReply::cancelled(),
        }
    }

    /// The portal's `a(ddd)`: the colour as red, green and blue in 0..1, or an empty array when
    /// cancelled.
    pub fn wire(&self) -> Vec<(f64, f64, f64)> {
        let unit = |channel: u8| f64::from(channel) / 255.0;
        self.color
            .map(|c| (unit(c.red), unit(c.green), unit(c.blue)))
            .into_iter()
            .collect()
    }
}
