use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{anyhow, bail, Context};
use same_file::is_same_file;

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        println!("usage: typst src.typ [out.pdf]");
        return Ok(());
    }

    // Determine source and destination path.
    let src_path = Path::new(&args[1]);
    let dest_path = if let Some(arg) = args.get(2) {
        PathBuf::from(arg)
    } else {
        let name = src_path
            .file_name()
            .ok_or_else(|| anyhow!("source path is not a file"))?;

        Path::new(name).with_extension("pdf")
    };

    // Ensure that the source file is not overwritten.
    if is_same_file(src_path, &dest_path).unwrap_or(false) {
        bail!("source and destination files are the same");
    }

    // Create a loader for fonts and files.
    let mut loader = typst::loading::FsLoader::new();
    loader.search_path("fonts");
    loader.search_system();

    // Resolve the file id of the source file and read the file.
    let src_id = loader.resolve_path(src_path).context("source file not found")?;
    let src = fs::read_to_string(&src_path)
        .map_err(|_| anyhow!("failed to read source file"))?;

    // Typeset.
    let mut ctx = typst::Context::new(Rc::new(loader));
    let pass = ctx.typeset(src_id, &src);

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
    let buffer = typst::export::pdf(&ctx, &pass.output);
    fs::write(&dest_path, buffer).context("failed to write PDF file")?;

    Ok(())
}
