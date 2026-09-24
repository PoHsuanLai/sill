//! Which colour space a panel's primaries amount to, by the geometry of their triangle in CIE
//! 1931 xy against sRGB's and Display P3's. Pure: the float arithmetic stays inside
//! [`classify`]; nothing floating is stored.

use serde::{Deserialize, Serialize};

use super::edid::Primaries;

/// The gamut a display's primaries describe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gamut {
    /// sRGB-sized (within a fifth of sRGB's area), or anything the primaries cannot place.
    #[default]
    Srgb,
    /// Covers at least 90% of Display P3 without being much larger than it.
    DisplayP3,
    /// Larger than sRGB but not P3-shaped: Adobe RGB, Rec.2020-leaning panels.
    Wide,
}

type Point = (f64, f64);
type Triangle = [Point; 3];

const SRGB: Triangle = [(0.640, 0.330), (0.300, 0.600), (0.150, 0.060)];
const DISPLAY_P3: Triangle = [(0.680, 0.320), (0.265, 0.690), (0.150, 0.060)];

/// Covering this share of P3 makes a panel P3.
const P3_COVERAGE: f64 = 0.90;
/// ... unless its own triangle is this much larger than P3's (then it is wider than P3).
const P3_AREA_CEILING: f64 = 1.15;
/// A triangle this much larger than sRGB's that is not P3 is `Wide`.
const WIDE_OVER_SRGB: f64 = 1.20;

/// The gamut `primaries` describe. A degenerate triangle (all zero from a display that leaves
/// the chromaticity bytes empty) is `Srgb`, the assumption every desktop already makes.
pub fn classify(primaries: &Primaries) -> Gamut {
    let panel = [
        primaries.red.xy(),
        primaries.green.xy(),
        primaries.blue.xy(),
    ];
    let area = area(&panel);
    if area < 1e-4 {
        return Gamut::Srgb;
    }
    let p3_area = self::area(&DISPLAY_P3);
    let covers_p3 = intersection_area(&panel, &DISPLAY_P3) / p3_area;
    if covers_p3 >= P3_COVERAGE && area / p3_area <= P3_AREA_CEILING {
        Gamut::DisplayP3
    } else if area / self::area(&SRGB) >= WIDE_OVER_SRGB {
        Gamut::Wide
    } else {
        Gamut::Srgb
    }
}

/// Area of a polygon (shoelace), whatever its winding.
fn area(points: &[Point]) -> f64 {
    let n = points.len();
    let twice: f64 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    twice.abs() / 2.0
}

/// Area of the overlap of two triangles: Sutherland-Hodgman, clipping `subject` by each edge of
/// `clip` (both convex, so the result is exact).
fn intersection_area(subject: &Triangle, clip: &Triangle) -> f64 {
    let clip = counter_clockwise(clip);
    let clipped = (0..3).fold(subject.to_vec(), |polygon, i| {
        clip_edge(&polygon, clip[i], clip[(i + 1) % 3])
    });
    if clipped.len() < 3 {
        0.0
    } else {
        area(&clipped)
    }
}

fn counter_clockwise(t: &Triangle) -> Triangle {
    let cross = (t[1].0 - t[0].0) * (t[2].1 - t[0].1) - (t[1].1 - t[0].1) * (t[2].0 - t[0].0);
    if cross >= 0.0 { *t } else { [t[0], t[2], t[1]] }
}

/// `polygon` clipped to the left of the directed edge `a -> b`.
fn clip_edge(polygon: &[Point], a: Point, b: Point) -> Vec<Point> {
    let inside = |p: Point| (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0) >= 0.0;
    let crossing = |p: Point, q: Point| {
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let (ex, ey) = (b.0 - a.0, b.1 - a.1);
        let denominator = dx * ey - dy * ex;
        let t = ((a.0 - p.0) * ey - (a.1 - p.1) * ex) / denominator;
        (p.0 + t * dx, p.1 + t * dy)
    };
    let n = polygon.len();
    (0..n)
        .flat_map(|i| {
            let (current, next) = (polygon[i], polygon[(i + 1) % n]);
            match (inside(current), inside(next)) {
                (true, true) => vec![next],
                (true, false) => vec![crossing(current, next)],
                (false, true) => vec![crossing(current, next), next],
                (false, false) => vec![],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Gamut, classify};
    use crate::displays::edid::{Chromaticity, Primaries};

    /// Primaries from thousandths (as datasheets and `edid-decode` quote them), rounded to the
    /// EDID's 10-bit grid.
    fn primaries(points: [(u16, u16); 4]) -> Primaries {
        let c = |(x, y): (u16, u16)| {
            let tenbit = |v: u16| ((u32::from(v) * 1024 + 500) / 1000) as u16;
            Chromaticity {
                x: tenbit(x),
                y: tenbit(y),
            }
        };
        Primaries {
            red: c(points[0]),
            green: c(points[1]),
            blue: c(points[2]),
            white: c(points[3]),
        }
    }

    #[test]
    fn primaries_classify_by_their_triangle() {
        const D65: (u16, u16) = (313, 329);
        /// (label, primaries and white in thousandths, the gamut).
        type Case = (&'static str, [(u16, u16); 4], Gamut);
        let cases: &[Case] = &[
            (
                "BenQ MA270U (EDID)",
                [(681, 316), (256, 680), (152, 62), D65],
                Gamut::DisplayP3,
            ),
            (
                "Display P3 itself",
                [(680, 320), (265, 690), (150, 60), D65],
                Gamut::DisplayP3,
            ),
            (
                "a typical sRGB office panel",
                [(640, 335), (305, 600), (150, 60), D65],
                Gamut::Srgb,
            ),
            (
                "sRGB exactly",
                [(640, 330), (300, 600), (150, 60), D65],
                Gamut::Srgb,
            ),
            (
                "a 72% NTSC laptop panel, smaller than sRGB",
                [(590, 350), (330, 555), (155, 110), D65],
                Gamut::Srgb,
            ),
            (
                "Adobe RGB",
                [(640, 330), (210, 710), (150, 60), D65],
                Gamut::Wide,
            ),
            (
                "Rec.2020",
                [(708, 292), (170, 797), (131, 46), D65],
                Gamut::Wide,
            ),
            (
                "blank chromaticity bytes",
                [(0, 0), (0, 0), (0, 0), (0, 0)],
                Gamut::Srgb,
            ),
        ];
        for &(label, points, want) in cases {
            assert_eq!(classify(&primaries(points)), want, "{label}");
        }
    }
}
