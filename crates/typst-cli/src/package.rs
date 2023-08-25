use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codespan_reporting::term::{self, termcolor};
use once_cell::sync::Lazy;
use termcolor::WriteColor;
use typst::diag::{PackageError, PackageResult};
use typst::syntax::PackageSpec;

use super::color_stream;

/// HTTP request agent.
static AGENT: Lazy<ureq::Agent> = Lazy::new(|| {
    // Get the network proxy config from the environment.
    let proxy = env_proxy::for_url_str("https://typst.org")
        .to_url()
        .and_then(|url| ureq::Proxy::new(url).ok());

    // Check for a custom CA certificate
    let custom_tls_config = crate::ARGS
        .cert
        .as_ref()
        .map(|path| {
            let file = std::fs::OpenOptions::new().read(true).open(path)?;
            let mut buffer = std::io::BufReader::new(file);
            let certs = rustls_pemfile::certs(&mut buffer)?;
            let mut cert_store = rustls::RootCertStore::empty();
            cert_store.add_parsable_certificates(&certs);
            let config = rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(cert_store)
                .with_no_client_auth();

            Ok::<_, std::io::Error>(Arc::new(config))
        })
        // If there is an error loading certificate, just use the default configuration.
        .and_then(|x| x.ok());

    // Build the agent
    let mut agent = ureq::AgentBuilder::new()
        .user_agent(concat!("typst/{}", env!("CARGO_PKG_VERSION")));
    if let Some(proxy) = proxy {
        agent = agent.proxy(proxy);
    }
    if let Some(tls_config) = custom_tls_config {
        agent = agent.tls_config(tls_config);
    }
    agent.build()
});

/// Make a package available in the on-disk cache.
pub fn prepare_package(spec: &PackageSpec) -> PackageResult<PathBuf> {
    let subdir =
        format!("typst/packages/{}/{}/{}", spec.namespace, spec.name, spec.version);

    if let Some(data_dir) = dirs::data_dir() {
        let dir = data_dir.join(&subdir);
        if dir.exists() {
            return Ok(dir);
        }
    }

    if let Some(cache_dir) = dirs::cache_dir() {
        let dir = cache_dir.join(&subdir);

        // Download from network if it doesn't exist yet.
        if spec.namespace == "preview" && !dir.exists() {
            download_package(spec, &dir)?;
        }

        if dir.exists() {
            return Ok(dir);
        }
    }

    Err(PackageError::NotFound(spec.clone()))
}

/// Download a package over the network.
fn download_package(spec: &PackageSpec, package_dir: &Path) -> PackageResult<()> {
    // The `@preview` namespace is the only namespace that supports on-demand
    // fetching.
    assert_eq!(spec.namespace, "preview");

    let url = format!(
        "https://packages.typst.org/preview/{}-{}.tar.gz",
        spec.name, spec.version
    );

    print_downloading(spec).unwrap();
    let reader = match AGENT.get(&url).call() {
        Ok(response) => response.into_reader(),
        Err(ureq::Error::Status(404, _)) => {
            return Err(PackageError::NotFound(spec.clone()))
        }
        Err(_) => return Err(PackageError::NetworkFailed),
    };

    let decompressed = flate2::read::GzDecoder::new(reader);
    tar::Archive::new(decompressed).unpack(package_dir).map_err(|_| {
        fs::remove_dir_all(package_dir).ok();
        PackageError::MalformedArchive
    })
}

/// Print that a package downloading is happening.
fn print_downloading(spec: &PackageSpec) -> io::Result<()> {
    let mut w = color_stream();
    let styles = term::Styles::default();

    w.set_color(&styles.header_help)?;
    write!(w, "downloading")?;

    w.reset()?;
    writeln!(w, " {spec}")
}
