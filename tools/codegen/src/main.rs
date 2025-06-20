//! Usage: `cargo run -p typst-codegen -- <job>`
//!
//! See other files for more details.

use std::sync::Arc;

mod html;

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("html") => html::main(),
        Some(job) => panic!("unknown codegen job: {job}"),
        None => panic!("no codegen job provided"),
    }
}

fn fetch(url: &str) -> String {
    println!("Fetching {url}");
    // Can't use `Response::into_string` because it has a 10MB limit.
    let mut buf = String::new();
    ureq::AgentBuilder::new()
        .tls_connector(Arc::new(native_tls::TlsConnector::new().unwrap()))
        .build()
        .get(url)
        .call()
        .unwrap()
        .into_reader()
        .read_to_string(&mut buf)
        .unwrap();
    buf
}
