//! `switcher.*` (design/22 §3.11; design/13 §13.3.5): the app switcher. The section and its keys
//! live in `ds-settings` (`schema/behaviour.rs`), beside the machine's timing in
//! `ds-behaviour`, so the shell and the compositor read one definition; here it is only
//! re-exported.

pub use ds_settings::schema::SwitcherSettings;
