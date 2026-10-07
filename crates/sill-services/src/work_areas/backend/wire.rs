//! The property's wire type, `(us(iiuu)(iiuu)u)`, made from the model's.

use sill_model::work_areas::{Rect, WorkArea};

/// `(iiuu)`: x, y, width, height.
pub(super) type WireRect = (i32, i32, u32, u32);

/// One output: id, name, frame, work area, scale in 120ths.
pub(super) type WireArea = (u32, String, WireRect, WireRect, u32);

fn rect(rect: Rect) -> WireRect {
    (rect.x, rect.y, rect.width, rect.height)
}

pub(super) fn wire(area: &WorkArea) -> WireArea {
    (
        area.id,
        area.name.clone(),
        rect(area.frame),
        rect(area.work),
        area.scale,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::Type;

    #[test]
    fn the_signature_is_the_documented_one() {
        assert_eq!(
            <Vec<WireArea> as Type>::SIGNATURE.to_string(),
            "a(us(iiuu)(iiuu)u)"
        );
    }

    #[test]
    fn an_area_is_its_id_name_frame_work_and_scale() {
        let frame = Rect {
            x: -10,
            y: 0,
            width: 2560,
            height: 1440,
        };
        let area = WorkArea {
            id: 4,
            name: "DP-2".to_owned(),
            frame,
            work: Rect {
                y: 32,
                height: 1408,
                ..frame
            },
            scale: 180,
        };
        assert_eq!(
            wire(&area),
            (
                4,
                "DP-2".to_owned(),
                (-10, 0, 2560, 1440),
                (-10, 32, 2560, 1408),
                180
            )
        );
    }
}
