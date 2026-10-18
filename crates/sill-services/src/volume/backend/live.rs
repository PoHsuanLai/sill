//! libpulse over pipewire-pulse: follows the default sink and feeds every change through
//! [`step`]. Runs on its own OS thread (`Handle::spawn_blocking`), driving the
//! *standard* mainloop (`iterate()`, no thread of its own) rather than libpulse's threaded
//! mainloop: the threaded mainloop's callbacks are invoked from a second, C-spawned thread, and
//! the crate's own canonical usage reaches back into the shared `Context`/`Mainloop` from inside
//! those callbacks via `RefCell::as_ptr()` plus a raw-pointer deref, which is `unsafe` — denied
//! workspace-wide outside shell-host's one allowed module (CONVENTIONS §2). Driving the
//! standard mainloop from this one thread means every callback also runs on this one thread, so
//! `Rc<RefCell<_>>` is sound here with no unsafe at all.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};

use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::subscribe::InterestMaskSet;
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet, State as ContextState};
use libpulse_binding::mainloop::api::Mainloop as MainloopTrait;
use libpulse_binding::mainloop::events::io::FlagSet as IoFlags;
use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
use libpulse_binding::volume::{ChannelVolumes, Volume as PaVolume};
use sill_model::units::Percent;
use sill_model::volume::{Mute, Volume, VolumeCommand, VolumeEvent, step, stepped};
use tokio::sync::watch;

use super::wake::Wake;
use crate::supervise::Backoff;

/// The default sink this service last saw: enough to address a volume or mute change back to
/// it. Zero channels means "no sink": commands are dropped, matching `step`.
#[derive(Debug, Clone, Copy, Default)]
struct Sink {
    index: u32,
    channels: u8,
}

/// How one connection to the sound server ended.
enum Session {
    /// The commands closed: the service is done.
    Commands,
    /// The server was not there, refused, or went away (a restart of PipeWire); connect again.
    Lost,
}

/// Runs until `commands` closes (the sender was dropped), connecting again with a growing wait
/// whenever the sound server is missing or goes away, and disconnecting at the end. Blocking
/// throughout, asleep in libpulse's poll (or in the wait to connect again) until something
/// happens: call from a dedicated thread (`Handle::spawn_blocking`). `wake` is rung after each
/// command is queued.
pub(crate) fn run(
    tx: &watch::Sender<Volume>,
    commands: &Receiver<VolumeCommand>,
    wake: &Arc<Wake>,
) {
    let mut backoff = Backoff::default();
    loop {
        let started = Instant::now();
        match session(tx, commands, wake) {
            Session::Commands => return,
            Session::Lost => {
                apply_event(tx, VolumeEvent::SinkGone);
                backoff.after_attempt(started.elapsed());
                if matches!(wait_out(backoff.wait(), commands), Waited::CommandsClosed) {
                    return;
                }
            }
        }
    }
}

/// Whether the commands closed while waiting to connect again.
enum Waited {
    Elapsed,
    CommandsClosed,
}

/// Wait `wait` for the commands to close, dropping the commands that come meanwhile (there is no
/// sink to apply them to). The thread sleeps in the receive, not in a loop of short sleeps.
fn wait_out(wait: Duration, commands: &Receiver<VolumeCommand>) -> Waited {
    let until = Instant::now() + wait;
    loop {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Waited::Elapsed;
        }
        match commands.recv_timeout(left) {
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) => return Waited::Elapsed,
            Err(RecvTimeoutError::Disconnected) => return Waited::CommandsClosed,
        }
    }
}

/// Whether the connection to the sound server is up, so commands have a sink to go to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Link {
    Connecting,
    Ready,
}

