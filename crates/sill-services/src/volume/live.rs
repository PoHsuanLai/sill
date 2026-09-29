//! libpulse over pipewire-pulse: follows the default sink and feeds every change through
//! [`reduce_volume`]. Runs on its own OS thread (`Handle::spawn_blocking`), polling the
//! *standard* mainloop (`iterate()`, no thread of its own) rather than libpulse's threaded
//! mainloop: the threaded mainloop's callbacks are invoked from a second, C-spawned thread, and
//! the crate's own canonical usage reaches back into the shared `Context`/`Mainloop` from inside
//! those callbacks via `RefCell::as_ptr()` plus a raw-pointer deref, which is `unsafe` — denied
//! workspace-wide outside shell-host's one allowed module (CONVENTIONS §11). Driving the
//! standard mainloop from this one thread means every callback also runs on this one thread, so
//! `Rc<RefCell<_>>` is sound here with no unsafe at all.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::subscribe::InterestMaskSet;
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet, State as ContextState};
use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
use libpulse_binding::volume::{ChannelVolumes, Volume as PaVolume};
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::{mpsc, watch};

use super::model::{Mute, Volume, VolumeCommand, VolumeEvent, reduce_volume, stepped};
use crate::units::Percent;

/// The default sink this service last saw: enough to address a volume or mute change back to
/// it. Zero channels means "no sink": commands are dropped, matching `reduce_volume`.
#[derive(Debug, Clone, Copy, Default)]
struct Sink {
    index: u32,
    channels: u8,
}

/// How long to sleep between mainloop polls: short enough that a volume key feels immediate,
/// long enough not to spin a core for an idle sink.
const POLL: Duration = Duration::from_millis(10);

/// Runs until `commands` closes (the `Control` was dropped), then disconnects. Blocking
/// throughout: call from a dedicated thread (`Handle::spawn_blocking`).
pub(super) fn run(tx: watch::Sender<Volume>, mut commands: mpsc::UnboundedReceiver<VolumeCommand>) {
    let Some(mut mainloop) = Mainloop::new() else {
        return;
    };
    let Some(context) = Context::new(&mainloop, "sill") else {
        return;
    };
    let context = Rc::new(RefCell::new(context));
    let sink = Rc::new(RefCell::new(Sink::default()));

    if context
        .borrow_mut()
        .connect(None, ContextFlagSet::NOAUTOSPAWN, None)
        .is_err()
    {
        return;
    }

    let mut ready = false;
    loop {
        match commands.try_recv() {
            Ok(command) if ready => apply(&context, &sink, &tx, command),
            Ok(_) => {}
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => break,
        }

        match mainloop.iterate(false) {
            IterateResult::Quit(_) | IterateResult::Err(_) => break,
            IterateResult::Success(_) => {}
        }

        if !ready {
            // Bound to an owned value first: `context.borrow()` in a `match` scrutinee stays
            // borrowed for every arm's body (Rust's temporary-lifetime rule for `match`), which
            // would panic when `subscribe`/`refresh` below try to borrow it again.
            let state = context.borrow().get_state();
            match state {
                ContextState::Ready => {
                    ready = true;
                    subscribe(&context, &tx, &sink);
                    refresh(&context, &tx, &sink);
                }
                ContextState::Failed | ContextState::Terminated => break,
                _ => {}
            }
        }

        std::thread::sleep(POLL);
    }

    context.borrow_mut().set_subscribe_callback(None);
    context.borrow_mut().disconnect();
}

/// Subscribes to sink and server changes, refreshing the default sink on every one.
fn subscribe(context: &Rc<RefCell<Context>>, tx: &watch::Sender<Volume>, sink: &Rc<RefCell<Sink>>) {
    let context_ref = Rc::clone(context);
    let tx = tx.clone();
    let sink_ref = Rc::clone(sink);
    context.borrow_mut().set_subscribe_callback(Some(Box::new(
        move |_facility, _operation, _index| {
            refresh(&context_ref, &tx, &sink_ref);
        },
    )));
    let _ = context
        .borrow_mut()
        .subscribe(InterestMaskSet::SINK | InterestMaskSet::SERVER, |_| {});
}

