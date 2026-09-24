use super::{
    Chromaticity, Edid, EdidError, Manufacturer, Millimetres, Mode, Pixels, Primaries, ProductCode,
    Serial, parse,
};

/// The MA270U's base block as its HDMI connector reports it, with the serial string replaced
/// by `SERIAL0000001` and the checksum fixed up.
pub(crate) const MA270U: &[u8; 128] = include_bytes!("ma270u.bin");

/// A base block with the given primaries (thousandths), size in cm, a 1920x1080 timing of
/// `timing_mm` and no descriptors but that timing.
pub(crate) fn block(primaries: [(u16, u16); 4], cm: (u8, u8), timing_mm: (u16, u16)) -> [u8; 128] {
    let mut b = [0u8; 128];
    b[..8].copy_from_slice(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
    // "DEL", product 0xA0B1, serial 7.
    b[8] = 0x10;
    b[9] = 0xAC;
    b[10] = 0xB1;
    b[11] = 0xA0;
    b[12] = 7;
    b[18] = 1;
    b[19] = 4;
    b[21] = cm.0;
    b[22] = cm.1;
    let tenbit = |thousandths: u16| ((u32::from(thousandths) * 1024 + 500) / 1000) as u16;
    let points: Vec<(u16, u16)> = primaries
        .iter()
        .map(|&(x, y)| (tenbit(x), tenbit(y)))
        .collect();
    let low = |i: usize| ((points[i].0 & 3) << 2 | (points[i].1 & 3)) as u8;
    b[25] = low(0) << 4 | low(1);
    b[26] = low(2) << 4 | low(3);
    for (i, (x, y)) in points.iter().enumerate() {
        b[27 + i * 2] = (x >> 2) as u8;
        b[28 + i * 2] = (y >> 2) as u8;
    }
    // 1920x1080 at 148.5 MHz, 2200x1125 total: 60 Hz.
    let d = &mut b[54..72];
    d[0] = 0x02;
    d[1] = 0x3A;
    d[2] = 0x80;
    d[3] = 0x18;
    d[4] = 0x71;
    d[5] = 0x38;
    d[6] = 0x2D;
    d[7] = 0x40;
    d[12] = (timing_mm.0 & 0xFF) as u8;
    d[13] = (timing_mm.1 & 0xFF) as u8;
    d[14] = ((timing_mm.0 >> 8) as u8) << 4 | (timing_mm.1 >> 8) as u8;
    let sum = b[..127].iter().fold(0u8, |s, v| s.wrapping_add(*v));
    b[127] = 0u8.wrapping_sub(sum);
    b
}

/// sRGB/Rec.709 primaries and D65, in thousandths.
pub(crate) const SRGB: [(u16, u16); 4] = [(640, 330), (300, 600), (150, 60), (313, 329)];

#[test]
fn the_ma270u_block_parses_to_what_the_panel_is() {
    let point = |x, y| Chromaticity { x, y };
    let want = Edid {
        manufacturer: Manufacturer("BNQ".to_owned()),
        product: ProductCode(0x8074),
        serial: Serial::Text("SERIAL0000001".to_owned()),
        name: "BenQ MA270U".to_owned(),
        size: Some(Millimetres {
            width: 597,
            height: 336,
        }),
        native: Some(Mode {
            pixels: Pixels {
                width: 3840,
                height: 2160,
            },
            refresh_mhz: 59996,
        }),
        primaries: Primaries {
            red: point(697, 324),
            green: point(262, 696),
            blue: point(156, 63),
            white: point(321, 337),
        },
    };
    assert_eq!(parse(MA270U), Ok(want));
}

#[test]
fn the_ma270u_primaries_read_as_the_panel_reports_them() {
    let p = parse(MA270U).unwrap_or_else(|e| panic!("{e}")).primaries;
    let got = [p.red, p.green, p.blue, p.white].map(|c| c.thousandths());
    assert_eq!(got, [(681, 316), (256, 680), (152, 62), (313, 329)]);
}

#[test]
fn sizes_fall_back_from_the_timing_to_the_base_block_to_none() {
    /// (label, base-block cm, timing mm, the size expected).
    type Case = (&'static str, (u8, u8), (u16, u16), Option<(u32, u32)>);
    let cases: &[Case] = &[
        (
            "timing millimetres win",
            (53, 30),
            (527, 296),
            Some((527, 296)),
        ),
        (
            "base centimetres otherwise",
            (53, 30),
            (0, 0),
            Some((530, 300)),
        ),
        ("a projector reports nothing", (0, 0), (0, 0), None),
        ("an aspect ratio is not a size", (0, 79), (0, 0), None),
    ];
    for &(label, cm, mm, want) in cases {
        let edid = parse(&block(SRGB, cm, mm)).unwrap_or_else(|e| panic!("{label}: {e}"));
        let want = want.map(|(width, height)| Millimetres { width, height });
        assert_eq!(edid.size, want, "{label}");
        assert_eq!(edid.manufacturer.0, "DEL", "{label}");
        assert_eq!(edid.serial, Serial::Number(7), "{label}");
        assert_eq!(
            edid.native
                .map(|m| (m.pixels.width, m.pixels.height, m.refresh_mhz)),
            Some((1920, 1080, 60000)),
            "{label}"
        );
    }
}

#[test]
fn bad_bytes_are_refused_with_their_reason() {
    let mut bad_sum = *MA270U;
    bad_sum[40] ^= 1;
    let mut bad_header = *MA270U;
    bad_header[0] = 1;
    let cases: Vec<(&str, Vec<u8>, EdidError)> = vec![
        ("empty", Vec::new(), EdidError::Short),
        ("short", MA270U[..100].to_vec(), EdidError::Short),
        ("header", bad_header.to_vec(), EdidError::Header),
        ("checksum", bad_sum.to_vec(), EdidError::Checksum),
    ];
    for (label, bytes, want) in cases {
        assert_eq!(parse(&bytes), Err(want), "{label}");
    }
}
