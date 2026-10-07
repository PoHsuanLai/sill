//! How a driven test is written, and how one written against the old per-input `Harness`
//! methods converts (the traits are `ds_harness::{Driver, Input, Query}`; import the ones a file
//! uses):
//!
//! | old | now |
//! |---|---|
//! | `Harness::new(app, vp)`, `Harness::with_config(app, config)` | `Harness::new(app, vp)`, `Harness::new(app, config)` (a bare `Viewport` is a config) |
//! | `Harness::with_contexts(app, vp, ctx)` | `Harness::new(app, HarnessConfig::new(vp).with_contexts(ctx))` |
//! | `Harness::unmapped(app, vp)` | `Harness::new(app, HarnessConfig::new(vp).with_layout(Layout::Held))` |
//! | `Harness::try_with_config(app, config)` | `Harness::try_new(app, config)` |
//! | `h.click(p)`, `h.press(p, b)`, `h.pointer_move(p)`, `h.pointer_down(p)`, `h.pointer_up(p)`, `h.button_down(p, b)`, `h.button_up(p, b)`, `h.drag(a, b, n)` | `h.send(Input::click(p))`, `Input::press(p, b)`, `Input::pointer_move(p)`, `Input::pointer_down(p)`, `Input::pointer_up(p)`, `Input::button_down(p, b)`, `Input::button_up(p, b)`, `Input::drag(a, b, n)` |
//! | `h.key(k)`, `h.chord(&[held], k)`, `h.wheel(p, dx, dy)` | `h.send(Input::key(k))`, `Input::chord(&[held], k)`, `Input::wheel(p, dx, dy)` |
//! | `h.ime_start()`, `ime_update(t, c)`, `ime_commit(t)`, `ime_end()`, `h.paste_html(html, text)` | `h.send(Input::ime_start())`, `Input::ime_update(t, c)`, `Input::ime_commit(t)`, `Input::ime_end()`, `Input::paste(html, text)` |
//! | `h.file_drag(step)` (returned the answer) | `h.send(Input::FileDrag(step))`, then `h.drop_answer()` |
//! | `h.pointer_move_with(p, mods)`, `button_down_with`, `button_up_with`, `click_with` | `h.send(Input::Pointer(PointerInput::new(p, action).with_mods(mods)))` |
//! | `h.is_focused(sel)` | `h.focus_of(sel) == FocusState::Focused` (or `assert_eq!` against it) |
//! | `h.has_class(sel, c)` | `h.has_class(sel, c) == ClassPresence::Present` |
//! | `h.advance(d)`, `h.render()` | unchanged, from `Driver` |
//! | `h.rect`, `text_of`, `attr`, `count`, `ink_of`, `fill_of` | unchanged, from `Query` |
//! | `ds_blitz::NativeError` | `ds_harness::HarnessError` |
//!
//! Probes shared by the headless tests: a laid-out rect, the pixels inside it, and how far
//! they stand out from their own ground. Each test crate uses some of them.

#![allow(dead_code)]

use ds::prelude::*;
use ds_harness::{Harness, Query};
use image::RgbaImage;

pub fn rect(harness: &Harness, selector: &str) -> Rect {
    harness
        .rect(selector)
        .unwrap_or_else(|| panic!("{selector} is not in the document:\n{}", harness.html()))
}

/// Where `part` is painted inside a pill centred by `translateX(-50%)`: the layout rect Blitz
/// reports leaves the transform out, so the part sits half the pill's width further left.
pub fn centred(harness: &Harness, pill: &str, part: &str) -> Rect {
    let shift = rect(harness, pill).size.width.0 / 2.0;
    let mut at = rect(harness, part);
    at.origin.x = Px(at.origin.x.0 - shift);
    at
}

/// Keep `frame` in the test's scratch directory, for review.
pub fn keep(frame: &RgbaImage, name: &str) {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.png"));
    frame.save(&path).ok();
}

/// The pixels inside `rect`, inset by `inset` on every side.
pub fn pixels(frame: &RgbaImage, rect: Rect, inset: f32) -> Vec<[u8; 4]> {
    let x0 = (rect.origin.x.0 + inset).max(0.0) as u32;
    let y0 = (rect.origin.y.0 + inset).max(0.0) as u32;
    let x1 = ((rect.origin.x.0 + rect.size.width.0 - inset) as u32).min(frame.width());
    let y1 = ((rect.origin.y.0 + rect.size.height.0 - inset) as u32).min(frame.height());
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .map(|(x, y)| frame.get_pixel(x, y).0)
        .collect()
}

/// The most common colour among `seen`.
pub fn modal(seen: &[[u8; 4]]) -> [u8; 4] {
    let mut sorted = seen.to_vec();
    sorted.sort_unstable();
    sorted
        .chunk_by(|a, b| a == b)
        .max_by_key(|run| run.len())
        .map_or([0; 4], |run| run[0])
}

/// How far apart two colours are: the largest channel difference.
pub fn distance(a: [u8; 4], b: [u8; 4]) -> u8 {
    (0..3).map(|i| a[i].abs_diff(b[i])).max().unwrap_or(0)
}

/// How many of `seen` stand out from their region's own ground by more than `by`.
pub fn ink(seen: &[[u8; 4]], by: u8) -> usize {
    let ground = modal(seen);
    seen.iter()
        .filter(|pixel| distance(**pixel, ground) > by)
        .count()
}
