use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context};

use typst::diag::Pass;
use typst::env::{Env, FsLoader};
use typst::exec::State;
use typst::library;
use typst::parse::LineMap;
use typst::pdf;
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

    let mut loader = FsLoader::new();
    loader.search_dir("fonts");
    loader.search_system();

    let mut env = Env::new(loader);

    let scope = library::_new();
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

    let pdf_data = pdf::export(&env, &frames);
    fs::write(&dest_path, pdf_data).context("Failed to write PDF file.")?;

    Ok(())
}
