//! Asking before Bluetooth goes off: the user cut off their Bluetooth keyboard twice with one
//! press. A press that would power the adapter
//! off, on the Bluetooth tile or the pane's switch, only raises the question; quire's `Alert`
//! (`Flow::Inline`, over the panel) asks "Turn Bluetooth off?", and only its "Turn Off" sends
//! `BluetoothCommand::Power(Off)`. Cancel, Escape or a press on the scrim leaves it on. Turning
//! it on asks nothing.
//!
//! The panel (and the Bluetooth bar item's own dropdown) holds the question in a [`PowerAsk`]
//! and draws [`TurnOffAlert`] as the last child of its positioned body.

use dioxus::prelude::*;
use ds::{Alert, AlertEmphasis, Flow, Text};
use sill_services::{BluetoothCommand, BtPower};

use crate::hooks::use_services;

/// Whether the panel is asking before Bluetooth goes off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Confirming {
    #[default]
    Idle,
    TurnOff,
}

/// The panel's question, which the tile and the pane's switch raise and [`TurnOffAlert`]
/// answers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PowerAsk(Signal<Confirming>);

/// What a press asking for `to` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PowerStep {
    /// Send it at once.
    Send(BtPower),
    /// Ask first.
    Ask,
}

/// On goes at once; off asks first.
pub(crate) fn power_step(to: BtPower) -> PowerStep {
    match to {
        BtPower::On => PowerStep::Send(BtPower::On),
        BtPower::Off => PowerStep::Ask,
    }
}

/// The question, for the panel that draws [`TurnOffAlert`] (the control center, the Bluetooth
/// bar item's dropdown).
pub(crate) fn provide_power_ask() -> PowerAsk {
    let confirming = use_signal(Confirming::default);
    use_context_provider(|| PowerAsk(confirming))
}

/// A press on the tile or the pane's switch asking for `to`. Without a panel to ask in (a
/// module drawn on its own), it is sent at once.
pub(crate) fn use_power_request() -> Callback<BtPower> {
    let ask = try_use_context::<PowerAsk>();
    let control = use_services().bluetooth_control;
    use_callback(move |to: BtPower| {
        let send = |power: BtPower| {
            // A stopped service drops it: the tile shows the power it last published.
            let _gone = control.send(BluetoothCommand::Power(power));
        };
        match (power_step(to), ask) {
            (PowerStep::Ask, Some(PowerAsk(mut confirming))) => {
                eprintln!("control_center.bluetooth_ask=shown");
                confirming.set(Confirming::TurnOff);
            }
            (PowerStep::Ask, None) => send(BtPower::Off),
            (PowerStep::Send(power), _) => send(power),
        }
    })
}

/// The alert, while the panel asks; render it last in the panel's positioned body.
/// `on_answered` hears either answer, once the alert (and the button that held the keyboard)
/// is going: the panel takes the keyboard back there.
#[component]
pub(crate) fn TurnOffAlert(on_answered: EventHandler<()>) -> Element {
    let Some(PowerAsk(mut confirming)) = try_use_context::<PowerAsk>() else {
        return rsx! {};
    };
    let control = use_services().bluetooth_control;
    match confirming() {
        Confirming::Idle => rsx! {},
        Confirming::TurnOff => rsx! {
            Alert {
                title: "Turn Bluetooth off?",
                message: Some(Text::from(
                    "Bluetooth devices such as keyboards and mice will be disconnected.",
                )),
                action: "Turn Off",
                // Turn Off is the default, so Return confirms; a `Destructive` action would
                // hand Return to Cancel.
                emphasis: AlertEmphasis::Default,
                flow: Flow::Inline,
                onaction: move |()| {
                    eprintln!("control_center.bluetooth_ask=turn-off");
                    let _gone = control.send(BluetoothCommand::Power(BtPower::Off));
                    confirming.set(Confirming::Idle);
                    on_answered.call(());
                },
                oncancel: move |()| {
                    eprintln!("control_center.bluetooth_ask=cancel");
                    confirming.set(Confirming::Idle);
                    on_answered.call(());
                },
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{PowerStep, power_step};
    use sill_services::BtPower;

    #[test]
    fn only_turning_off_asks() {
        assert_eq!(power_step(BtPower::On), PowerStep::Send(BtPower::On));
        assert_eq!(power_step(BtPower::Off), PowerStep::Ask);
    }
}
