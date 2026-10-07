//! The vocabulary: a rectangle, an edge zone, one output's work area, and what the service
//! publishes.

/// An axis-aligned rectangle in the compositor's logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// The edge of an output a panel keeps clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// A strip along one edge that a panel reserves (its exclusive zone), in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zone {
    pub edge: Edge,
    pub px: u32,
}

/// One output as an app sees it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkArea {
    /// The shell's id for the output; it lasts while the output is connected.
    pub id: u32,
    /// The connector name (`DP-1`); empty when the compositor sent none.
    pub name: String,
    /// The whole output, in logical pixels.
    pub frame: Rect,
    /// The part a window may use: `frame` less the panels' zones.
    pub work: Rect,
    /// Device pixels per logical pixel, in 120ths (120 is 1x, 180 is 1.5x).
    pub scale: u32,
}

/// Why the bus name is not served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkTrouble {
    /// No session bus.
    NoBus,
    /// Another process owns the name.
    NameTaken,
    /// The bus refused, with its reason.
    Refused(String),
}

/// Whether the work areas are on the bus.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WorkAreaLink {
    /// Not told to serve yet: nothing claims the name.
    #[default]
    Passive,
    Serving,
    Unavailable(WorkTrouble),
}

/// What the service publishes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkAreas {
    /// One per connected output, in the host's order.
    pub areas: Vec<WorkArea>,
    pub link: WorkAreaLink,
}
