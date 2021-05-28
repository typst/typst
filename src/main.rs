use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context};

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        println!("Usage: typst src.typ [out.pdf]");
        return Ok(());
    }

    let src_path = Path::new(&args[1]);
    let dest_path = if args.len() <= 2 {
        let name = src_path
            .file_name()
            .ok_or_else(|| anyhow!("Source path is not a file."))?;
        Path::new(name).with_extension("pdf")
    } else {
        PathBuf::from(&args[2])
    };

    if src_path == dest_path {
        bail!("Source and destination path are the same.");
    }

    let src = fs::read_to_string(src_path).context("Failed to read from source file.")?;

    let mut loader = typst::loading::FsLoader::new();
    loader.search_path("fonts");
    loader.search_system();

    let mut cache = typst::cache::Cache::new(&loader);
    let scope = typst::library::new();
    let state = typst::exec::State::default();
    let pass = typst::typeset(&mut loader, &mut cache, &src, &scope, state);
    let map = typst::parse::LineMap::new(&src);
    for diag in pass.diags {
        let start = map.location(diag.span.start).unwrap();
        let end = map.location(diag.span.end).unwrap();
        println!(
            "{}: {}:{}-{}: {}",
            diag.level,
            src_path.display(),
            start,
            end,
            diag.message,
        );
    }

    let buffer = typst::export::pdf(&cache, &pass.output);
    fs::write(&dest_path, buffer).context("Failed to write PDF file.")?;

    Ok(())
}
