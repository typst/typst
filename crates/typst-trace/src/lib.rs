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

/// The global event recorder.
static RECORDER: Mutex<Recorder> = Mutex::new(Recorder::new());

/// The recorder of events.
struct Recorder {
    /// The events that have been recorded.
    events: Vec<Event>,
    /// The discriminator of the next event.
    discriminator: u64,
}

impl Recorder {
    /// Create a new recorder.
    const fn new() -> Self {
        Self { events: Vec::new(), discriminator: 0 }
    }
}

/// An event that has been recorded.
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
struct Event {
    /// Whether this is a start or end event.
    kind: EventKind,
    /// The start time of this event.
    timestamp: Instant,
    /// The discriminator of this event.
    id: u64,
    /// The name of this event.
    name: &'static str,
    /// The span of code that this event was recorded in.
    span: Option<Span>,
    /// The thread ID of this event.
    thread_id: ThreadId,
}

/// Whether an event marks the start or end of a span.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum EventKind {
    Start,
    End,
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

    #[derive(Serialize)]
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

    let run_start = first.timestamp;
    if first.kind != EventKind::Start {
        unreachable!("first event is not a start event")
    }

    let mut serializer = serde_json::Serializer::new(writer);
    let mut seq = serializer
        .serialize_seq(Some(recorder.events.len()))
        .map_err(|e| format!("failed to serialize events: {e}"))?;

    for event in recorder.events.iter() {
        seq.serialize_element(&Entry {
            name: event.name,
            cat: "typst",
            ph: match event.kind {
                EventKind::Start => "B",
                EventKind::End => "E",
            },
            ts: (event.timestamp - run_start).as_nanos() as f64 / 1_000.0,
            pid: 1,
            tid: unsafe {
                // Safety: `thread_id` is a `ThreadId` which is a `u64`.
                std::mem::transmute_copy(&event.thread_id)
            },
            args: event.span.map(&mut source).map(|(file, line)| Args { file, line }),
        })
        .map_err(|e| format!("failed to serialize event: {e}"))?;
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
        let timestamp = Instant::now();
        let thread_id = std::thread::current().id();

        let mut recorder = RECORDER.lock();
        let id = recorder.discriminator;
        recorder.discriminator += 1;
        recorder.events.push(Event {
            kind: EventKind::Start,
            timestamp,
            id,
            name,
            span,
            thread_id,
        });

        Scope { name, span, id, thread_id }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        let event = Event {
            kind: EventKind::End,
            timestamp: Instant::now(),
            id: self.id,
            name: self.name,
            span: self.span,
            thread_id: self.thread_id,
        };

        RECORDER.lock().events.push(event);
    }
}

/// Creates a scope around an expression.
///
/// The output of the expression is returned.
///
/// The scope will be named `name` and will have the span `span`. The span is
/// optional.
///
/// ## Example
///
/// ```rs
/// // With a scope name and span.
/// scoped!(
///     "my scope",
///     span = Span::detached(),
///     std::thread::sleep(std::time::Duration::from_secs(1))
/// );
///
/// // With a scope name and no span.
/// scoped!(
///     "my scope",
///     std::thread::sleep(std::time::Duration::from_secs(1))
/// );
/// ```
#[macro_export]
macro_rules! scoped {
    ($name:literal, span = $span:expr, $eval:expr) => {{
        let __inner_scope = $crate::Scope::new($name, Some($span));
        let out = { $eval };
        drop(__inner_scope);
        out
    }};
    ($name:literal, $eval:expr) => {{
        let __inner_scope = $crate::Scope::new($name, None);
        let out = { $eval };
        drop(__inner_scope);
        out
    }};
}
