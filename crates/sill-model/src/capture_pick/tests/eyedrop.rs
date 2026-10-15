use std::sync::Arc;

use super::super::*;
use crate::desktop::{OutputPicture, OutputShot};

/// A picture `width` by `height` whose pixel `(x, y)` is `(x, y, 7)`, so every pixel names itself.
fn shot(width: u32, height: u32) -> OutputShot {
    let rgba: Vec<u8> = (0..height)
        .flat_map(|y| (0..width).flat_map(move |x| [x as u8, y as u8, 7, 255]))
        .collect();
    OutputShot {
        size: (width, height),
        rgba: Arc::from(rgba),
        png: Arc::from(vec![0x89, b'P']),
    }
}

fn at(x: u32, y: u32) -> ShotPixel {
    ShotPixel { x, y }
}

#[test]
fn the_pixel_under_the_pointer_follows_the_scale_on_each_axis() {
    // (name, picture, surface, pointer, pixel)
    type Case = (
        &'static str,
        (u32, u32),
        (u32, u32),
        (f64, f64),
        Option<ShotPixel>,
    );
    const CASES: &[Case] = &[
        (
            "one to one",
            (1920, 1080),
            (1920, 1080),
            (100.5, 50.2),
            Some(ShotPixel { x: 100, y: 50 }),
        ),
        (
            "scale 1.5 floors the device position",
            (1920, 1080),
            (1280, 720),
            (100.5, 50.2),
            Some(ShotPixel { x: 150, y: 75 }),
        ),
        (
            "scale 1.25",
            (1600, 900),
            (1280, 720),
            (3.9, 3.9),
            Some(ShotPixel { x: 4, y: 4 }),
        ),
        (
            "scale 2 maps the last logical pixel to the last two",
            (3840, 2160),
            (1920, 1080),
            (1919.9, 1079.9),
            Some(ShotPixel { x: 3839, y: 2159 }),
        ),
        (
            "the origin",
            (1920, 1080),
            (1280, 720),
            (0.0, 0.0),
            Some(ShotPixel { x: 0, y: 0 }),
        ),
        (
            "a rounded logical size scales each axis alone",
            (1366, 768),
            (911, 512),
            (910.9, 511.9),
            Some(ShotPixel { x: 1365, y: 767 }),
        ),
        (
            "right of the surface",
            (1920, 1080),
            (1280, 720),
            (1280.0, 10.0),
            None,
        ),
        (
            "below the surface",
            (1920, 1080),
            (1280, 720),
            (10.0, 720.0),
            None,
        ),
        (
            "left of the surface",
            (1920, 1080),
            (1280, 720),
            (-0.1, 5.0),
            None,
        ),
        (
            "not a number",
            (1920, 1080),
            (1280, 720),
            (f64::NAN, 5.0),
            None,
        ),
        ("an empty surface", (1920, 1080), (0, 720), (0.0, 0.0), None),
        ("an empty picture", (0, 1080), (1280, 720), (0.0, 0.0), None),
    ];
    for (name, picture, surface, pointer, want) in CASES {
        assert_eq!(pixel_under(*picture, *surface, *pointer), *want, "{name}");
    }
}

#[test]
fn a_pixels_colour_is_read_from_its_row_and_column() {
    let picture = shot(4, 3);
    let colour = |pixel| colour_at(&picture, pixel);
    assert_eq!(
        colour(at(0, 0)),
        Some(Rgb8 {
            red: 0,
            green: 0,
            blue: 7
        })
    );
    assert_eq!(
        colour(at(3, 0)),
        Some(Rgb8 {
            red: 3,
            green: 0,
            blue: 7
        })
    );
    assert_eq!(
        colour(at(1, 2)),
        Some(Rgb8 {
            red: 1,
            green: 2,
            blue: 7
        })
    );
    assert_eq!(colour(at(4, 0)), None, "past the last column");
    assert_eq!(colour(at(0, 3)), None, "past the last row");
    let short = OutputShot {
        rgba: Arc::from(vec![1_u8, 2, 3, 255]),
        ..picture
    };
    assert_eq!(
        colour_at(&short, at(1, 0)),
        None,
        "pixels the buffer does not hold"
    );
}

