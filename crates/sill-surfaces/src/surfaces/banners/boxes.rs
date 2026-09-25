//! Measured cards as region boxes, pure: a card slot's measured rect (quire's host measurer,
//! logical px, fractional) rounded outward to whole pixels, and the input box, which also takes
//! the close button hanging over the card's top-left corner.

use shell_host::Rect;

use super::regions::CardBox;

/// How far the close button reaches past the card's top and left edges, px (quire's 26 px
/// `IconButton{Strip}` pulled 35% out, rounded up; an 18 px button waits on Q120).
pub const CLOSE_REACH: i32 = 10;

/// `rect` as a box, rounded outward.
pub fn card_box(rect: ds::Rect) -> CardBox {
    let (left, top) = (rect.origin.x.0.floor(), rect.origin.y.0.floor());
    let right = (rect.origin.x.0 + rect.size.width.0).ceil();
    let bottom = (rect.origin.y.0 + rect.size.height.0).ceil();
    CardBox(Rect {
        x: left as i32,
        y: top as i32,
        width: (right - left).max(0.0) as i32,
        height: (bottom - top).max(0.0) as i32,
    })
}

/// `b` grown `by` px up and to the left (an empty box stays empty).
pub fn reached(CardBox(r): CardBox, by: i32) -> CardBox {
    match r.width > 0 && r.height > 0 {
        true => CardBox(Rect {
            x: r.x - by,
            y: r.y - by,
            width: r.width + by,
            height: r.height + by,
        }),
        false => CardBox(r),
    }
}

#[cfg(test)]
mod tests {
    use super::{card_box, reached};
    use crate::surfaces::banners::regions::CardBox;
    use ds::{Point, Px, Size};
    use shell_host::Rect;

    fn ds_rect(x: f32, y: f32, w: f32, h: f32) -> ds::Rect {
        ds::Rect {
            origin: Point { x: Px(x), y: Px(y) },
            size: Size {
                width: Px(w),
                height: Px(h),
            },
        }
    }

    fn rect(x: i32, y: i32, width: i32, height: i32) -> CardBox {
        CardBox(Rect {
            x,
            y,
            width,
            height,
        })
    }

    #[test]
    fn boxes_round_outward_and_reach_for_the_close_button() {
        assert_eq!(card_box(ds_rect(40.0, 40.0, 360.0, 64.0)), rect(40, 40, 360, 64));
        assert_eq!(card_box(ds_rect(40.4, 39.6, 360.0, 64.2)), rect(40, 39, 361, 65));
        assert_eq!(reached(rect(40, 40, 360, 64), 10), rect(30, 30, 370, 74));
        assert_eq!(reached(rect(40, 40, 0, 64), 10), rect(40, 40, 0, 64));
    }
}
