//! Status glyph props and words from service state: which `ds` glyph state each status item
//! draws ([`glyph`]), and the lines its menu shows ([`lines`]). Pure over the service values, so the bar and the control center draw one
//! battery, one volume and one network.

pub mod glyph;
pub mod lines;
