//! Which page the control center shows and how it moves between them (design/13 §13.3.7
//! "Detail": a tile's chevron opens its detail pane in place, `slide-l` in and `slide-r` back),
//! pure.

/// A module with a detail page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Detail {
    /// The Wi-Fi network list (`pages::wifi_networks::WifiPane`).
    Wifi,
    /// The Bluetooth device list (`pages::bluetooth_devices::BluetoothPane`).
    Bluetooth,
}

/// The page on show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CcPage {
    /// The module grid.
    #[default]
    Modules,
    Detail(Detail),
}

/// What moves the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageInput {
    /// A tile's chevron.
    Open(Detail),
    /// A detail page's back button.
    Back,
    /// The Escape key.
    Escape,
}

/// Which way the change slides (quire `Anim::SlideL` / `Anim::SlideR`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slide {
    /// No page change.
    None,
    /// Into a detail page: `slide-l`.
    ToDetail,
    /// Back to the grid: `slide-r`.
    ToModules,
}

/// Whether the panel stays open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageOut {
    Stay,
    /// Escape on the grid: the popup closes.
    Close,
}

/// `page` after `input`: Open from the grid slides to the detail (from a detail page it switches
/// with no slide); Back and Escape from a detail slide to the grid; Back on the grid does
/// nothing; Escape on the grid closes.
pub fn step_page(page: CcPage, input: PageInput) -> (CcPage, Slide, PageOut) {
    match (page, input) {
        (CcPage::Modules, PageInput::Open(detail)) => {
            (CcPage::Detail(detail), Slide::ToDetail, PageOut::Stay)
        }
        (CcPage::Detail(_), PageInput::Open(detail)) => {
            (CcPage::Detail(detail), Slide::None, PageOut::Stay)
        }
        (CcPage::Detail(_), PageInput::Back | PageInput::Escape) => {
            (CcPage::Modules, Slide::ToModules, PageOut::Stay)
        }
        (CcPage::Modules, PageInput::Back) => (CcPage::Modules, Slide::None, PageOut::Stay),
        (CcPage::Modules, PageInput::Escape) => (CcPage::Modules, Slide::None, PageOut::Close),
    }
}

#[cfg(test)]
mod tests {
    use super::{CcPage, Detail, PageInput, PageOut, Slide, step_page};

    #[test]
    fn pages_slide_in_and_back_and_escape_unwinds() {
        use CcPage::{Detail as D, Modules};
        use Detail::{Bluetooth, Wifi};
        use PageInput::{Back, Escape, Open};
        let cases: &[(CcPage, PageInput, (CcPage, Slide, PageOut))] = &[
            (
                Modules,
                Open(Wifi),
                (D(Wifi), Slide::ToDetail, PageOut::Stay),
            ),
            (D(Wifi), Back, (Modules, Slide::ToModules, PageOut::Stay)),
            (
                D(Bluetooth),
                Escape,
                (Modules, Slide::ToModules, PageOut::Stay),
            ),
            (Modules, Escape, (Modules, Slide::None, PageOut::Close)),
            (Modules, Back, (Modules, Slide::None, PageOut::Stay)),
            (
                D(Wifi),
                Open(Bluetooth),
                (D(Bluetooth), Slide::None, PageOut::Stay),
            ),
        ];
        for &(page, input, want) in cases {
            assert_eq!(step_page(page, input), want, "{page:?} {input:?}");
        }
    }
}
