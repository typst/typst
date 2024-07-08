// build.rs

use std::env;
use std::fs;
use std::path::Path;

/// This function removes comments, line spaces and carriage returns from a
/// PostScript program. This is necessary to optimize the size of the PDF file.
fn minify(source: &str) -> String {
    let mut buf = String::with_capacity(source.len());
    let mut s = unscanny::Scanner::new(source);
    while let Some(c) = s.eat() {
        match c {
            '%' => {
                s.eat_until('\n');
            }
            c if c.is_whitespace() => {
                s.eat_whitespace();
                if buf.ends_with(|c: char| !c.is_whitespace()) {
                    buf.push(' ');
                }
            }
            _ => buf.push(c),
        }
    }
    buf
}

/// Compress data with the DEFLATE algorithm.
fn deflate(data: &[u8]) -> Vec<u8> {
    const COMPRESSION_LEVEL: u8 = 6;
    miniz_oxide::deflate::compress_to_vec_zlib(data, COMPRESSION_LEVEL)
}

fn write_deflated(out_dir: &Path, dest: &str, inflated: &[u8]) {
    let dest_path = out_dir.join(dest);
    fs::write(&dest_path, deflate(inflated)).expect("write failed");
}

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);
    write_deflated(out_dir, "srgb_icc_deflated", typst_assets::icc::S_RGB_V4);
    write_deflated(out_dir, "gray_icc_deflated", typst_assets::icc::S_GREY_V4);
    write_deflated(
        out_dir,
        "oklab_deflated",
        minify(include_str!("src/oklab.ps")).as_bytes(),
    );
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/oklab.ps");
}
