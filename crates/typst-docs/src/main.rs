mod templates;

use std::{
    fs::{create_dir_all, write},
    path::{Path, PathBuf},
};

use clap::Parser;
use include_dir::{include_dir, Dir};
use pulldown_cmark::escape::escape_html;
use typst::{model::Document, visualize::Color};
use typst_docs::{provide, Html, PageModel, Resolver};
use typst_render::render;

use self::templates::render_page;

static PUBLIC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/public");

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
                r#"<div class="previewed-code">
                    <pre>{code_safe}</pre>
                    <div class="preview">
                        <img src="{src}" alt="Preview" width="480" height="190" />
                    </div>
                </div>"#
            ))
        } else {
            Html::new(format!(
                r#"<div class="preview">
                <img src="{src}" alt="Preview" width="480" height="190" />
            </div>"#
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

/// Generates the documentation website for the Typst project. Expects the
/// site to be hosted so that the main docs website is at '/docs'. You are
/// encouraged to post-process the resulting file tree output to customize the
/// base URL or assets location to work better with your hosting setup.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "_site")]
    out_dir: PathBuf,
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let own_root_pages = provide(&MyResolver {
        out_dir: args.out_dir.as_path(),
        verbose: args.verbose.clone(),
    });
    let root_pages: Vec<_> = own_root_pages.iter().collect();
    eprintln!("Generated data for {} root pages", root_pages.len());

    fn pages_flat_helper<'a>(
        page: &'a PageModel,
        mut all_pages: &mut Vec<&'a PageModel>,
    ) {
        all_pages.push(page);
        for page in &page.children {
            pages_flat_helper(page, &mut all_pages);
        }
    }
    let mut all_pages: Vec<&PageModel> = Vec::new();
    for root_page in &root_pages {
        pages_flat_helper(root_page, &mut all_pages);
    }
    eprintln!("Crawled root pages and found data for {} total pages", all_pages.len());

    for page in &all_pages {
        let mut path = args.out_dir.clone();
        let mut route_path = page.route.to_string();
        if route_path.ends_with("/") {
            route_path.push_str("index.html");
        }
        if route_path.starts_with("/") {
            route_path.remove(0);
        }
        path.push(route_path);

        let html = render_page(page, &all_pages, &root_pages)?;
        if args.verbose {
            eprintln!("Generated {} chars of HTML for {:?}", html.len(), &page.route);
        }

        create_dir_all(path.parent().ok_or("no parent")?)?;
        write(&path, html.as_str())?;
        eprintln!("Created {:?}", &path);
    }

    create_dir_all(&args.out_dir)?;
    PUBLIC_DIR.extract(&args.out_dir)?;
    eprintln!("Extracted other assets to {:?}", &args.out_dir);

    eprintln!("All done!");

    Ok(())
}
