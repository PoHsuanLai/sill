//! The reduction: an output less its panels' zones.

use super::model::{Edge, Rect, WorkArea, Zone};

/// `frame` less every zone: a top zone moves the top edge down, a bottom zone moves the bottom
/// edge up, and so on. Zones on one edge add up. The result never has a negative size: past
/// the far edge it is empty, sitting on the edge the zones squeezed it against.
pub fn work_rect(frame: Rect, zones: &[Zone]) -> Rect {
    let sum = |edge: Edge| {
        zones
            .iter()
            .filter(|zone| zone.edge == edge)
            .map(|zone| i64::from(zone.px))
            .sum::<i64>()
    };
    let (left, right) = (sum(Edge::Left), sum(Edge::Right));
    let (top, bottom) = (sum(Edge::Top), sum(Edge::Bottom));
    let width = (i64::from(frame.width) - left - right).max(0);
    let height = (i64::from(frame.height) - top - bottom).max(0);
    Rect {
        x: clamp(i64::from(frame.x) + left),
        y: clamp(i64::from(frame.y) + top),
        width: u32::try_from(width).unwrap_or(u32::MAX),
        height: u32::try_from(height).unwrap_or(u32::MAX),
    }
}

fn clamp(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

/// One output's [`WorkArea`]: `frame` and `scale` as given, `work` from `zones`.
pub fn work_area(id: u32, name: &str, frame: Rect, scale: u32, zones: &[Zone]) -> WorkArea {
    WorkArea {
        id,
        name: name.to_owned(),
        frame,
        work: work_rect(frame, zones),
        scale,
    }
}
