use super::{DisplayScale, Ppi, ScaleRounding, default_scale, density};
use crate::displays::edid::{Millimetres, Pixels};

/// The panel size of a `diagonal_in` display whose pixels are square, to the millimetre.
fn size_of(diagonal_in: f64, width: u32, height: u32) -> Millimetres {
    let diagonal_px = f64::from(width).hypot(f64::from(height));
    let mm_per_px = diagonal_in * 25.4 / diagonal_px;
    Millimetres {
        width: (f64::from(width) * mm_per_px).round() as u32,
        height: (f64::from(height) * mm_per_px).round() as u32,
    }
}

/// (label, pixels, size, expected ppi, expected scale). The ppi column is the arithmetic the
/// scale follows from, so a failing row shows which half moved.
type Case = (&'static str, Pixels, Option<Millimetres>, Option<u16>, u16);

/// `display.scale_rounding = "nearest"`: the nearest step, pulled to an integer within 0.15 -
/// sill's original behaviour, unchanged by `ScaleRounding`.
#[test]
fn every_common_display_gets_its_nearest_default_scale() {
    let px = |width, height| Pixels { width, height };
    let cases: Vec<Case> = vec![
        // raw 1.497: nearest step.
        (
            "27in 4K",
            px(3840, 2160),
            Some(size_of(27.0, 3840, 2160)),
            Some(163),
            150,
        ),
        // The MA270U's own EDID millimetres (597 x 336): raw 1.50.
        (
            "BenQ MA270U",
            px(3840, 2160),
            Some(Millimetres {
                width: 597,
                height: 336,
            }),
            Some(163),
            150,
        ),
        // raw 1.996: the integer.
        (
            "27in 5K",
            px(5120, 2880),
            Some(size_of(27.0, 5120, 2880)),
            Some(218),
            200,
        ),
        // raw 0.84: below every step, so 1.
        (
            "24in 1080p",
            px(1920, 1080),
            Some(size_of(24.0, 1920, 1080)),
            Some(92),
            100,
        ),
        // raw 2.08: the integer.
        (
            "13.3in 2560x1600",
            px(2560, 1600),
            Some(size_of(13.3, 2560, 1600)),
            Some(227),
            200,
        ),
        // raw 2.23: outside the integer pull, but 2 is still the nearest step (2.5 is 0.27 away).
        (
            "14in 2880x1800",
            px(2880, 1800),
            Some(size_of(14.0, 2880, 1800)),
            Some(242),
            200,
        ),
        // raw 1.26: nearest step.
        (
            "32in 4K",
            px(3840, 2160),
            Some(size_of(32.0, 3840, 2160)),
            Some(138),
            125,
        ),
        // raw 1.00.
        (
            "27in 1440p",
            px(2560, 1440),
            Some(size_of(27.0, 2560, 1440)),
            Some(109),
            100,
        ),
        // raw 1.29: Windows' own 125% for these.
        (
            "15.6in 1080p",
            px(1920, 1080),
            Some(size_of(15.6, 1920, 1080)),
            Some(141),
            125,
        ),
        // raw 1.48.
        (
            "14in 1920x1200",
            px(1920, 1200),
            Some(size_of(14.0, 1920, 1200)),
            Some(162),
            150,
        ),
        // raw 1.73.
        (
            "16in 2560x1600",
            px(2560, 1600),
            Some(size_of(16.0, 2560, 1600)),
            Some(189),
            175,
        ),
        // raw 2.06.
        (
            "13.6in 2560x1664",
            px(2560, 1664),
            Some(size_of(13.6, 2560, 1664)),
            Some(224),
            200,
        ),
        // raw 1.27.
        (
            "34in 3440x1440 ultrawide",
            px(3440, 1440),
            Some(size_of(34.0, 3440, 1440)),
            Some(110),
            100,
        ),
        // raw 1.98: the integer.
        (
            "6K 32in",
            px(6016, 3384),
            Some(size_of(32.0, 6016, 3384)),
            Some(216),
            200,
        ),
        ("a projector at 1080p", px(1920, 1080), None, None, 100),
        ("a projector at 4K", px(3840, 2160), None, None, 200),
        // A TV whose EDID says 16 x 9 cm: 13 700 ppi is not a display.
        (
            "a 4K TV with a nonsense size",
            px(3840, 2160),
            Some(Millimetres {
                width: 160,
                height: 90,
            }),
            None,
            200,
        ),
    ];
    for (label, pixels, size, ppi, scale) in cases {
        assert_eq!(density(pixels, size).map(|p| p.0), ppi, "{label}: ppi");
        assert_eq!(
            default_scale(pixels, size, Ppi::APPLE_27, ScaleRounding::Nearest),
            DisplayScale(scale),
            "{label}: scale"
        );
    }
}

/// `display.scale_rounding = "prefer_whole"` (the default): below 1.5, the same answer as
/// `Nearest`; at or above it, a whole scale (1, 2 or 3) when that leaves at least
/// `MIN_LOGICAL_WIDTH` logical pixels across, else `Nearest`'s answer.
#[test]
fn every_common_display_gets_its_prefer_whole_default_scale() {
    let px = |width, height| Pixels { width, height };
    let cases: Vec<Case> = vec![
        // raw 1.497, rounds to 1.50: at the threshold, so the whole branch; 3840 / 2 = 1920.
        (
            "27in 4K",
            px(3840, 2160),
            Some(size_of(27.0, 3840, 2160)),
            Some(163),
            200,
        ),
        // The MA270U's own EDID millimetres (597 x 336): raw 1.499, same rounding.
        (
            "BenQ MA270U",
            px(3840, 2160),
            Some(Millimetres {
                width: 597,
                height: 336,
            }),
            Some(163),
            200,
        ),
        // raw 1.996: already whole under Nearest.
        (
            "27in 5K",
            px(5120, 2880),
            Some(size_of(27.0, 5120, 2880)),
            Some(218),
            200,
        ),
        // raw 0.84: below 1.5, Nearest's answer.
        (
            "24in 1080p",
            px(1920, 1080),
            Some(size_of(24.0, 1920, 1080)),
            Some(92),
            100,
        ),
        // raw 2.08: whole scale 2; 2560 / 2 = 1280, exactly MIN_LOGICAL_WIDTH.
        (
            "13.3in 2560x1600",
            px(2560, 1600),
            Some(size_of(13.3, 2560, 1600)),
            Some(227),
            200,
        ),
        // raw 2.23: whole scale 2 (Nearest's answer too); 2880 / 2 = 1440.
        (
            "14in 2880x1800",
            px(2880, 1800),
            Some(size_of(14.0, 2880, 1800)),
            Some(242),
            200,
        ),
        // raw 1.26: below 1.5, unchanged.
        (
            "32in 4K",
            px(3840, 2160),
            Some(size_of(32.0, 3840, 2160)),
            Some(138),
            125,
        ),
        // raw 1.00: below 1.5, unchanged.
        (
            "27in 1440p",
            px(2560, 1440),
            Some(size_of(27.0, 2560, 1440)),
            Some(109),
            100,
        ),
        // raw 1.29: below 1.5, unchanged.
        (
            "15.6in 1080p",
            px(1920, 1080),
            Some(size_of(15.6, 1920, 1080)),
            Some(141),
            125,
        ),
        // raw 1.48: below 1.5, unchanged (Nearest already lands on 1.5).
        (
            "14in 1920x1200",
            px(1920, 1200),
            Some(size_of(14.0, 1920, 1200)),
            Some(162),
            150,
        ),
        // raw 1.73: whole scale 2; 2560 / 2 = 1280, exactly MIN_LOGICAL_WIDTH.
        (
            "16in 2560x1600",
            px(2560, 1600),
            Some(size_of(16.0, 2560, 1600)),
            Some(189),
            200,
        ),
        // raw 2.06: whole scale 2 (Nearest's answer too).
        (
            "13.6in 2560x1664",
            px(2560, 1664),
            Some(size_of(13.6, 2560, 1664)),
            Some(224),
            200,
        ),
        // raw 1.01: below 1.5, unchanged.
        (
            "34in 3440x1440 ultrawide",
            px(3440, 1440),
            Some(size_of(34.0, 3440, 1440)),
            Some(110),
            100,
        ),
        // raw 1.98: whole scale 2 (Nearest's answer too).
        (
            "6K 32in",
            px(6016, 3384),
            Some(size_of(32.0, 6016, 3384)),
            Some(216),
            200,
        ),
        // raw ~1.84: whole scale 2 wants 2256 / 2 = 1128, under MIN_LOGICAL_WIDTH, so this
        // falls back to Nearest's 1.75.
        (
            "13.5in 2256x1504",
            px(2256, 1504),
            Some(size_of(13.5, 2256, 1504)),
            Some(201),
            175,
        ),
        // raw ~2.59: whole scale 3 (clamped); 3840 / 3 = 1280, exactly MIN_LOGICAL_WIDTH.
        (
            "15.6in 4K",
            px(3840, 2160),
            Some(size_of(15.6, 3840, 2160)),
            Some(283),
            300,
        ),
        ("a projector at 1080p", px(1920, 1080), None, None, 100),
        ("a projector at 4K", px(3840, 2160), None, None, 200),
        // A TV whose EDID says 16 x 9 cm: 13 700 ppi is not a display.
        (
            "a 4K TV with a nonsense size",
            px(3840, 2160),
            Some(Millimetres {
                width: 160,
                height: 90,
            }),
            None,
            200,
        ),
    ];
    for (label, pixels, size, ppi, scale) in cases {
        assert_eq!(density(pixels, size).map(|p| p.0), ppi, "{label}: ppi");
        assert_eq!(
            default_scale(pixels, size, Ppi::APPLE_27, ScaleRounding::PreferWhole),
            DisplayScale(scale),
            "{label}: scale"
        );
    }
}

#[test]
fn a_different_target_moves_the_answer() {
    let pixels = Pixels {
        width: 3840,
        height: 2160,
    };
    let size = Some(size_of(27.0, 3840, 2160));
    let cases: &[(u16, u16)] = &[(109, 150), (82, 200), (163, 100), (0, 200)];
    for &(target, want) in cases {
        assert_eq!(
            default_scale(pixels, size, Ppi(target), ScaleRounding::Nearest),
            DisplayScale(want),
            "target {target}"
        );
    }
}

#[test]
fn scales_print_and_parse_as_decimals() {
    const CASES: &[(u16, &str)] = &[
        (100, "1"),
        (125, "1.25"),
        (150, "1.5"),
        (200, "2"),
        (175, "1.75"),
        (105, "1.05"),
    ];
    for &(hundredths, text) in CASES {
        assert_eq!(DisplayScale(hundredths).decimal(), text);
        assert_eq!(
            DisplayScale::parse(text),
            Some(DisplayScale(hundredths)),
            "{text}"
        );
    }
    for bad in ["", "0", "1.255", "-1", "one", "1.x", ".5"] {
        assert_eq!(DisplayScale::parse(bad), None, "{bad:?}");
    }
    assert_eq!(DisplayScale::parse(" 1.50 "), Some(DisplayScale(150)));
}
