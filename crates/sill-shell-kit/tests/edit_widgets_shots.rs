//! Pictures of Edit Widgets as the person meets it, for the progress
//! page: a 1280 x 800 desktop over a wallpaper-like wash, its widgets at their cells from the
//! right, and the gallery in the bottom sheet. Posed on the virtual clock: opened; a moment after
//! Add to Desktop (the new widget on the desktop, the button's check drawing, the new row
//! rising); the check held; a Remove's card halfway through its exit; and the Batteries row with
//! one to four devices. Each render must paint; the pictures are written only when
//! `QUIRE_GALLERY_SHOTS` names a directory, as `edit-widgets-<pose>.png`.

use dioxus::prelude::*;
use ds::components::content::status::battery_state::{BatteryPower, BatteryState, LowAt};
use ds::components::overlays::sheet_attach::Attach;
use ds::components::overlays::sheet_width::SheetWidth;
use ds::prelude::*;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Viewport};
use sill_shell_kit::battery::device_glyph::Device;
use sill_shell_kit::prelude::*;
use sill_shell_kit::tokens::widgets::WidgetMetrics;
use sill_shell_kit::widget::battery::{BatteryCell, BatteryEntry, BatteryWidget};
use sill_shell_kit::widget::calendar::MonthWidget;
use sill_shell_kit::widget::clock::WorldClockWidget;
use sill_shell_kit::widget::timeline::Timeline;
use sill_shell_kit::widget::{
    DesktopGrid, WidgetAt, WidgetEdit, WidgetLayout, WidgetPlacement, apply,
};
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

const GRID: DesktopGrid = DesktopGrid {
    columns: 7,
    rows: 2,
};

const PITCH: u16 = 164 + 16;
const WASH: &str = "linear-gradient(135deg,#9fc7c4,#e6c79a 55%,#e48f7a)";

fn opening() -> WidgetLayout {
    [
        (
            BatteryWidget::kind(),
            WidgetSize::Small,
            WidgetHost::Desktop,
        ),
        (
            WorldClockWidget::kind(),
            WidgetSize::Medium,
            WidgetHost::Desktop,
        ),
        (MonthWidget::kind(), WidgetSize::Large, WidgetHost::Tile),
    ]
    .into_iter()
    .fold(WidgetLayout::default(), |layout, (kind, size, host)| {
        apply(layout.clone(), WidgetEdit::Add { kind, size, host }, GRID).unwrap_or(layout)
    })
}

fn removed(before: &WidgetLayout, after: &WidgetLayout) -> Vec<WidgetPlacement> {
    before
        .items()
        .iter()
        .filter(|item| item.at.host() == WidgetHost::Desktop)
        .filter(|item| after.items().iter().all(|kept| kept.id != item.id))
        .cloned()
        .collect()
}

#[allow(non_snake_case)]
fn Stage() -> Element {
    let mut layout = use_signal(opening);
    let mut leaving = use_signal(Vec::<WidgetPlacement>::new);
    let onedit = move |edit: WidgetEdit| {
        let before = layout();
        if let Ok(next) = apply(before.clone(), edit, GRID) {
            leaving.with_mut(|going| going.extend(removed(&before, &next)));
            layout.set(next);
        }
    };
    let metrics = WidgetMetrics::default().style_attr();
    let placed: Vec<WidgetPlacement> = layout()
        .items()
        .iter()
        .filter(|item| item.at.host() == WidgetHost::Desktop)
        .cloned()
        .collect();
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Widget, chrome: Some(RootChrome::Transparent), extent: RootExtent::Viewport,
            div { style: "position:absolute;inset:0;background:{WASH};{metrics}",
                for item in placed {
                    Card { key: "{item.id.0}", item, shown: Shown::Visible, on_hidden: |()| {} }
                }
                for item in leaving() {
                    Card { key: "{item.id.0}", item: item.clone(), shown: Shown::Hidden,
                        on_hidden: move |()| leaving.with_mut(|going| going.retain(|gone| gone.id != item.id)) }
                }
            }
            Sheet { label: "Edit Widgets", onclose: |_| {}, attach: Attach::Bottom, width: SheetWidth::Wide,
                div { style: "display:flex;flex-direction:column;height:380px;min-height:0;{metrics}",
                    WidgetGallery { layout: layout(), onedit }
                }
            }
        }
    }
}

