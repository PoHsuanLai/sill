//! `sill debug widget-drag`: a made-up press, move or release for one output's desktop layer, fed
//! to the same drag machine as its pointer (FINDINGS "Widget drag"). The nested compositor's
//! injection has a click but no separate press and release (shell-host's `Inject`), so a scripted
//! drag on it comes through here; every input is delivered, none coalesced.

use shell_host::OutputId;
use tokio::sync::broadcast;

use super::drag::DragInput;

/// One made-up input for the layer on `output`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragAsk {
    pub output: OutputId,
    pub input: DragInput,
}

/// The daemon's half and every layer's receiver. Clone is cheap.
#[derive(Debug, Clone)]
pub struct WidgetDragLink {
    asks: broadcast::Sender<DragAsk>,
}

impl Default for WidgetDragLink {
    fn default() -> Self {
        WidgetDragLink::new()
    }
}

impl WidgetDragLink {
    /// Nobody listening yet.
    pub fn new() -> WidgetDragLink {
        WidgetDragLink {
            asks: broadcast::Sender::new(64),
        }
    }

    /// Send `input` to `output`'s layer; how many layers heard it.
    pub fn ask(&self, output: OutputId, input: DragInput) -> usize {
        self.asks.send(DragAsk { output, input }).unwrap_or(0)
    }

    /// A receiver for one layer.
    pub fn subscribe(&self) -> broadcast::Receiver<DragAsk> {
        self.asks.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;

    use super::{DragAsk, WidgetDragLink};
    use crate::surfaces::desktop_widgets::arrange::LayerPoint;
    use crate::surfaces::desktop_widgets::drag::DragInput;

    #[test]
    fn every_ask_reaches_every_layer_in_order() {
        let link = WidgetDragLink::new();
        let at = LayerPoint { x: 1, y: 2 };
        assert_eq!(link.ask(OutputId(1), DragInput::Down(at)), 0, "nobody yet");
        let mut rx = link.subscribe();
        let inputs = [DragInput::Down(at), DragInput::Move(at), DragInput::Up(at)];
        for input in inputs {
            assert_eq!(link.ask(OutputId(1), input), 1);
        }
        for input in inputs {
            assert_eq!(
                rx.try_recv().ok(),
                Some(DragAsk {
                    output: OutputId(1),
                    input
                })
            );
        }
    }
}
