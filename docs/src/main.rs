use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use typst::model::Document;
use typst_docs::{provide, Html, Resolver};
use typst_render::render;

#[derive(Debug)]
struct CliResolver<'a> {
    assets_dir: &'a Path,
    verbose: bool,
    base: &'a str,
}

impl<'a> Resolver for CliResolver<'a> {
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
        document: &Document,
    ) -> typst_docs::Html {
        if self.verbose {
            eprintln!(
                "example(0x{hash:x}, {:?} chars, Document)",
                source.as_ref().map(|s| s.as_str().len())
            );
        }

        let page = document.pages.first().expect("page 0");
        let pixmap = render(page, 2.0);
        let filename = format!("{hash:x}.png");
        let path = self.assets_dir.join(&filename);
        fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        pixmap.save_png(path.as_path()).expect("save png");
        let src = format!("{}assets/{filename}", self.base);
        eprintln!("Generated example image {path:?}");

        if let Some(code) = source {
            let code_safe = code.as_str();
            Html::new(format!(
                r#"<div class="previewed-code"><pre>{code_safe}</pre><div class="preview"><img src="{src}" alt="Preview" /></div></div>"#
            ))
        } else {
            Html::new(format!(
                r#"<div class="preview"><img src="{src}" alt="Preview" /></div>"#
            ))
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
#[derive(Parser, Debug)]
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
