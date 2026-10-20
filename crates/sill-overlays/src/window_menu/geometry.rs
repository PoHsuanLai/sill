//! Where the window menu's popup goes: the point the compositor named, in the desktop's logical
//! pixels turned into the layer's (the layer covers its output). The popup's own geometry is
//! [`sill_ui_kit::menu::PopupGeom`].

use shell_host::Rect;

/// `at`, in the desktop's logical pixels, in the layer of the output whose rectangle is
/// `output`; `None` when the point is on another output.
pub(super) fn local_point(at: (i32, i32), output: Rect) -> Option<(i32, i32)> {
    let inside_x = at.0 >= output.x && at.0 < output.x.saturating_add(output.width);
    let inside_y = at.1 >= output.y && at.1 < output.y.saturating_add(output.height);
    (inside_x && inside_y).then(|| (at.0 - output.x, at.1 - output.y))
}

#[cfg(test)]
mod tests {
    use shell_host::Rect;

    use super::local_point;

    fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// (case, the point, what the right-hand output makes of it).
    type PointCase = (&'static str, (i32, i32), Option<(i32, i32)>);

    #[test]
    fn the_points_on_the_output_are_made_local_and_the_rest_are_not_ours() {
        let right = rect(1920, 0, 1920, 1080);
        // (case, the point, what the right-hand output makes of it)
        let cases: [PointCase; 5] = [
            ("its top left corner", (1920, 0), Some((0, 0))),
            ("inside", (2000, 40), Some((80, 40))),
            ("past its right edge", (3840, 40), None),
            ("on the other output", (100, 40), None),
            ("past its bottom edge", (2000, 1080), None),
        ];
        for (name, at, want) in cases {
            assert_eq!(local_point(at, right), want, "{name}");
        }
    }
}
