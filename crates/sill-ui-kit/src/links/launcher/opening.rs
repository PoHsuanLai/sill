//! One opening of the launcher: where, with what already typed, and since when.

use std::time::Instant;

use shell_host::OutputId;

use crate::links::{OpenSerial, Opened, PanelFocus};

/// One opening of the launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    /// The output whose panel shows.
    pub output: OutputId,
    /// The query the field starts with (`sill launcher open --query`).
    pub query: String,
    /// Which opening this is: a new one remounts the panel's contents (the entrance plays,
    /// the field is focused and the selection starts at the top).
    pub serial: OpenSerial,
    /// When the request arrived, for `open_latency_ms`.
    pub requested: Instant,
    /// Whether the panel has had keyboard focus since it opened.
    pub focus: PanelFocus,
}

impl Opened for Opening {
    fn output(&self) -> OutputId {
        self.output
    }

    fn serial(&self) -> OpenSerial {
        self.serial
    }

    fn focus(&self) -> PanelFocus {
        self.focus
    }

    fn focused(self) -> Self {
        Opening {
            focus: PanelFocus::Held,
            ..self
        }
    }
}
