//! The Sound module: a `SectionHeader` naming the output device (`Volume::sink`) and a volume
//! quire `Slider` with mute, through `VolumeCommand::{SetLevel, SetMute}` (the bar's volume
//! panel's controls). Choosing another output device needs a sink list and a `SetSink` the
//! volume service does not have; it is left for later (FINDINGS "M5 freeze"). Reads
//! `use_volume`.

use dioxus::prelude::*;

/// The Sound module.
#[component]
pub fn SoundModule() -> Element {
    todo!("M5 fill: control-center (c)")
}
