//! `hot_corners.*`: what each screen corner does when the pointer rests in it (design/20 §1.16;
//! macOS "Desktop & Dock" > Hot Corners, hence `Page::Dock`). The section and its keys live in
//! `ds-settings` (`schema/behaviour.rs`), beside the machine's timing in `ds-behaviour`, so the
//! shell and the compositor read one definition; here it is only found in a [`ShellFile`].

use crate::file::ShellFile;
use crate::section::Section;

pub use ds_settings::schema::{CornerAction, HotCornerSettings};

impl Section for HotCornerSettings {
    const KEY: &'static str = "hot_corners";

    fn of(file: &ShellFile) -> &Self {
        &file.hot_corners
    }
}
