use std::{
    fs::{create_dir_all, write},
    path::{Path, PathBuf},
};

use clap::Parser;
use typst::{model::Document, visualize::Color};
use typst_docs::{provide, Html, Resolver};
use typst_render::render;

struct MyResolver<'a> {
    out_dir: &'a Path,
    verbose: bool,
}
impl<'a> Resolver for MyResolver<'a> {
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

        let frame = &document.pages.first().expect("page 0").frame;
        let pixmap = render(frame, 2.0, Color::WHITE);
        let filename = format!("{hash:x}.png");
        let path = self.out_dir.join("assets").join("docs").join(&filename);
        create_dir_all(path.parent().expect("parent")).expect("create dir");
        pixmap.save_png(path.as_path()).expect("save png");
        let src = format!("/assets/docs/{filename}");
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

        let path = self.out_dir.join("assets").join("docs").join(filename);
        create_dir_all(path.parent().expect("parent")).expect("create dir");
        write(&path, data).expect("write image");
        eprintln!("Created {} byte image at {path:?}", data.len());

        format!("/assets/docs/{filename}")
    }
    fn link(&self, link: &str) -> Option<String> {
        if self.verbose {
            eprintln!("link({link})");
        }
        None
    }
}

/// Generates the JSON representation of the documentation. This can be used
/// to generate the HTML yourself. You are encouraged to post-process the generated
/// JSON to rewrite links and other things to match your site's structure.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The generation process can produce additional assets. Namely images. This
    /// option controls where to spit them out. It assumes that this is the base
    /// folder of the site ("/" in the URL). Images & example renderings will be
    /// placed in a folder called "assets/docs" under this directory and expected
    /// to be made available at "/assets/docs/*".
    #[arg(short, long, default_value = "_site")]
    out_dir: PathBuf,

    /// Enable verbose logging. This will print out all the calls to the resolver
    /// and the paths of the generated assets.
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let root_pages = provide(&MyResolver {
        out_dir: args.out_dir.as_path(),
        verbose: args.verbose,
    });
    let json = serde_json::to_string_pretty(&root_pages)?;
    println!("{json}");

    eprintln!("All done!");
    Ok(())
}
