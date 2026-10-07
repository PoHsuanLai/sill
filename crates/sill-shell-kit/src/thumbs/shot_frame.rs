//! The shape of a screenshot thumbnail's card (design/04-COMPONENTS.md section 39): the card is
//! `width` across, the picture box inside a 4 px mat takes the picture's own ratio held between
//! 2:1 and 16:10, and the picture is fitted into that box, centred, so a picture outside those
//! ratios is letterboxed on the material. Computed here rather than left to `object-fit`, so the
//! box the host sizes its surface for is a number a test can read.

use ds::base::geometry::units::{Point, Px, Rect, Size};
use ds::components::content::image_source::ImageSize;

/// The mat between the card's edge and the picture box: `--s-4`, which the stylesheet's picture
/// radius subtracts from the card's.
pub(crate) const MAT: Px = Px(4.0);

/// The tallest picture box, as height over width: 16:10, the shape sill sizes the thumbnail's
/// surface for; a taller picture is pillarboxed.
const TALLEST: f32 = 10.0 / 16.0;

/// The widest picture box, as height over width: 2:1; a wider picture is letterboxed rather
/// than drawing a card too thin to press.
const WIDEST: f32 = 1.0 / 2.0;

/// A card's shape: its outer size, and the picture's rect inside it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShotFrame {
    /// The card's outer size.
    pub card: Size,
    /// The picture, relative to the card's top-left corner.
    pub picture: Rect,
}

/// The card `width` across for a picture of `size`. A picture with no pixels fills the box.
pub(crate) fn shot_frame(width: Px, size: ImageSize) -> ShotFrame {
    let inner = Px((width.0 - 2.0 * MAT.0).max(0.0));
    let ratio = ratio(size);
    let box_height = Px(inner.0 * ratio.unwrap_or(TALLEST).clamp(WIDEST, TALLEST));
    let picture = fit(
        Size {
            width: inner,
            height: box_height,
        },
        ratio,
    );
    ShotFrame {
        card: Size {
            width,
            height: Px(box_height.0 + 2.0 * MAT.0),
        },
        picture: Rect {
            origin: Point {
                x: Px(MAT.0 + picture.origin.x.0),
                y: Px(MAT.0 + picture.origin.y.0),
            },
            size: picture.size,
        },
    }
}

/// Height over width, or `None` for a picture with no pixels.
fn ratio(size: ImageSize) -> Option<f32> {
    (size.width > 0 && size.height > 0).then(|| size.height as f32 / size.width as f32)
}

/// A picture of `ratio` fitted inside `room`, centred: the largest rect of that ratio that fits.
fn fit(room: Size, ratio: Option<f32>) -> Rect {
    let size = match ratio {
        None => room,
        Some(ratio) if room.width.0 * ratio <= room.height.0 => Size {
            width: room.width,
            height: Px(room.width.0 * ratio),
        },
        Some(ratio) => Size {
            width: Px(room.height.0 / ratio),
            height: room.height,
        },
    };
    Rect {
        origin: Point {
            x: Px((room.width.0 - size.width.0) / 2.0),
            y: Px((room.height.0 - size.height.0) / 2.0),
        },
        size,
    }
}

#[cfg(test)]
mod tests {
    use super::{MAT, shot_frame};
    use ds::base::geometry::units::{Point, Px, Rect, Size};
    use ds::components::content::image_source::ImageSize;
    use ds::style::tokens::spacing::SpacingToken;

    fn rect(left: f32, top: f32, width: f32, height: f32) -> Rect {
        Rect {
            origin: Point {
                x: Px(left),
                y: Px(top),
            },
            size: Size {
                width: Px(width),
                height: Px(height),
            },
        }
    }

    #[test]
    fn the_box_follows_the_picture_between_two_to_one_and_sixteen_to_ten() {
        // (width, picture, card height, the picture's rect in the card)
        #[rustfmt::skip]
        let cases = [
            // 16:9, inside the range: the box is the picture, no bars.
            (240.0, (1920, 1080), 138.5, rect(4.0, 4.0, 232.0, 130.5)),
            // 16:10 exactly.
            (240.0, (1440, 900), 153.0, rect(4.0, 4.0, 232.0, 145.0)),
            // Portrait: the box stops at 16:10 and the picture is pillarboxed.
            (240.0, (900, 1800), 153.0, rect(83.75, 4.0, 72.5, 145.0)),
            // A panorama: the box stops at 2:1 and the picture is letterboxed.
            (240.0, (4000, 1000), 124.0, rect(4.0, 33.0, 232.0, 58.0)),
            // No pixels: the picture fills a 16:10 box.
            (240.0, (0, 0), 153.0, rect(4.0, 4.0, 232.0, 145.0)),
            // Another width scales the same way.
            (320.0, (1920, 1200), 203.0, rect(4.0, 4.0, 312.0, 195.0)),
        ];
        for (width, (w, h), card, want) in cases {
            let frame = shot_frame(
                Px(width),
                ImageSize {
                    width: w,
                    height: h,
                },
            );
            assert_eq!(frame.card.width, Px(width), "{w}x{h}");
            assert!(
                (frame.card.height.0 - card).abs() < 0.01,
                "{w}x{h}: {frame:?}"
            );
            let close = |got: Px, want: Px| (got.0 - want.0).abs() < 0.01;
            assert!(
                close(frame.picture.left(), want.left())
                    && close(frame.picture.top(), want.top())
                    && close(frame.picture.size.width, want.size.width)
                    && close(frame.picture.size.height, want.size.height),
                "{w}x{h}: {:?}",
                frame.picture
            );
        }
    }

    #[test]
    fn the_mat_is_the_spacing_token_the_stylesheet_subtracts() {
        assert_eq!(u32::from(SpacingToken::S4.tenths()), (MAT.0 * 10.0) as u32);
    }
}
