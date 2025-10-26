use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use typst::layout::PagedDocument;
use typst_docs::{Html, Resolver, provide};
use typst_render::{RenderOptions, render};

#[derive(Debug)]
struct CliResolver<'a> {
    assets_dir: &'a Path,
    verbose: bool,
    base: &'a str,
}

impl Resolver for CliResolver<'_> {
    fn commits(&self, from: &str, to: &str) -> Vec<typst_docs::Commit> {
        if self.verbose {
            eprintln!("commits({from}, {to})");
        }
        vec![]
    }

    fn example(
        &self,
        hash: u128,
        source: Option<Html>,
        document: &PagedDocument,
    ) -> typst_docs::Html {
        if self.verbose {
            eprintln!(
                "example(0x{hash:x}, {:?} chars, Document)",
                source.as_ref().map(|s| s.as_str().len())
            );
        }

        fs::create_dir_all(self.assets_dir).expect("create dir");

        let pages = match &document.pages[..] {
            [page] => vec![(page, format!("{hash:x}.png"), "Preview".to_string())],
            pages => pages
                .iter()
                .enumerate()
                .map(|(i, page)| {
                    (page, format!("{hash:x}-{i}.png"), format!("Preview page {}", i + 1))
                })
                .collect(),
        }
        .iter()
        .map(|(page, filename, alt)| {
            let pixmap =
                render(page, RenderOptions { pixel_per_pt: 2.0, render_bleed: false });
            let path = self.assets_dir.join(filename);
            pixmap.save_png(path.as_path()).expect("save png");
            eprintln!("Generated example image {path:?}");

            let src = format!("{}assets/{filename}", self.base);
            format!(r#"<img src="{src}" alt="{alt}">"#)
        })
        .collect::<String>();

        if let Some(code) = source {
            let code_safe = code.as_str();
            Html::new(format!(
                r#"<div class="previewed-code"><pre>{code_safe}</pre><div class="preview">{pages}</div></div>"#
            ))
        } else {
            Html::new(format!(r#"<div class="preview">{pages}</div>"#))
        }
    }

    fn image(&self, filename: &str, data: &[u8]) -> String {
        if self.verbose {
            eprintln!("image({filename}, {} bytes)", data.len());
        }

        let path = self.assets_dir.join(filename);
        fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        fs::write(&path, data).expect("write image");
        eprintln!("Created {} byte image at {path:?}", data.len());

        format!("{}assets/{filename}", self.base)
    }

    fn link(&self, link: &str) -> Option<String> {
        if self.verbose {
            eprintln!("link({link})");
        }
        None
    }

    fn base(&self) -> &str {
        self.base
    }
}

/// Generates the JSON representation of the documentation. This can be used to
/// generate the HTML yourself. Be warned: the JSON structure is not stable and
/// may change at any time.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// The generation process can produce additional assets. Namely images.
    /// This option controls where to spit them out. The HTML generation will
    /// assume that this output directory is served at `${base_url}/assets/*`.
    /// The default is `assets`. For example, if the base URL is `/docs/` then
    /// the generated HTML might look like `<img src="/docs/assets/foo.png">`
    /// even though the `--assets-dir` was set to `/tmp/images` or something.
    #[arg(long, default_value = "assets")]
    assets_dir: PathBuf,

    /// Write the JSON output to this file. The default is `-` which is a
    /// special value that means "write to standard output". If you want to
    /// write to a file named `-` then use `./-`.
    #[arg(long, default_value = "-")]
    out_file: PathBuf,

    /// The base URL for the documentation. This can be an absolute URL like
    /// `https://example.com/docs/` or a relative URL like `/docs/`. This is
    /// used as the base URL for the generated page's `.route` properties as
    /// well as cross-page links. The default is `/`. If a `/` trailing slash is
    /// not present then it will be added. This option also affects the HTML
    /// asset references. For example: `--base /docs/` will generate
    /// `<img src="/docs/assets/foo.png">`.
    #[arg(long, default_value = "/")]
    base: String,

    /// Enable verbose logging. This will print out all the calls to the
    /// resolver and the paths of the generated assets.
    #[arg(long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut base = args.base.clone();
    if !base.ends_with('/') {
        base.push('/');
    }

    let resolver = CliResolver {
        assets_dir: &args.assets_dir,
        verbose: args.verbose,
        base: &base,
    };
    if args.verbose {
        eprintln!("resolver: {resolver:?}");
    }
    let pages = provide(&resolver);

    eprintln!("Be warned: the JSON structure is not stable and may change at any time.");
    let json = serde_json::to_string_pretty(&pages)?;

    if args.out_file.to_string_lossy() == "-" {
        println!("{json}");
    } else {
        fs::write(&args.out_file, &*json)?;
    }

    Ok(())
}
