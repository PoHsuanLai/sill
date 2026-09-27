//! The one thread that talks DDC. Every bus transaction happens here, one at a time; the rest
//! of the shell sends [`DdcJob`]s and reads the published [`DdcDisplay`]s, which double as the
//! read cache (a value is read at probe time and then follows sill's own writes, so a slider
//! never waits on the bus).

use std::collections::BTreeSet;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use super::bus::{DdcBus, DdcError, DdcId, Endpoint};
use super::debounce::{Debounce, Write};
use super::retry;
use super::vcp::{Controls, Feature};
use crate::units::Stamp;

/// One DDC display and its cached controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DdcDisplay {
    pub endpoint: Endpoint,
    pub controls: Controls,
}

/// What the worker can be asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdcJob {
    /// Find the displays that answer and read every control (at start, and when outputs change).
    Probe,
    /// Set a feature: cached at once, written after the debounce.
    Write(Write),
    /// Read `feature` again on every display (a change made on the monitor or by another
    /// program), except where sill's own write of it is still waiting.
    Reread(Feature),
}

/// Which half of a transaction failed, for the log-once set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Op {
    Read,
    Write,
}

/// Whether a re-read found anything new.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Changed {
    Same,
    Different,
}

struct Worker<B> {
    bus: B,
    displays: Vec<DdcDisplay>,
    debounce: Debounce,
    warned: BTreeSet<(DdcId, Feature, Op)>,
    tx: watch::Sender<Vec<DdcDisplay>>,
    origin: Instant,
    /// How many displays the last probe found (logged when it changes).
    found: Option<usize>,
}

/// Serve `jobs` until every sender is gone, then write whatever is still pending (a restore
/// sent just before shutdown still reaches the monitor). Blocking: run it on its own thread.
pub fn run<B: DdcBus>(bus: B, jobs: Receiver<DdcJob>, tx: watch::Sender<Vec<DdcDisplay>>) {
    let mut worker = Worker {
        bus,
        displays: Vec::new(),
        debounce: Debounce::default(),
        warned: BTreeSet::new(),
        tx,
        origin: Instant::now(),
        found: None,
    };
    loop {
        let now = worker.now();
        let next = match worker.debounce.next_due() {
            Some(due) => jobs.recv_timeout(Duration::from_millis(due.since(now))),
            None => jobs.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match next {
            Ok(job) => worker.handle(job),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        worker.flush(worker.now());
    }
    worker.flush(Stamp(u64::MAX));
}

impl<B: DdcBus> Worker<B> {
    fn now(&self) -> Stamp {
        Stamp(u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX))
    }

    fn handle(&mut self, job: DdcJob) {
        match job {
            DdcJob::Probe => self.probe(),
            DdcJob::Write(write) => {
                let Some(display) = self.displays.iter_mut().find(|d| d.endpoint.id == write.id)
                else {
                    return;
                };
                display.controls = display.controls.written(write.feature, write.value);
                let debounce = std::mem::take(&mut self.debounce);
                self.debounce = debounce.push(self.now(), write);
                self.publish();
            }
            DdcJob::Reread(feature) => self.reread(feature),
        }
    }

    fn reread(&mut self, feature: Feature) {
        let ids: Vec<DdcId> = self
            .displays
            .iter()
            .map(|d| d.endpoint.id)
            .filter(|id| self.debounce.waiting(*id, feature).is_none())
            .collect();
        let mut changed = Changed::Same;
        for id in ids {
            let read = retry::read(&mut self.bus, id, feature);
            let Some(level) = self.quietly(id, feature, Op::Read, read) else {
                continue;
            };
            if let Some(display) = self.displays.iter_mut().find(|d| d.endpoint.id == id) {
                let now = display.controls.reread(feature, level);
                if now != display.controls {
                    display.controls = now;
                    changed = Changed::Different;
                }
            }
        }
        match changed {
            Changed::Different => self.publish(),
            Changed::Same => {}
        }
    }

    fn probe(&mut self) {
        let endpoints = self.bus.probe();
        self.displays = endpoints
            .into_iter()
            .map(|endpoint| {
                let readings: Vec<_> = Feature::ALL
                    .iter()
                    .map(|&feature| {
                        let read = retry::read(&mut self.bus, endpoint.id, feature);
                        (feature, self.quietly(endpoint.id, feature, Op::Read, read))
                    })
                    .collect();
                DdcDisplay {
                    controls: Controls::from_readings(&readings),
                    endpoint,
                }
            })
            .filter(|d| d.controls.any())
            .collect();
        let found = self.displays.len();
        if self.found != Some(found) {
            eprintln!("sill: ddc: {found} display(s) answer DDC/CI");
            self.found = Some(found);
        }
        self.publish();
    }

    fn flush(&mut self, now: Stamp) {
        let (rest, due) = std::mem::take(&mut self.debounce).take_due(now);
        self.debounce = rest;
        for write in due {
            let done = retry::write_verified(&mut self.bus, write.id, write.feature, write.value);
            let _ = self.quietly(write.id, write.feature, Op::Write, done);
        }
    }

    /// `result`'s value; a failure is logged the first time that transaction fails, and the
    /// capability simply reads as absent.
    fn quietly<T>(
        &mut self,
        id: DdcId,
        feature: Feature,
        op: Op,
        result: Result<T, DdcError>,
    ) -> Option<T> {
        result
            .map_err(|error| {
                if self.warned.insert((id, feature, op)) {
                    eprintln!("sill: ddc bus {} {feature:?} {op:?}: {error}", id.0);
                }
            })
            .ok()
    }

    fn publish(&self) {
        self.tx.send_replace(self.displays.clone());
    }
}

#[cfg(test)]
mod tests;