/// One connection to the sound server, from connecting to its end.
fn session(
    tx: &watch::Sender<Volume>,
    commands: &Receiver<VolumeCommand>,
    wake: &Arc<Wake>,
) -> Session {
    let Some(mut mainloop) = Mainloop::new() else {
        return Session::Lost;
    };
    let Some(context) = Context::new(&mainloop, "sill") else {
        return Session::Lost;
    };
    let context = Rc::new(RefCell::new(context));
    let sink = Rc::new(RefCell::new(Sink::default()));

    // The poll below watches this descriptor too, so a command that is queued wakes it. The event
    // lives as long as this binding does.
    let woken = Arc::clone(wake);
    let Some(_wake_event) = mainloop.new_io_event(
        wake.raw(),
        IoFlags::INPUT,
        Box::new(move |_event, _fd, _flags| woken.drain()),
    ) else {
        return Session::Lost;
    };

    if context
        .borrow_mut()
        .connect(None, ContextFlagSet::NOAUTOSPAWN, None)
        .is_err()
    {
        return Session::Lost;
    }

    let mut link = Link::Connecting;
    let ended = loop {
        // Bound to an owned value first: `context.borrow()` in a `match` scrutinee stays
        // borrowed for every arm's body (Rust's temporary-lifetime rule for `match`), which
        // would panic when `subscribe`/`refresh` below try to borrow it again.
        let state = context.borrow().get_state();
        match state {
            ContextState::Ready if link == Link::Connecting => {
                link = Link::Ready;
                subscribe(&context, tx, &sink);
                refresh(&context, tx, &sink);
            }
            // Checked for the whole life of the connection, so a server that restarts after a
            // good start is noticed as well as one that never came up.
            ContextState::Failed | ContextState::Terminated => break Session::Lost,
            _ => {}
        }

        if let Some(ended) = drain(commands, link, &context, &sink, tx) {
            break ended;
        }

        // Asleep until the sound server says something or a command rings the wake descriptor.
        match mainloop.iterate(true) {
            IterateResult::Quit(_) | IterateResult::Err(_) => break Session::Lost,
            IterateResult::Success(_) => {}
        }
    };

    context.borrow_mut().set_subscribe_callback(None);
    context.borrow_mut().disconnect();
    ended
}

/// Apply every queued command (dropped while the link is not up); the session is over when the
/// sender has gone.
fn drain(
    commands: &Receiver<VolumeCommand>,
    link: Link,
    context: &Rc<RefCell<Context>>,
    sink: &Rc<RefCell<Sink>>,
    tx: &watch::Sender<Volume>,
) -> Option<Session> {
    loop {
        match commands.try_recv() {
            Ok(command) if link == Link::Ready => apply(context, sink, tx, command),
            Ok(_) => {}
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => return Some(Session::Commands),
        }
    }
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
    introspector.get_server_info(move |info| {
        if let Some(name) = info.default_sink_name.as_deref() {
            fetch_sink(&context, name.to_owned(), &tx, &sink);
        } else {
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
    let next = step(tx.borrow().clone(), event);
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
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::{PaVolume, Waited, channel_volumes, percent_of, wait_out};
    use sill_model::units::Percent;
    use sill_model::volume::VolumeCommand;

    #[test]
    fn waiting_to_connect_again_sleeps_until_the_commands_close() {
        let (sender, commands) = mpsc::channel::<VolumeCommand>();
        let closer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            drop(sender);
        });
        let started = Instant::now();
        let waited = wait_out(Duration::from_secs(60), &commands);
        assert!(matches!(waited, Waited::CommandsClosed));
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "it waited the whole minute"
        );
        closer
            .join()
            .unwrap_or_else(|_| panic!("the closer panicked"));
    }

    #[test]
    fn commands_that_come_while_waiting_are_dropped_and_the_wait_still_ends() {
        let (sender, commands) = mpsc::channel();
        for level in [10, 20, 30] {
            sender
                .send(VolumeCommand::SetLevel(Percent(level)))
                .unwrap_or_else(|e| panic!("{e}"));
        }
        let waited = wait_out(Duration::from_millis(100), &commands);
        assert!(matches!(waited, Waited::Elapsed));
        assert!(commands.try_recv().is_err(), "they were taken, not kept");
    }

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
