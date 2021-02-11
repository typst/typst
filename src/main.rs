use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context};
use fontdock::fs::FsIndex;

use typst::diag::Pass;
use typst::env::{Env, ResourceLoader};
use typst::exec::State;
use typst::export::pdf;
use typst::font::FsIndexExt;
use typst::library;
use typst::parse::LineMap;
use typst::typeset;

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

    let mut index = FsIndex::new();
    index.search_dir("fonts");
    index.search_system();

    let mut env = Env {
        fonts: index.into_dynamic_loader(),
        resources: ResourceLoader::new(),
    };

    let scope = library::new();
    let state = State::default();

    let Pass { output: frames, diags } = typeset(&mut env, &src, &scope, state);
    if !diags.is_empty() {
        let map = LineMap::new(&src);
        for diag in diags {
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
    }

    let pdf_data = pdf::export(&frames, &env);
    fs::write(&dest_path, pdf_data).context("Failed to write PDF file.")?;

    Ok(())
}