#[component]
fn Card(item: WidgetPlacement, shown: Shown, on_hidden: EventHandler<()>) -> Element {
    let WidgetAt::Desktop(cell) = item.at else {
        return rsx! {};
    };
    let (left, top) = (
        1280 - 8 - (7 - cell.column) * PITCH + 16,
        8 + cell.row * PITCH,
    );
    let size = item.size;
    let on_hidden = Some(on_hidden);
    let card = match item.kind.as_str() {
        "quire.battery" => rsx! {
            WidgetCard { widget: BatteryWidget, timeline: Timeline::now(BatteryWidget::preview(size)), size, shown, on_hidden }
        },
        "quire.world-clock" => rsx! {
            WidgetCard { widget: WorldClockWidget, timeline: Timeline::now(WorldClockWidget::preview(size)), size, shown, on_hidden }
        },
        "quire.month" => rsx! {
            WidgetCard { widget: MonthWidget, timeline: Timeline::now(MonthWidget::preview(size)), size, shown, on_hidden }
        },
        _ => rsx! {},
    };
    rsx! {
        div { style: "position:absolute;left:{left}px;top:{top}px", {card} }
    }
}

/// How many batteries the row shows.
static COUNT: GlobalSignal<usize> = Signal::global(|| 1);

#[allow(non_snake_case)]
fn Row() -> Element {
    let devices = [
        Device::Laptop,
        Device::Phone,
        Device::Headphones,
        Device::Mouse,
    ];
    let cells = devices
        .into_iter()
        .take(COUNT())
        .enumerate()
        .map(|(at, device)| BatteryCell {
            name: format!("device {at}"),
            device,
            state: BatteryState {
                level: Fraction(900 - 200 * at as u16),
                power: BatteryPower::Battery,
                low_at: LowAt::default(),
            },
        })
        .collect();
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Widget, chrome: Some(RootChrome::Transparent),
            div { style: "box-sizing:border-box;padding:12px;width:368px;height:188px;background:{WASH};{WidgetMetrics::default().style_attr()}",
                WidgetCard { widget: BatteryWidget, timeline: Timeline::now(BatteryEntry::Devices(cells)), size: WidgetSize::Medium }
            }
        }
    }
}

fn save(harness: &mut Harness, name: &str) {
    let shot = harness.render().expect("renders");
    assert!(shot.width() > 0, "{name}");
    if let Ok(dir) = std::env::var("QUIRE_GALLERY_SHOTS") {
        shot.save(format!("{dir}/edit-widgets-{name}.png"))
            .expect("writes the shot");
    }
}

#[test]
fn edit_widgets_paints_each_pose() {
    let mut harness = Harness::new(Stage, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(1500));
    save(&mut harness, "opened");

    let clock = harness
        .centre(".ds-widget-gallery .ds-list-item:nth-child(3) .ds-row")
        .expect("Calendar in the list");
    harness.send(Input::click(clock));
    harness.advance(Duration::from_millis(50));
    let add = harness
        .centre(".ds-widget-gallery-actions .ds-button")
        .expect("Add to Desktop");
    harness.send(Input::click(add));
    harness.advance(Duration::from_millis(120));
    save(&mut harness, "added-moment");
    harness.advance(Duration::from_millis(400));
    save(&mut harness, "added-check");
    harness.advance(Duration::from_millis(1500));

    let remove = harness
        .centre(".ds-widget-gallery-surface[*|data-host=desktop] .ds-widget-gallery-row .ds-button")
        .expect("Remove");
    harness.send(Input::click(remove));
    harness.advance(Duration::from_millis(200));
    save(&mut harness, "removed-leaving");
    harness.advance(Duration::from_millis(540));
    save(&mut harness, "removed-gone");

    let view = Viewport {
        width: 368,
        height: 188,
        scale_percent: 200,
    };
    let mut row = Harness::new(Row, HarnessConfig::new(view).with_clock(Clock::Virtual));
    for count in 1..=4 {
        row.within(|| *COUNT.write() = count);
        row.advance(Duration::from_millis(1200));
        save(&mut row, &format!("batteries-row-{count}"));
    }
}
