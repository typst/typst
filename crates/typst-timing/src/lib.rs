//! Performance timing for Typst.

use std::io::Write;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use parking_lot::Mutex;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

thread_local! {
    /// Data that is initialized once per thread.
    static THREAD_DATA: ThreadData = ThreadData {
        id: {
            // We only need atomicity and no synchronization of other
            // operations, so `Relaxed` is fine.
            static COUNTER: AtomicU64 = AtomicU64::new(1);
            COUNTER.fetch_add(1, Ordering::Relaxed)
        },
        #[cfg(all(target_arch = "wasm32", feature = "wasm"))]
        timer: WasmTimer::new(),
    };
}

/// Whether the timer is enabled. Defaults to `false`.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// The list of collected events.
static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());

/// Per-thread data.
struct ThreadData {
    /// The thread's ID.
    ///
    /// In contrast to `std::thread::current().id()`, this is wasm-compatible
    /// and also a bit cheaper to access because the std version does a bit more
    /// stuff (including cloning an `Arc`).
    id: u64,
    /// A way to get the time in Wasm.
    #[cfg(all(target_arch = "wasm32", feature = "wasm"))]
    timer: WasmTimer,
}

/// An event that has been recorded.
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
        let (thread_id, timestamp) =
            THREAD_DATA.with(|data| (data.id, Timestamp::now_with(data)));
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
#[derive(Copy, Clone)]
struct Timestamp {
    #[cfg(not(target_arch = "wasm32"))]
    inner: std::time::SystemTime,
    #[cfg(target_arch = "wasm32")]
    inner: f64,
}

impl Timestamp {
    fn now() -> Self {
        #[cfg(target_arch = "wasm32")]
        return THREAD_DATA.with(Self::now_with);

        #[cfg(not(target_arch = "wasm32"))]
        Self { inner: std::time::SystemTime::now() }
    }

    #[allow(unused_variables)]
    fn now_with(data: &ThreadData) -> Self {
        #[cfg(all(target_arch = "wasm32", feature = "wasm"))]
        return Self { inner: data.timer.now() };

        #[cfg(all(target_arch = "wasm32", not(feature = "wasm")))]
        return Self { inner: 0.0 };

        #[cfg(not(target_arch = "wasm32"))]
        Self::now()
    }

    fn micros_since(self, start: Self) -> f64 {
        #[cfg(target_arch = "wasm32")]
        return (self.inner - start.inner) * 1000.0;

        #[cfg(not(target_arch = "wasm32"))]
        (self
            .inner
            .duration_since(start.inner)
            .unwrap_or(std::time::Duration::ZERO)
            .as_nanos() as f64
            / 1_000.0)
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
            ts: event.timestamp.micros_since(events[0].timestamp),
            pid: 1,
            tid: event.thread_id,
            args: event.span.map(&mut source).map(|(file, line)| Args { file, line }),
        })
        .map_err(|e| format!("failed to serialize event: {e}"))?;
    }

    seq.end().map_err(|e| format!("failed to serialize events: {e}"))?;

    Ok(())
}

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
struct WasmTimer {
    /// The cached JS performance handle for the thread.
    perf: web_sys::Performance,
    /// The cached JS time origin.
    time_origin: f64,
}

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
impl WasmTimer {
    fn new() -> Self {
        // Retrieve `performance` from global object, either the window
        // globalThis.
        let perf = web_sys::window()
            .and_then(|window| window.performance())
            .or_else(|| {
                use web_sys::wasm_bindgen::JsCast;
                web_sys::js_sys::global()
                    .dyn_into::<web_sys::WorkerGlobalScope>()
                    .ok()
                    .and_then(|scope| scope.performance())
            })
            .expect("failed to get JS performance handle");

        // Every thread gets its own time origin. To make the results consistent
        // across threads, we need to add this to each `now()` call.
        let time_origin = perf.time_origin();

        Self { perf, time_origin }
    }

    fn now(&self) -> f64 {
        self.time_origin + self.perf.now()
    }
}
