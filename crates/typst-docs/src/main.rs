mod templates;

use std::{
    fs::{create_dir_all, write},
    path::{Path, PathBuf},
};

use clap::Parser;
use include_dir::{include_dir, Dir};
use typst::model::Document;
use typst_docs::{provide, Html, PageModel, Resolver};

use self::templates::render_page;

static PUBLIC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/public");

struct MyResolver<'a> {
    out_dir: &'a Path,
}
impl<'a> Resolver for MyResolver<'a> {
    fn commits(&self, from: &str, to: &str) -> Vec<typst_docs::Commit> {
        eprintln!("commits({from}, {to})");
        vec![]
    }
    fn example(
        &self,
        hash: u128,
        source: Option<typst_docs::Html>,
        _document: &Document,
    ) -> typst_docs::Html {
        eprintln!(
            "example(0x{hash:x}, {:?} chars, Document)",
            source.as_ref().map(|s| s.as_str().len())
        );

        Html::new("".to_string())
    }
    fn image(&self, filename: &str, data: &[u8]) -> String {
        eprintln!("image({filename}, {} bytes)", data.len());

        let path = self.out_dir.join("docs").join(filename);
        create_dir_all(path.parent().expect("parent")).expect("create dir");
        write(path, data).expect("write image");

        format!("/assets/docs/{filename}")
    }
    fn link(&self, link: &str) -> Option<String> {
        eprintln!("link({link})");
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let own_pages = provide(&MyResolver { out_dir: args.out_dir.as_path() });
    let pages: Vec<_> = own_pages.iter().collect();

    fn pages_flat_helper<'a>(page: &'a PageModel, pages: &mut Vec<&'a PageModel>) {
        pages.push(page);
        pages.extend(page.children.iter());
    }
    let mut all_pages: Vec<&PageModel> = Vec::new();
    for page in &pages {
        pages_flat_helper(page, &mut all_pages);
    }

    for page in &pages {
        let mut path = args.out_dir.clone();
        let mut route = page.route.to_string();
        if route.ends_with("/") {
            route.push_str("index.html");
        }
        let mut route = route.strip_prefix("/docs/").unwrap_or(&route).to_string();
        if route.starts_with("/") {
            route.remove(0);
        }
        path.push(route);

        let html = render_page(page, &all_pages)?;

        create_dir_all(path.parent().expect("parent")).expect("create dir");
        write(path, html.as_str()).expect("write page");
    }

    PUBLIC_DIR.extract(args.out_dir)?;

    Ok(())
}
