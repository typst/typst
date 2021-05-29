use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context};

use typst::loading::Loader;

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        println!("usage: typst src.typ [out.pdf]");
        return Ok(());
    }

    // Create a loader for fonts and files.
    let mut loader = typst::loading::FsLoader::new();
    loader.search_path("fonts");
    loader.search_system();

    // Resolve the canonical path because the compiler needs it for module
    // loading.
    let src_path = Path::new(&args[1]);

    // Find out the file name to create the output file.
    let name = src_path
        .file_name()
        .ok_or_else(|| anyhow!("source path is not a file"))?;

    let dest_path = if args.len() <= 2 {
        Path::new(name).with_extension("pdf")
    } else {
        PathBuf::from(&args[2])
    };

    // Ensure that the source file is not overwritten.
    let src_hash = loader.resolve(&src_path);
    let dest_hash = loader.resolve(&dest_path);
    if src_hash.is_some() && src_hash == dest_hash {
        bail!("source and destination files are the same");
    }

    // Read the source.
    let src = fs::read_to_string(&src_path)
        .map_err(|_| anyhow!("failed to read source file"))?;

    // Compile.
    let mut cache = typst::cache::Cache::new(&loader);
    let scope = typst::library::new();
    let state = typst::exec::State::default();
    let pass = typst::typeset(&mut loader, &mut cache, &src_path, &src, &scope, state);

    // Print diagnostics.
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

    // Export the PDF.
    let buffer = typst::export::pdf(&cache, &pass.output);
    fs::write(&dest_path, buffer).context("failed to write PDF file")?;

    Ok(())
}
