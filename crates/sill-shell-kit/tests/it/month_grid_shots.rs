//! Snapshots of the compact MonthGrid in a small desktop widget (design/04-COMPONENTS.md section
//! 39): a five-week month (September 2026 from Monday) and a six-week one (August 2026), light
//! and dark, at 1x and 4x, over a wallpaper-like wash as sill's desktop shows it. Each render
//! must paint; the pictures are written only when `QUIRE_CAL_SHOTS` names a directory, as
//! `calendar-compact-<tag>-<weeks>-<scheme>-<scale>.png` with `<tag>` from `QUIRE_CAL_TAG`
//! (default `now`), for the progress gallery.

use crate::support::month_sample;

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::{Clock, Harness, HarnessConfig, Viewport};
use month_sample::{AUGUST, First, SEPTEMBER, month};
use sill_shell_kit::month_grid::data::{DayKey, MonthKey, MonthStep};
use sill_shell_kit::prelude::*;
use sill_shell_kit::tokens::widgets::WidgetMetrics;
use std::time::Duration;

/// A shot's component.
type App = fn() -> Element;

/// The widget's 164 px card and a 12 px margin of wallpaper round it.
const SIDE: u32 = 188;

/// A day of 2026.
const fn day(month: i8, day: i8) -> DayKey {
    DayKey {
        year: 2026,
        month,
        day,
    }
}

/// One month's widget in one scheme: `SIX` picks August (six weeks) over September (five).
#[allow(non_snake_case)]
fn Shot<const DARK: bool, const SIX: bool>() -> Element {
    let theme = if DARK { Theme::Dark } else { Theme::Light };
    let (key, name, today, busy): (MonthKey, &str, DayKey, [DayKey; 3]) = if SIX {
        (
            AUGUST,
            "August",
            day(8, 14),
            [day(8, 5), day(8, 14), day(9, 3)],
        )
    } else {
        (
            SEPTEMBER,
            "September",
            day(9, 26),
            [day(9, 9), day(9, 26), day(10, 2)],
        )
    };
    let mut data = month(key, First::Monday, today, &busy);
    data.title = TextLine::from(name);
    let wash = if DARK {
        "linear-gradient(135deg,#2a3b5c,#5a3a58 55%,#7a4a3a)"
    } else {
        "linear-gradient(135deg,#9fc7c4,#e6c79a 55%,#e48f7a)"
    };
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance { theme, ..Appearance::default() }, material: Material::Widget, chrome: Some(RootChrome::Transparent),
            div { style: "box-sizing:border-box;padding:12px;width:188px;height:188px;background:{wash}",
                div { style: WidgetMetrics::default().style_attr(),
                    WidgetFrame { size: WidgetSize::Small,
                        MonthGrid { data, onstep: move |_: MonthStep| {} }
                    }
                }
            }
        }
    }
}

#[test]
fn the_compact_widget_paints_at_1x_and_4x() {
    let out = std::env::var("QUIRE_CAL_SHOTS").ok();
    let tag = std::env::var("QUIRE_CAL_TAG").unwrap_or_else(|_| "now".to_owned());
    let cases: [(App, &str, &str); 4] = [
        (Shot::<false, false>, "5wk", "light"),
        (Shot::<true, false>, "5wk", "dark"),
        (Shot::<false, true>, "6wk", "light"),
        (Shot::<true, true>, "6wk", "dark"),
    ];
    for (app, weeks, scheme) in cases {
        for scale in [1u16, 4] {
            let view = Viewport {
                width: SIDE,
                height: SIDE,
                scale_percent: scale * 100,
            };
            let mut harness =
                Harness::new(app, HarnessConfig::new(view).with_clock(Clock::Virtual));
            harness.advance(Duration::from_millis(50));
            let shot = harness.render().expect("renders");
            assert_eq!(shot.width(), SIDE * u32::from(scale), "{weeks} {scheme}");
            if let Some(dir) = &out {
                let path = format!("{dir}/calendar-compact-{tag}-{weeks}-{scheme}-{scale}x.png");
                shot.save(&path).expect("writes the shot");
            }
        }
    }
}
