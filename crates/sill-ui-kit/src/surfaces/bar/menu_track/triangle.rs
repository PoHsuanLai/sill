//! The safe triangle (design/13 §13.5): `q` is guarded iff it is inside the triangle
//! `(p_prev, A - (0, 4), B + (0, 4))`; barycentric sign test.

use shell_host::Point;

use super::types::Guard;

/// Whether `q` is inside (or on an edge of) the triangle `a b c`, either winding.
pub fn inside(q: Point, a: Point, b: Point, c: Point) -> bool {
    let cross = |p: Point, r: Point, s: Point| {
        i64::from(r.x - p.x) * i64::from(s.y - p.y) - i64::from(r.y - p.y) * i64::from(s.x - p.x)
    };
    let d1 = cross(a, b, q);
    let d2 = cross(b, c, q);
    let d3 = cross(c, a, q);
    let negative = d1 < 0 || d2 < 0 || d3 < 0;
    let positive = d1 > 0 || d2 > 0 || d3 > 0;
    !(negative && positive)
}

/// Whether `q` is inside `guard`'s triangle, the submenu corners inflated 4 px vertically.
pub fn guarded(guard: &Guard, q: Point) -> bool {
    let top = Point {
        x: guard.top.x,
        y: guard.top.y - 4,
    };
    let bottom = Point {
        x: guard.bottom.x,
        y: guard.bottom.y + 4,
    };
    inside(q, guard.from, top, bottom)
}
