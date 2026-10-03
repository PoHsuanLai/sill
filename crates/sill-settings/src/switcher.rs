//! `switcher.*` (design/22 §3.11; design/13 §13.3.5): the app switcher. The section and its keys
//! live in `ds-settings` (`schema/behaviour.rs`), beside the machine's timing in
//! `ds-behaviour`, so the shell and the compositor read one definition; here it is only found
//! in a [`ShellFile`].

use crate::file::ShellFile;
use crate::section::Section;

pub use ds_settings::schema::SwitcherSettings;

impl Section for SwitcherSettings {
    const KEY: &'static str = "switcher";

    fn of(file: &ShellFile) -> &Self {
        &file.switcher
    }
}
