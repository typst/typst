use std::hash::Hash;
use std::io::Write;
use std::path::PathBuf;
use std::thread::ThreadId;
use std::time::Instant;
use std::{fs::File, io::BufWriter};

use parking_lot::Mutex;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use typst_syntax::Span;

/// Whether the tracer is enabled.
/// This is `false` by default.
///
/// # Safety
/// This is `unsafe` because it is a global variable that is not thread-safe.
/// But at worst, if we have a race condition, we will just be missing some
/// events. So it's not a big deal. And it avoids needing to do an atomic
/// operation every time we want to check if the tracer is enabled.
static mut ENABLED: bool = false;

/// The recorder of events.
struct Recorder {
    /// The events that have been recorded.
    events: Vec<Event>,
    /// The discriminator of the next event.
    discriminator: u64,
}

impl Recorder {
    /// Create a new recorder.
    pub const fn new() -> Self {
        Self { events: Vec::new(), discriminator: 0 }
    }
}

/// The global event recorder.
pub(crate) static RECORDER: Mutex<Recorder> = Mutex::new(Recorder::new());

/// An event that has been recorded.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum Event {
    Start {
        /// The start time of this event.
        start: Instant,
        /// The discriminator of this event.
        id: u64,
        /// The name of this event.
        name: &'static str,
        /// The span of code that this event was recorded in.
        span: Option<Span>,
        /// The thread ID of this event.
        thread_id: ThreadId,
    },
    End {
        /// The end time of this event.
        end: Instant,
        /// The discriminator of this event.
        id: u64,
        /// The name of this event.
        name: &'static str,
        /// The span of code that this event was recorded in.
        span: Option<Span>,
        /// The thread ID of this event.
        thread_id: ThreadId,
    },
}

/// Enable the tracer.
#[inline]
pub fn enable() {
    unsafe {
        ENABLED = true;
    }
}

/// Whether the tracer is enabled.
#[inline]
pub fn is_enabled() -> bool {
    unsafe { ENABLED }
}

/// Clears the recorded events.
#[inline]
pub fn clear() {
    RECORDER.lock().events.clear();
}

/// Record an event.
#[inline]
pub fn record<O>(name: &'static str, span: Option<Span>, call: impl FnOnce() -> O) -> O {
    if !is_enabled() {
        return call();
    }

    let scope = Scope::new(name, span);
    let out = call();
    drop(scope);
    out
}

/// Export data as JSON for Chrome's tracing tool.
///
/// The `source` function is called for each span to get the source code
/// location of the span. The first element of the tuple is the file path and
/// the second element is the line number.
pub fn export_json(
    path: PathBuf,
    mut source: impl FnMut(Span) -> (String, u32),
) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("failed to create file: {e}"))?;
    let mut writer = BufWriter::with_capacity(1 << 20, file);

    if !is_enabled() {
        writer
            .write_all(b"[]")
            .map_err(|e| format!("failed to write to file: {e}"))?;
        return Ok(());
    }

    #[derive(Clone, Serialize)]
    struct Args {
        file: String,
        line: u32,
    }

    #[derive(Serialize)]
    struct Entry {
        name: &'static str,
        cat: &'static str,
        ph: &'static str,
        ts: f64,
        pid: u64,
        tid: u64,
        args: Option<Args>,
    }

    let recorder = RECORDER.lock();
    let Some(first) = recorder.events.first() else {
        writer
            .write_all(b"[]")
            .map_err(|e| format!("failed to write to file: {e}"))?;
        return Ok(());
    };

    let Event::Start { start: run_start, .. } = first else {
        unreachable!("first event is not a start event")
    };

    let mut serializer = serde_json::Serializer::new(writer);
    let mut seq = serializer
        .serialize_seq(Some(recorder.events.len()))
        .map_err(|e| format!("failed to serialize events: {e}"))?;
    for entry in recorder.events.iter() {
        match entry {
            Event::Start { start, name, span, thread_id, .. } => {
                let args = span.map(&mut source).map(|(file, line)| Args { file, line });
                seq.serialize_element(&Entry {
                    name,
                    cat: "typst",
                    ph: "B",
                    ts: (*start - *run_start).as_nanos() as f64 / 1_000.0,
                    pid: 1,
                    tid: unsafe {
                        // Safety: `thread_id` is a `ThreadId` which is a `u64`.
                        std::mem::transmute_copy(&thread_id)
                    },
                    args: args.clone(),
                })
                .map_err(|e| format!("failed to serialize event: {e}"))?;
            }
            Event::End { end, name, span, thread_id, .. } => {
                let args = span.map(&mut source).map(|(file, line)| Args { file, line });
                seq.serialize_element(&Entry {
                    name,
                    cat: "typst",
                    ph: "E",
                    ts: (*end - *run_start).as_nanos() as f64 / 1_000.0,
                    pid: 1,
                    tid: unsafe {
                        // Safety: `thread_id` is a `ThreadId` which is a `u64`.
                        std::mem::transmute_copy(&thread_id)
                    },
                    args,
                })
                .map_err(|e| format!("failed to serialize event: {e}"))?;
            }
        }
    }

    seq.end().map_err(|e| format!("failed to serialize events: {e}"))?;

    Ok(())
}

/// A scope that records an event when it is dropped.
pub struct Scope {
    name: &'static str,
    span: Option<Span>,
    id: u64,
    thread_id: ThreadId,
}

impl Scope {
    /// Create a new scope.
    pub fn new(name: &'static str, span: Option<Span>) -> Self {
        let start = Instant::now();
        let thread_id = std::thread::current().id();
        let id = {
            let mut recorder = RECORDER.lock();
            let id = recorder.discriminator;
            recorder.discriminator += 1;
            recorder
                .events
                .push(Event::Start { start, id, name, span, thread_id });
            id
        };

        Scope { name, span, id, thread_id }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        let event = Event::End {
            end: Instant::now(),
            id: self.id,
            name: self.name,
            span: self.span,
            thread_id: self.thread_id,
        };

        RECORDER.lock().events.push(event);
    }
}
