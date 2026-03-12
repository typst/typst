//! A minimal hot-reloading HTTP server.

#![cfg(feature = "http-server")]

use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::sync::Arc;

use ecow::eco_format;
use parking_lot::{Condvar, Mutex, MutexGuard};
use tiny_http::{Header, Request, Response, StatusCode};
use typst_library::diag::{StrResult, bail};
use typst_library::foundations::Bytes;

type Router = Box<dyn Fn(&str) -> Option<HttpBody> + Send + Sync>;
type RouterBucket = Bucket<Router>;

/// Serves HTML with live reload.
pub struct HttpServer {
    addr: SocketAddr,
    bucket: Arc<RouterBucket>,
}

impl HttpServer {
    /// Create a new HTTP server that serves live HTML.
    pub fn new(title: &str, port: Option<u16>, live: bool) -> StrResult<Self> {
        let (addr, server) = start_server(port)?;

        let placeholder = PLACEHOLDER_HTML.replace("{INPUT}", title);
        let bucket = Arc::new(Bucket::new(html_single_fs(placeholder)));
        let bucket2 = bucket.clone();

        std::thread::spawn(move || {
            for req in server.incoming_requests() {
                let _ = handle(req, live, &bucket2);
            }
        });

        Ok(Self { addr, bucket })
    }

    /// The address that we serve the HTML on.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Updates the served contents to a page of HTML served on `/`, triggering
    /// a reload in all connected browsers.
    pub fn set_html(&self, html: String) {
        self.bucket.put(html_single_fs(html));
    }

    /// Updates the content handler, triggering a reload in all connected browsers.
    pub fn set_router<R>(&self, router: R)
    where
        R: Fn(&str) -> Option<HttpBody> + Send + Sync + 'static,
    {
        self.bucket.put(Box::new(router));
    }
}

/// Creates a handler that serves just one HTML page at `/`.
fn html_single_fs(html: String) -> Router {
    Box::new(move |route| (route == "/").then(|| HttpBody::Html(html.clone())))
}

/// Something that can be served by the [`HttpServer`].
pub enum HttpBody {
    /// An HTML page.
    ///
    /// The string must contain valid HTML. If live reload is enabled, a script
    /// will be injected into the HTML.
    Html(String),
    /// A raw body that does not support live reload.
    Raw(Bytes),
}

/// Starts a local HTTP server.
///
/// Uses the specified port or tries to find a free port in the range
/// `3000..=3005`.
fn start_server(port: Option<u16>) -> StrResult<(SocketAddr, tiny_http::Server)> {
    const BASE_PORT: u16 = 3000;

    let mut addr;
    let mut retries = 0;

    let listener = loop {
        addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            port.unwrap_or(BASE_PORT + retries),
        );

        match TcpListener::bind(addr) {
            Ok(listener) => break listener,
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
                if let Some(port) = port {
                    bail!("port {port} is already in use")
                } else if retries < 5 {
                    // If the port is in use, try the next one.
                    retries += 1;
                } else {
                    bail!("could not find free port for HTTP server");
                }
            }
            Err(err) => bail!("failed to start TCP server: {err}"),
        }
    };

    let server = tiny_http::Server::from_listener(listener, None)
        .map_err(|err| eco_format!("failed to start HTTP server: {err}"))?;

    Ok((addr, server))
}

/// Handles a request.
fn handle(req: Request, reload: bool, bucket: &Arc<RouterBucket>) -> io::Result<()> {
    let path = req.url();
    if path == "/__events" {
        return handle_events(req, bucket.clone());
    }

    let fs = bucket.get();
    if let Some(body) = fs(path) {
        handle_body(req, reload, body)
    } else {
        req.respond(Response::new_empty(StatusCode(404)))
    }
}

/// Handles for the `/` route. Serves the compiled HTML.
fn handle_body(req: Request, reload: bool, mut body: HttpBody) -> io::Result<()> {
    let (data, mime) = match &mut body {
        HttpBody::Html(html) => {
            if reload {
                inject_live_reload_script(html);
            }
            (html.as_bytes(), Some("text/html"))
        }
        HttpBody::Raw(data) => (data.as_slice(), select_mime_type(req.url(), data)),
    };

    let mut headers = Vec::new();
    if let Some(mime) = mime {
        headers.push(Header::from_bytes("Content-Type", mime).unwrap());
    }

    req.respond(Response::new(StatusCode(200), headers, data, Some(data.len()), None))
}

