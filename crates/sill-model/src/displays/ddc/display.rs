//! A monitor that answers DDC/CI: where it answers, and its controls as last read.

use serde::{Deserialize, Serialize};

use super::vcp::Controls;
use crate::ids::{DdcId, OutputName};

/// A display a probe found answering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub id: DdcId,
    /// The DRM connector whose EDID matched the one this bus returned.
    pub connector: Option<OutputName>,
    /// The EDID product name.
    pub product: String,
}

/// One DDC display and its cached controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DdcDisplay {
    pub endpoint: Endpoint,
    pub controls: Controls,
}
