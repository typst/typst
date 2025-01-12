//! Performance timing for Typst.

use std::io::Write;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use parking_lot::Mutex;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

/// Whether the timer is enabled. Defaults to `false`.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// The global event recorder.
static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());

/// An event that has been recorded.
#[derive(Clone, Copy)]
struct Event {
    /// Whether this is a start or end event.
    kind: EventKind,
    /// The time at which this event occurred.
    timestamp: Timestamp,
    /// The name of this event.
    name: &'static str,
    /// The raw value of the span of code that this event was recorded in.
    span: Option<NonZeroU64>,
    /// The thread ID of this event.
    thread_id: u64,
}

/// Whether an event marks the start or end of a scope.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum EventKind {
    Start,
    End,
}

/// Enable the timer.
#[inline]
pub fn enable() {
    // We only need atomicity and no synchronization of other
    // operations, so `Relaxed` is fine.
    ENABLED.store(true, Ordering::Relaxed);
}

/// Whether the timer is enabled.
#[inline]
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Clears the recorded events.
#[inline]
pub fn clear() {
    EVENTS.lock().clear();
}

/// A scope that records an event when it is dropped.
pub struct TimingScope {
    name: &'static str,
    span: Option<NonZeroU64>,
    thread_id: u64,
}

impl TimingScope {
    /// Create a new scope if timing is enabled.
    #[inline]
    pub fn new(name: &'static str) -> Option<Self> {
        Self::with_span(name, None)
    }

    /// Create a new scope with a span if timing is enabled.
    ///
    /// The span is a raw number because `typst-timing` can't depend on
    /// `typst-syntax` (or else `typst-syntax` couldn't depend on
    /// `typst-timing`).
    #[inline]
    pub fn with_span(name: &'static str, span: Option<NonZeroU64>) -> Option<Self> {
        if is_enabled() {
            return Some(Self::new_impl(name, span));
        }
        None
    }

    /// Create a new scope without checking if timing is enabled.
    fn new_impl(name: &'static str, span: Option<NonZeroU64>) -> Self {
        let timestamp = Timestamp::now();
        let thread_id = thread_id();
        EVENTS.lock().push(Event {
            kind: EventKind::Start,
            timestamp,
            name,
            span,
            thread_id,
        });
        Self { name, span, thread_id }
    }
}

impl Drop for TimingScope {
    fn drop(&mut self) {
        let timestamp = Timestamp::now();
        EVENTS.lock().push(Event {
            kind: EventKind::End,
            timestamp,
            name: self.name,
            span: self.span,
            thread_id: self.thread_id,
        });
    }
}

/// A cross-platform way to get the current time.
#[derive(Clone, Copy)]
struct Timestamp {
    #[cfg(not(target_arch = "wasm32"))]
    inner: std::time::SystemTime,
    #[cfg(target_arch = "wasm32")]
    inner: f64,
}

impl Timestamp {
    #[cfg(not(target_arch = "wasm32"))]
    fn now() -> Self {
        Self { inner: std::time::SystemTime::now() }
    }

    #[cfg(target_arch = "wasm32")]
    fn now() -> Self {
        use web_sys::js_sys;
        use web_sys::wasm_bindgen::JsCast;

        thread_local! {
            static PERF: Option<web_sys::Performance> =
                web_sys::window().and_then(|window| window.performance()).or_else(|| {
                    js_sys::global()
                        .dyn_into::<web_sys::WorkerGlobalScope>()
                        .ok()
                        .and_then(|scope| scope.performance())
                });
        }

        let inner = PERF.with(|perf| match perf {
            Some(perf) => perf.time_origin() + perf.now(),
            None => panic!("failed to get performance"),
        });

        Self { inner }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn millis_since(self, start: Self) -> f64 {
        self.inner
            .duration_since(start.inner)
            .unwrap_or(std::time::Duration::ZERO)
            .as_nanos() as f64
            / 1_000.0
    }

    #[cfg(target_arch = "wasm32")]
    fn millis_since(self, start: Self) -> f64 {
        self.inner - start.inner
    }
}

/// Cross platform way to generate a unique ID per-thread.
///
/// Should also have less overhead than `std::thread::current().id()` because
/// the former does a bunch of stuff and also clone an `Arc`.
fn thread_id() -> u64 {
    static CURRENT: AtomicU64 = AtomicU64::new(1);

    thread_local! {
        // We only need atomicity and no synchronization of other
        // operations, so `Relaxed` is fine.
        static ID: u64 = CURRENT.fetch_add(1, Ordering::Relaxed)
    }

    ID.with(|&id| id)
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
        let __scope = $crate::TimingScope::with_span($name, Some($span));
        $body
    }};
    ($name:expr, $body:expr $(,)?) => {{
        let __scope = $crate::TimingScope::new($name);
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
    mut source: impl FnMut(NonZeroU64) -> (String, u32),
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

    let lock = EVENTS.lock();
    let events = lock.as_slice();

    let mut serializer = serde_json::Serializer::new(writer);
    let mut seq = serializer
        .serialize_seq(Some(events.len()))
        .map_err(|e| format!("failed to serialize events: {e}"))?;

    for event in events.iter() {
        seq.serialize_element(&Entry {
            name: event.name,
            cat: "typst",
            ph: match event.kind {
                EventKind::Start => "B",
                EventKind::End => "E",
            },
            ts: event.timestamp.millis_since(events[0].timestamp),
            pid: 1,
            tid: event.thread_id,
            args: event.span.map(&mut source).map(|(file, line)| Args { file, line }),
        })
        .map_err(|e| format!("failed to serialize event: {e}"))?;
    }

    seq.end().map_err(|e| format!("failed to serialize events: {e}"))?;

    Ok(())
}