/// Handler for the `/__events` route.
fn handle_events(req: Request, bucket: Arc<RouterBucket>) -> io::Result<()> {
    std::thread::spawn(move || {
        // When this returns an error, the client is disconnected and we can
        // terminate the thread.
        let _ = handle_events_blocking(req, &bucket);
    });
    Ok(())
}

/// Event stream for the `/events` route.
fn handle_events_blocking(req: Request, bucket: &RouterBucket) -> io::Result<()> {
    let mut writer = req.into_writer();
    let writer: &mut dyn Write = &mut *writer;

    // We need to write the header manually because `tiny-http` defaults to
    // `Transfer-Encoding: chunked` when no `Content-Length` is provided, which
    // Chrome & Safari dislike for `Content-Type: text/event-stream`.
    write!(writer, "HTTP/1.1 200 OK\r\n")?;
    write!(writer, "Content-Type: text/event-stream\r\n")?;
    write!(writer, "Cache-Control: no-cache\r\n")?;
    write!(writer, "\r\n")?;
    writer.flush()?;

    // If the user closes the browser tab, this loop will terminate once it
    // tries to write to the dead socket for the first time.
    loop {
        bucket.wait();
        // Trigger a server-sent event. The browser is listening to it via
        // an `EventSource` listener` (see `inject_script`).
        write!(writer, "event: reload\ndata:\n\n")?;
        writer.flush()?;
    }
}

/// Injects the live reload script into a string of HTML.
fn inject_live_reload_script(html: &mut String) {
    let pos = html.rfind("</body>").unwrap_or(html.len());
    html.insert_str(pos, LIVE_RELOAD_SCRIPT);
}

/// Selects a MIME type for a request based on path and data.
fn select_mime_type(path: &str, buf: &[u8]) -> Option<&'static str> {
    match path.rsplit_once('.').map(|(_, r)| r) {
        Some("html") => Some("text/html"),
        Some("pdf") => Some("application/pdf"),
        Some("png") => Some("image/png"),
        Some("svg") => Some("image/svg+xml"),
        _ => infer::get(buf).map(|ty| ty.mime_type()),
    }
}

/// Holds data and notifies consumers when it's updated.
struct Bucket<T> {
    mutex: Mutex<T>,
    condvar: Condvar,
}

impl<T> Bucket<T> {
    /// Creates a new bucket with initial data.
    fn new(init: T) -> Self {
        Self { mutex: Mutex::new(init), condvar: Condvar::new() }
    }

    /// Retrieves the current data in the bucket.
    fn get(&self) -> MutexGuard<'_, T> {
        self.mutex.lock()
    }

    /// Puts new data into the bucket and notifies everyone who's currently
    /// [waiting](Self::wait).
    fn put(&self, data: T) {
        *self.mutex.lock() = data;
        self.condvar.notify_all();
    }

    /// Waits for new data in the bucket.
    fn wait(&self) {
        self.condvar.wait(&mut self.mutex.lock());
    }
}

/// The initial HTML before compilation is finished.
const PLACEHOLDER_HTML: &str = "\
<!DOCTYPE html>
<html>
  <head>
    <meta charset=\"utf-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
    <title>Waiting for {INPUT}</title>
    <style>
      body {
        display: flex;
        justify-content: center;
        align-items: center;
        color: #565565;
        background: #eff0f3;
      }

      body > main > div {
        margin-block: 16px;
        text-align: center;
      }
    </style>
  </head>
  <body>
    <main>
      <div>Waiting for output…</div>
      <div><code>typst watch {INPUT}</code></div>
    </main>
  </body>
</html>
";

/// Reloads the page whenever it receives a "reload" server-sent event
/// on the `/__events` route.
const LIVE_RELOAD_SCRIPT: &str = "\
<script>\
  new EventSource(\"/__events\")\
    .addEventListener(\"reload\", () => location.reload())\
</script>\
";