#[test]
fn a_click_answers_with_the_pixel_under_the_pointer_at_a_fractional_scale() {
    // 6 by 4 device pixels on a surface of 4 by 3 logical pixels: 1.5 across, 4/3 down.
    let picture = shot(6, 4);
    let picked = |pointer| pick_at(&picture, (4, 3), pointer);
    assert_eq!(
        picked((2.9, 1.0)),
        Some(Rgb8 {
            red: 4,
            green: 1,
            blue: 7
        })
    );
    assert_eq!(
        picked((3.1, 2.9)),
        Some(Rgb8 {
            red: 4,
            green: 3,
            blue: 7
        })
    );
    assert_eq!(picked((4.0, 1.0)), None);
}

#[test]
fn the_magnifier_centres_the_pixel_under_the_pointer_and_leaves_the_edges_empty() {
    let loupe = |pointer| loupe_at((1920, 1080), (1920, 1080), pointer).expect("on the picture");
    let middle = loupe((100.2, 50.0));
    assert_eq!(
        (middle.left, middle.top),
        (100 - 66, 50 - 66),
        "centred on the pointer"
    );
    assert_eq!(
        (middle.picture_left, middle.picture_top),
        ((5 - 100) * 12, (5 - 50) * 12),
        "pixel 100 lies in the fifth of eleven cells"
    );
    assert_eq!(
        (middle.picture_width, middle.picture_height),
        (1920 * 12, 1080 * 12)
    );
    let corner = loupe((0.0, 0.0));
    assert_eq!(
        (corner.picture_left, corner.picture_top),
        (5 * 12, 5 * 12),
        "pixel 0 in the centre cell leaves five cells before it empty"
    );
    assert_eq!(loupe_at((1920, 1080), (1920, 1080), (1920.0, 5.0)), None);
    assert_eq!((SIDE, GRID, CELL), (132, 11, 12));
}

fn screens(names: &[&str]) -> EyedropIn {
    EyedropIn::Asked(Some((
        AskId(1),
        names.iter().map(|name| (*name).to_owned()).collect(),
    )))
}

fn arrives(name: &str, width: u32) -> EyedropIn {
    EyedropIn::Picture(name.to_owned(), OutputPicture::Shot(shot(width, 1)))
}

#[test]
fn a_question_takes_a_picture_of_every_screen_once_and_ends_with_the_question() {
    let (held, out) = eyedrop_step(None, screens(&["DP-1", "eDP-1"]));
    assert_eq!(
        out,
        vec![
            EyedropOut::Capture("DP-1".to_owned()),
            EyedropOut::Capture("eDP-1".to_owned())
        ]
    );
    let (held, out) = eyedrop_step(held, screens(&["DP-1", "eDP-1"]));
    assert_eq!(out, vec![], "the same question is not asked again");
    let (held, out) = eyedrop_step(held, arrives("DP-1", 2));
    assert_eq!(out, vec![]);
    let (held, out) = eyedrop_step(held, arrives("DP-1", 3));
    assert_eq!(out, vec![], "a picture nobody waits for is dropped");
    let held = held.expect("held");
    assert_eq!(held.shots.get("DP-1").map(|shot| shot.size), Some((2, 1)));
    let (gone, out) = eyedrop_step(Some(held), EyedropIn::Asked(None));
    assert_eq!((gone, out), (None, vec![]), "the question ended");
}

#[test]
fn a_question_nobody_can_point_into_is_cancelled() {
    let failed = |name: &str| EyedropIn::Picture(name.to_owned(), OutputPicture::Failed);
    let (empty, out) = eyedrop_step(None, screens(&[]));
    assert_eq!(out, vec![EyedropOut::Cancel(AskId(1))], "no screens");
    let (held, _) = eyedrop_step(empty, EyedropIn::Asked(None));
    assert_eq!(held, None);

    let (held, _) = eyedrop_step(None, screens(&["DP-1", "eDP-1"]));
    let (held, out) = eyedrop_step(held, failed("DP-1"));
    assert_eq!(out, vec![], "one screen is still to come");
    let (held, out) = eyedrop_step(held, failed("eDP-1"));
    assert_eq!(
        out,
        vec![EyedropOut::Cancel(AskId(1))],
        "every picture failed"
    );
    assert!(held.is_some_and(|held| held.shots.is_empty()));

    let (held, _) = eyedrop_step(None, screens(&["DP-1", "eDP-1"]));
    let (held, _) = eyedrop_step(held, failed("DP-1"));
    let (_, out) = eyedrop_step(held, arrives("eDP-1", 2));
    assert_eq!(out, vec![], "one picture is enough to point into");
}

#[test]
fn a_picture_with_no_question_open_is_dropped() {
    let (held, out) = eyedrop_step(None, arrives("DP-1", 2));
    assert_eq!((held, out), (None, vec![]));
}
