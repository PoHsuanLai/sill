//! The control center's detail pages (design/13 §13.3.7 "Detail"): each a quire `PaneHeader`
//! (a back button, the title, the feature's switch) over its list and a `PaneFooter`, sliding in
//! over the grid.

pub mod bluetooth_devices;
pub mod wifi_networks;

mod feature_switch;
pub(crate) mod foot;