/// Re-reads the default sink and sends the resulting `Volume` (or `SinkGone` if there is none).
fn refresh(context: &Rc<RefCell<Context>>, tx: &watch::Sender<Volume>, sink: &Rc<RefCell<Sink>>) {
    let introspector = context.borrow().introspect();
    let context = Rc::clone(context);
    let tx = tx.clone();
    let sink = Rc::clone(sink);
    introspector.get_server_info(move |info| match info.default_sink_name.as_deref() {
        Some(name) => fetch_sink(&context, name.to_owned(), &tx, &sink),
        None => {
            *sink.borrow_mut() = Sink::default();
            apply_event(&tx, VolumeEvent::SinkGone);
        }
    });
}

/// Reads the named sink's volume, mute and description, and publishes it.
fn fetch_sink(
    context: &Rc<RefCell<Context>>,
    name: String,
    tx: &watch::Sender<Volume>,
    sink: &Rc<RefCell<Sink>>,
) {
    let introspector = context.borrow().introspect();
    let tx = tx.clone();
    let sink = Rc::clone(sink);
    let mut seen = false;
    introspector.get_sink_info_by_name(&name, move |result| match result {
        ListResult::Item(info) => {
            seen = true;
            *sink.borrow_mut() = Sink {
                index: info.index,
                channels: info.volume.len(),
            };
            let mute = if info.mute {
                Mute::Muted
            } else {
                Mute::Audible
            };
            apply_event(
                &tx,
                VolumeEvent::Sink {
                    sink: info.description.as_deref().unwrap_or_default().to_owned(),
                    level: percent_of(info.volume.avg()),
                    mute,
                },
            );
        }
        ListResult::End => {
            if !seen {
                *sink.borrow_mut() = Sink::default();
                apply_event(&tx, VolumeEvent::SinkGone);
            }
        }
        ListResult::Error => {}
    });
}

/// Applies `command` to the current default sink, if there is one.
fn apply(
    context: &Rc<RefCell<Context>>,
    sink: &Rc<RefCell<Sink>>,
    tx: &watch::Sender<Volume>,
    command: VolumeCommand,
) {
    let Sink { index, channels } = *sink.borrow();
    if channels == 0 {
        return;
    }
    match command {
        VolumeCommand::SetLevel(level) => {
            let volumes = channel_volumes(level, channels);
            context
                .borrow()
                .introspect()
                .set_sink_volume_by_index(index, &volumes, None);
        }
        VolumeCommand::Step(step) => {
            let current = tx.borrow().level;
            let volumes = channel_volumes(stepped(current, step), channels);
            context
                .borrow()
                .introspect()
                .set_sink_volume_by_index(index, &volumes, None);
        }
        VolumeCommand::SetMute(mute) => {
            context
                .borrow()
                .introspect()
                .set_sink_mute_by_index(index, mute == Mute::Muted, None);
        }
        VolumeCommand::ToggleMute => {
            let muted = tx.borrow().mute == Mute::Muted;
            context
                .borrow()
                .introspect()
                .set_sink_mute_by_index(index, !muted, None);
        }
    }
}

/// Reduces `event` onto the receiver's current value and publishes the result.
fn apply_event(tx: &watch::Sender<Volume>, event: VolumeEvent) {
    let next = reduce_volume(tx.borrow().clone(), event);
    let _ = tx.send(next);
}

/// Rounds to the nearest percent rather than truncating, so it round-trips through
/// [`channel_volumes`] (also rounded) for every percent a caller can set.
fn percent_of(volume: PaVolume) -> Percent {
    let normal = u64::from(PaVolume::NORMAL.0);
    let scaled = (u64::from(volume.0) * 100 + normal / 2) / normal;
    Percent(u16::try_from(scaled).unwrap_or(u16::MAX))
}

fn channel_volumes(level: Percent, channels: u8) -> ChannelVolumes {
    let raw = (u32::from(level.0) * PaVolume::NORMAL.0 + 50) / 100;
    let mut volumes = ChannelVolumes::default();
    volumes.set(channels, PaVolume(raw));
    volumes
}

#[cfg(test)]
mod tests {
    use super::{PaVolume, channel_volumes, percent_of};
    use crate::units::Percent;

    #[test]
    fn percent_and_channel_volumes_round_trip_through_pa_normal() {
        for level in [0u16, 40, 100, 130] {
            let volumes = channel_volumes(Percent(level), 2);
            assert_eq!(volumes.len(), 2);
            assert_eq!(percent_of(volumes.avg()), Percent(level), "{level}");
        }
        assert_eq!(percent_of(PaVolume::NORMAL), Percent(100));
    }
}
