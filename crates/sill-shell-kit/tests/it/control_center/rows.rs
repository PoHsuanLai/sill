//! `Row`'s settings-height specimens: each accessory, a row with no leading element or detail, a
//! disabled row, and a list of three networks in both schemes.

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::base::vocab::RowState;
use ds::components::content::text_runs::RunTone;
use ds::components::lists::row::size::RowSize;
use ds::prelude::*;

/// Which specimen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowCase {
    None,
    CheckOn,
    CheckOff,
    Toggle,
    Chevron,
    Value,
    Glyph,
    Bare,
    Disabled,
    Networks(Theme),
}

#[derive(Props, Clone, PartialEq)]
pub struct RowProps {
    pub case: RowCase,
}

pub const CASES: [(RowCase, &str); 11] = [
    (RowCase::None, "row-none"),
    (RowCase::CheckOn, "row-check-on"),
    (RowCase::CheckOff, "row-check-off"),
    (RowCase::Toggle, "row-toggle"),
    (RowCase::Chevron, "row-chevron"),
    (RowCase::Value, "row-value"),
    (RowCase::Glyph, "row-glyph"),
    (RowCase::Bare, "row-bare"),
    (RowCase::Disabled, "row-disabled"),
    (RowCase::Networks(Theme::Light), "rows-networks-light"),
    (RowCase::Networks(Theme::Dark), "rows-networks-dark"),
];

/// The trailing mark a single-row case ends in.
fn mark(case: RowCase) -> Accessory {
    match case {
        RowCase::CheckOn => Accessory::Check(Check::On),
        RowCase::CheckOff => Accessory::Check(Check::Off),
        RowCase::Toggle => Accessory::Toggle {
            value: Check::On,
            on_toggle: EventHandler::new(|_| {}),
        },
        RowCase::Chevron => Accessory::Chevron,
        RowCase::Value => Accessory::Text("84%".to_string()),
        RowCase::Glyph => Accessory::Glyph(Icon::Lock),
        RowCase::None | RowCase::Bare | RowCase::Disabled | RowCase::Networks(_) => Accessory::None,
    }
}

/// The three networks a Wi-Fi detail lists: the one in use checked, a secured one, an open one.
fn networks() -> Element {
    let item = |title: &'static str, row: Element| ListItem::row(title, title, row);
    rsx! {
        List::<&'static str> {
            label: "Networks",
            items: vec![
                item("Home", rsx! { Row { leading: RowLeading::Icon(Icon::Wifi), title: "Home", detail: TextLine::from("Connected"), accessory: Accessory::Check(Check::On), size: RowSize::Settings, onclick: |_| {} } }),
                item("Studio 5G", rsx! { Row { leading: RowLeading::Icon(Icon::WifiHigh), title: "Studio 5G", accessory: Accessory::Glyph(Icon::Lock), size: RowSize::Settings, onclick: |_| {} } }),
                item("Café Guest", rsx! {
                    Row {
                        leading: RowLeading::Icon(Icon::WifiLow),
                        title: TextLine::Runs(vec![TextRun::new("Café ", RunTone::Plain), TextRun::new("Guest", RunTone::Faint)]),
                        accessory: Accessory::Check(Check::Off),
                        size: RowSize::Settings,
                        onclick: |_| {},
                    }
                }),
            ],
        }
    }
}

/// A specimen in a Popover root.
pub fn row(props: RowProps) -> Element {
    let theme = match props.case {
        RowCase::Networks(theme) => theme,
        _ => Theme::Light,
    };
    let body = match props.case {
        RowCase::Networks(_) => networks(),
        RowCase::Bare => rsx! {
            Row { title: "Show in menu bar", size: RowSize::Settings, onclick: |_| {} }
        },
        RowCase::Disabled => rsx! {
            Row {
                leading: RowLeading::Icon(Icon::Bluetooth),
                title: "Keyboard",
                detail: TextLine::from("Not connected"),
                accessory: Accessory::Chevron,
                state: RowState { availability: Availability::Disabled, ..RowState::default() },
                size: RowSize::Settings,
                onclick: |_| {},
            }
        },
        case => rsx! {
            Row { leading: RowLeading::Icon(Icon::Headphones), title: "Headphones", detail: TextLine::from("Battery 84%"), accessory: mark(case), size: RowSize::Settings, onclick: |_| {} }
        },
    };
    rsx! {
        Ds {
            appearance: Appearance { theme, ..Appearance::default() },
            material: Material::Popover,
            stylesheet: Inject::Host,
            {body}
        }
    }
}
