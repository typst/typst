use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::sync::Arc;

use ecow::eco_format;
use parking_lot::{Condvar, Mutex, MutexGuard};
use tiny_http::{Header, Request, Response, StatusCode};
use typst::diag::{StrResult, bail};

use crate::args::{Input, ServerArgs};

/// Serves HTML with live reload.
pub struct HtmlServer {
    addr: SocketAddr,
    bucket: Arc<Bucket<String>>,
}

impl HtmlServer {
    /// Create a new HTTP server that serves live HTML.
    pub fn new(input: &Input, args: &ServerArgs) -> StrResult<Self> {
        let reload = !args.no_reload;
        let (addr, server) = start_server(args.port)?;

        let placeholder = PLACEHOLDER_HTML.replace("{INPUT}", &input.to_string());
        let bucket = Arc::new(Bucket::new(placeholder));
        let bucket2 = bucket.clone();

        std::thread::spawn(move || {
            for req in server.incoming_requests() {
                let _ = handle(req, reload, &bucket2);
            }
        });

        Ok(Self { addr, bucket })
    }

    /// The address that we serve the HTML on.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Updates the HTML, triggering a reload all connected browsers.
    pub fn update(&self, html: String) {
        self.bucket.put(html);
    }
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
fn handle(req: Request, reload: bool, bucket: &Arc<Bucket<String>>) -> io::Result<()> {
    let path = req.url();
    match path {
        "/" => handle_root(req, reload, bucket),
        "/events" => handle_events(req, bucket.clone()),
        _ => req.respond(Response::new_empty(StatusCode(404))),
    }
}

/// Handles for the `/` route. Serves the compiled HTML.
fn handle_root(req: Request, reload: bool, bucket: &Bucket<String>) -> io::Result<()> {
    let mut html = bucket.get().clone();
    if reload {
        inject_live_reload_script(&mut html);
    }
    req.respond(Response::new(
        StatusCode(200),
        vec![Header::from_bytes("Content-Type", "text/html").unwrap()],
        html.as_bytes(),
        Some(html.len()),
        None,
    ))
}

/// Handler for the `/events` route.
fn handle_events(req: Request, bucket: Arc<Bucket<String>>) -> io::Result<()> {
    std::thread::spawn(move || {
        // When this returns an error, the client is disconnected and we can
        // terminate the thread.
        let _ = handle_events_blocking(req, &bucket);
    });
    Ok(())
}

/// Event stream for the `/events` route.
fn handle_events_blocking(req: Request, bucket: &Bucket<String>) -> io::Result<()> {
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
    let pos = html.rfind("</html>").unwrap_or(html.len());
    html.insert_str(pos, LIVE_RELOAD_SCRIPT);
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
/// on the `/events` route.
const LIVE_RELOAD_SCRIPT: &str = "\
<script>\
  new EventSource(\"/events\")\
    .addEventListener(\"reload\", () => location.reload())\
</script>\
";
