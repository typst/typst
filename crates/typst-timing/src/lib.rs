//! Performance timing for Typst.

use std::hash::Hash;
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::thread::ThreadId;
use std::time::{Duration, SystemTime};

use parking_lot::Mutex;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use typst_syntax::Span;

/// Whether the timer is enabled. Defaults to `false`.
static ENABLED: AtomicBool = AtomicBool::new(false);

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
    timestamp: SystemTime,
    /// The discriminator of this event.
    id: u64,
    /// The name of this event.
    name: &'static str,
    /// The span of code that this event was recorded in.
    span: Option<Span>,
    /// The thread ID of this event.
    thread_id: ThreadId,
}

/// Whether an event marks the start or end of a scope.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum EventKind {
    Start,
    End,
}

/// Enable the timer.
#[inline]
pub fn enable() {
    // We only need atomicity and no synchronization of other
    // operations, so `Relaxed` is fine.
    ENABLED.store(true, Relaxed);
}

/// Whether the timer is enabled.
#[inline]
pub fn is_enabled() -> bool {
    ENABLED.load(Relaxed)
}

/// Clears the recorded events.
#[inline]
pub fn clear() {
    RECORDER.lock().events.clear();
}

/// A scope that records an event when it is dropped.
pub struct TimingScope {
    name: &'static str,
    span: Option<Span>,
    id: u64,
    thread_id: ThreadId,
}

impl TimingScope {
    /// Create a new scope if timing is enabled.
    pub fn new(name: &'static str, span: Option<Span>) -> Option<Self> {
        if !is_enabled() {
            return None;
        }

        let timestamp = SystemTime::now();
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

        Some(TimingScope { name, span, id, thread_id })
    }
}

impl Drop for TimingScope {
    fn drop(&mut self) {
        let event = Event {
            kind: EventKind::End,
            timestamp: SystemTime::now(),
            id: self.id,
            name: self.name,
            span: self.span,
            thread_id: self.thread_id,
        };

        RECORDER.lock().events.push(event);
    }
}

/// Creates a timing scope around an expression.
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
/// timed!(
///     "my scope",
///     span = Span::detached(),
///     std::thread::sleep(std::time::Duration::from_secs(1)),
/// );
///
/// // With a scope name and no span.
/// timed!(
///     "my scope",
///     std::thread::sleep(std::time::Duration::from_secs(1)),
/// );
/// ```
#[macro_export]
macro_rules! timed {
    ($name:expr, span = $span:expr, $body:expr $(,)?) => {{
        let __scope = $crate::TimingScope::new($name, Some($span));
        $body
    }};
    ($name:expr, $body:expr $(,)?) => {{
        let __scope = $crate::TimingScope::new($name, None);
        $body
    }};
}

/// Export data as JSON for Chrome's tracing tool.
///
/// The `source` function is called for each span to get the source code
/// location of the span. The first element of the tuple is the file path and
/// the second element is the line number.
pub fn export_json<W: Write>(
    writer: W,
    mut source: impl FnMut(Span) -> (String, u32),
) -> Result<(), String> {
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

    #[derive(Serialize)]
    struct Args {
        file: String,
        line: u32,
    }

    let recorder = RECORDER.lock();
    let run_start = recorder
        .events
        .first()
        .map(|event| event.timestamp)
        .unwrap_or_else(SystemTime::now);

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
            ts: event
                .timestamp
                .duration_since(run_start)
                .unwrap_or(Duration::ZERO)
                .as_nanos() as f64
                / 1_000.0,
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
